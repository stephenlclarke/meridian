//! Host-only panels. Opening a panel never submits a gameplay command.
use crate::docked_screens::Screen;
use meridian_core::unwritten_route::ClueProgress;

pub(crate) const FLIGHT_STATUS: &str = "Flight status";
pub(crate) const FLIGHT_CONTROLS: &str = "Flight controls";

pub const MAX_LINES: usize = 18;
pub const MAX_COLUMNS: usize = 78;
pub const MAX_VERTICES: usize = 6 + MAX_LINES * MAX_COLUMNS * 35 * 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Panel {
    pub title: String,
    pub lines: Vec<String>,
    pub footer: &'static str,
}

impl Panel {
    pub fn occupied_cut(review: &meridian_core::OccupiedCutReview) -> Self {
        let mut panel = Self::new(
            "Occupied salvage - confirm contact",
            &[
                "OCCUPIED / CLAIMED. The ship is not abandoned salvage.",
                "Crew and paid cargo remain protected.",
                "Deliberate cutter contact is attributed to your commander.",
                "No verified detachable material is available on this hull.",
                "Enter confirms one contact. Escape cancels without any change.",
            ],
        );
        if review.is_unlawful() {
            panel.lines.insert(
                2,
                "Warning: contact is an unlawful attack in this game.".into(),
            );
        }
        panel.footer = "Enter Confirm | Escape Cancel";
        panel
    }
    fn new(title: impl Into<String>, controls: &[&str]) -> Self {
        Self {
            title: title.into(),
            lines: controls.iter().map(|line| (*line).into()).collect(),
            footer: "Escape / H / ? Close help",
        }
    }

    pub fn clue_journal(progress: &ClueProgress) -> Self {
        let mut lines = vec![format!(
            "{} of {} distinct source clues collected",
            progress.earned.len(),
            ClueProgress::REQUIRED
        )];
        if progress.complete() {
            lines.push("Ask the AI at PLUGH to decode the collected traces.".into());
        } else {
            lines.push("Collect the remaining source clues to unlock the next instruction.".into());
        }

        if progress.earned.is_empty() {
            lines.push("No clues earned yet.".into());
        } else {
            // Keep the text within the standard modal's bounds, including its footer.
            let visible = MAX_LINES - lines.len() - 3;
            lines.extend(
                progress
                    .earned
                    .iter()
                    .take(visible)
                    .map(|clue| clue.clue_id.replace('-', " ")),
            );
            if progress.earned.len() > visible {
                lines.push(format!(
                    "{} more clues earned",
                    progress.earned.len() - visible
                ));
            }
        }
        Self {
            title: "The Unwritten Route - Clue Journal".into(),
            lines,
            footer: "Escape / Shift+J Close journal",
        }
    }

    pub fn title() -> Self {
        Self::new(
            "Commander",
            &[
                "C Continue current save | N New commander",
                "N again confirms a replacement | Escape Quit",
                "Ctrl+Enter Fullscreen",
            ],
        )
    }

    pub fn commander(has_berth: bool) -> Self {
        let mut panel = Self::new(
            "Commander details",
            &[
                "Shift+J Clue Journal",
                "Ctrl+S Save | Ctrl+R Reload | Ctrl+Enter Fullscreen | Escape Save/Quit",
            ],
        );
        if has_berth {
            panel
                .lines
                .insert(0, "L Launch | 0 Commander details".into());
            panel.lines.push(
                "Shift+1/2 Lesson review/cancel | Shift+3/4/5 Gunnery review/receive/close".into(),
            );
            panel.lines.insert(
                1,
                "1 Galaxy | 2 System | 3 Showroom | 4 Market | 5 Equipment".into(),
            );
        } else {
            panel.lines.insert(0, "Enter Resume flight".into());
        }
        panel
    }

    pub fn flight_facing(
        weapon: Option<&str>,
        review: Option<&str>,
        facing: meridian_core::FlightView,
    ) -> Self {
        let mut panel = Self::flight(weapon, review);
        if facing != meridian_core::FlightView::Front {
            let mount = crate::flight_lesson::view_name(facing);
            panel.lines[1] = weapon.map_or_else(
                || format!("Z: no laser fitted to {mount}"),
                |name| format!("Z Fire {mount} {name}"),
            );
        }
        panel
    }
    pub(crate) fn flight_status(
        world: &meridian_core::World,
        last_notice: Option<&str>,
        placement: crate::hud::cockpit::notice::Placement,
    ) -> Self {
        let mut panel = Self::new(FLIGHT_STATUS, &[]);
        panel.footer = "N Change notice position | Escape / H Close";
        panel
            .lines
            .push(format!("Notice position: {}", placement.name()));
        let last = last_notice.map(|message| format!("Last notice: {message}"));
        for line in last
            .into_iter()
            .chain(crate::session::Session::flight_status(world))
        {
            let mut row = String::new();
            for word in line.split_whitespace() {
                if row.len() + 1 + word.len() > MAX_COLUMNS {
                    panel.lines.push(std::mem::take(&mut row));
                }
                if !row.is_empty() {
                    row.push(' ');
                }
                row.push_str(word);
            }
            panel.lines.push(row);
        }
        panel
    }
    pub fn flight(weapon: Option<&str>, review: Option<&str>) -> Self {
        let mut panel = Self::new(
            FLIGHT_CONTROLS,
            &[
                "W/S Accelerate/slow | Space Brake to zero | Arrows Pitch | A/D Yaw | Q/E Roll",
                "N Start or cancel auto-docking | 0 Front | 1 Rear | 2 Left | 3 Right view",
                "T Lock selected ship for missiles | 8 Next missile target | 9 Arm / safe",
                "= Launch missile | - Safe/clear target | K E.C.M. (22 energy)",
                "J Jump | Shift+J Clue Journal | P Pause | Shift+4 Reception | Shift+5 Close",
                "Shift+1 Flight lesson | Shift+2 Cancel | C Lesson details",
                "Tab Next scanner target (no core: mark) | F Manual/assist | Delete Release",
                "Basic aligns | Advanced approaches | Docking+nav docks | Specialists auto-work",
                "I Mission details | Shift+I Target details | Occupied: Enter/Escape",
                "Hold U 10 seconds: EMP | Release U cancels unfinished charge",
                "Shift+3 Gunnery review | 7 Trade goal | At berth: R from goal to review",
                "Ctrl+S Save | Ctrl+R Reload | Ctrl+Enter Fullscreen | Escape Save/Quit",
            ],
        );
        panel.lines.insert(
            1,
            weapon.map_or_else(
                || "Z: no fitted front laser | scoop ore | cut wreck | occupied review".into(),
                |weapon| {
                    if weapon == "Mining Laser" {
                        "Z Mine with Mining Laser | scoop ore | cut wreck | occupied review".into()
                    } else {
                        format!("Z Fire fitted {weapon} | scoop ore | cut wreck | occupied review")
                    }
                },
            ),
        );
        panel.footer = "I Flight status | Escape / H / ? Close help";
        panel.review(review, "After help: B cancels; Escape saves and quits");
        // The host app appends the audio control; reserve that row as well as title/footer.
        if panel.lines.len() < MAX_LINES - 3 {
            panel
                .lines
                .push("Solar fuel: fitted scoops, face sun and cruise; watch cabin temp".into());
        }
        panel
    }

    pub fn docked(review: Option<&str>) -> Self {
        let mut panel = Self::new(
            "Station controls",
            &[
                "L Launch | F Buy fuel | P Pause",
                "T Select crew | R Review interaction | I Mission details",
                "V Review deliveries | X Review abandonment",
                "Enter Confirm review | B Cancel review",
                "Shift+1 Flight lesson | Shift+2 Cancel | C Lesson details",
                "Shift+5 Close practice | Shift+4 Reception | Shift+3 Practice review",
                "7 Trade goal | R from goal to review | 6 Review cancellation",
                "Ctrl+S Save | Ctrl+R Reload | Ctrl+Enter Fullscreen | Escape Save/Quit",
            ],
        );
        panel.navigation();
        panel.review(review, "After help: B cancels; Escape saves and quits");
        panel
    }

    pub fn station(screen: Screen, review: Option<&str>, search: bool, details: bool) -> Self {
        let (title, controls): (&str, &[&str]) = match screen {
            Screen::Galaxy => (
                "Galaxy",
                &[
                    "Up / Down Select world | Page Up / Down Scroll",
                    "/ Search | Type a name | Enter Select result",
                    "H types into an active search; ? opens help",
                    "R Review route | 7 Trade destination | L Local chart | Home Current location",
                    "P Toggle best-profit scan | Route D trade details",
                    "[ / ] Previous / next galaxy | Drag Pan | Wheel Zoom",
                ],
            ),
            Screen::System => (
                "System",
                &[
                    "Up / Down Select route | Enter / R Review route",
                    "Route D Trade details | Page Up / Down Scroll | B Back",
                    "Physical-body chart routing is unavailable",
                    "Home Current location | Wheel Scroll",
                ],
            ),
            Screen::Showroom => (
                "Ship showroom",
                &[
                    "Up / Down Select ship | C Compare | Shift+B Pin baseline",
                    "V Current ship | Enter / D Details | / Search ships",
                    "R / Space Pause rotation | A Change rotation axis",
                    "Left / Right Turn | Home Reset view",
                    "Drag Rotate | Shift+drag Roll",
                    "Ship exchange is unavailable for the current catalogue",
                ],
            ),
            Screen::Trading => (
                "Market",
                &[
                    "Up / Down Select goods | Page Up / Down Scroll",
                    "G Buy | Shift+G Sell | Enter Review current trade mode",
                    "Left / Right Quantity -/+1 | Shift+Left / Right -/+10",
                    "F Refuel actual tank at the displayed price",
                    "7 Trade lesson for selected goods | R from goal to review",
                    "Trade confirmation uses the current price and native unit",
                ],
            ),
            Screen::Equipment => (
                "Equipment",
                &[
                    "] Next slot or laser mount | Up / Down Select owned item or offer",
                    "G / Enter Review purchase | Y Review fit | U Review store",
                    "D Details | O Options",
                    "EMP slot: G reviews base/grade; Y fits; U stores; no free charge",
                    "E.C.M. utility: G buy; Y fit; U store; K activates in flight",
                    "An unavailable offer explains its current refusal",
                ],
            ),
        };
        let mut panel = Self::new(title, controls);
        panel.navigation();
        if search {
            panel
                .lines
                .push("Search query is retained while help is open".into());
        }
        if details {
            panel
                .lines
                .push("Details/options stay open; Up/Down navigate after help".into());
        }
        panel.review(review, "After help: Escape / B cancels the review");
        if review.is_some() {
            panel
                .lines
                .push("Finish or cancel a review before changing station screens".into());
        }
        panel
    }

    fn navigation(&mut self) {
        self.lines.extend([
            "0 Commander | 1 Galaxy | 2 System | 3 Showroom | 4 Market | 5 Equipment".into(),
            "Tab / Shift+Tab Next / previous station screen".into(),
            "Escape Back: leave search/details/options, then the HUD".into(),
            "Shift+J Clue Journal | Ctrl+Shift+X XYZZY (docked only)".into(),
        ]);
    }

    fn review(&mut self, review: Option<&str>, cancel_hint: &str) {
        if let Some(title) = review {
            self.title = format!("{} - {}", self.title, title);
            self.lines
                .push("Review retained: Enter confirms only after help closes".into());
            self.lines.push(cancel_hint.into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clue_journal_displays_earned_names_and_hides_the_later_instruction_until_complete() {
        use meridian_core::unwritten_route::BaseClue;
        let progress = ClueProgress {
            earned: vec![BaseClue {
                source_id: "l337.training/flight-reception",
                clue_id: "flight-reception-family-trace",
            }],
            missing: vec![BaseClue {
                source_id: "l337.training/safe-gunnery",
                clue_id: "safe-gunnery-family-trace",
            }],
        };
        let panel = Panel::clue_journal(&progress);
        assert_eq!(panel.lines[0], "1 of 64 distinct source clues collected");
        assert!(
            panel
                .lines
                .iter()
                .any(|line| line == "flight reception family trace")
        );
        assert!(
            panel
                .lines
                .iter()
                .all(|line| !line.contains("safe gunnery") && !line.contains("PLUGH"))
        );
        assert!(panel.lines.len() + 2 <= MAX_LINES);
        assert!(panel.lines.iter().all(|line| line.len() <= MAX_COLUMNS));
    }

    #[test]
    fn all_station_contexts_and_reviews_explain_cross_screen_navigation() {
        for screen in Screen::ALL {
            for review in [
                None,
                Some("Purchase"),
                Some("Sale"),
                Some("Exchange"),
                Some("Route"),
                Some("Confirmation"),
            ] {
                let panel = Panel::station(screen, review, true, true);
                let content = panel.lines.join("\n");
                for required in [
                    "1 Galaxy",
                    "2 System",
                    "3 Showroom",
                    "4 Market",
                    "5 Equipment",
                    "Tab / Shift+Tab",
                    "Escape Back",
                    "Search query is retained",
                    "Details/options stay open",
                ] {
                    assert!(
                        content.contains(required),
                        "{screen:?} {review:?} {required}"
                    );
                }
                if let Some(review) = review {
                    assert!(panel.title.contains(review));
                    assert!(content.contains("only after help closes"));
                }
                assert!(panel.lines.len() + 2 <= MAX_LINES);
                assert!(panel.lines.iter().all(|line| line.len() <= MAX_COLUMNS));
            }
        }
    }
    #[test]
    fn sell_help_matches_the_actual_shift_g_binding_and_preserves_x() {
        use crate::docked_screens::Key;
        use winit::keyboard::KeyCode;
        assert_eq!(
            crate::docked::key(KeyCode::KeyG, true, Screen::Trading),
            Some(Key::Sell)
        );
        assert_eq!(
            crate::docked::key(KeyCode::KeyG, false, Screen::Trading),
            Some(Key::Buy)
        );
        assert_eq!(
            crate::docked::key(KeyCode::KeyH, false, Screen::Trading),
            None
        );
        assert_eq!(
            crate::docked::key(KeyCode::KeyX, false, Screen::Showroom),
            Some(Key::Exchange)
        );
        let market = Panel::station(Screen::Trading, None, false, false);
        assert!(
            market
                .lines
                .iter()
                .any(|line| line.contains("Shift+G Sell"))
        );
    }
    #[test]
    fn flight_describes_the_current_fitted_weapon_and_station_help_avoids_flight_controls() {
        let flight = Panel::flight(Some("MINING"), None);
        assert!(
            flight
                .lines
                .iter()
                .any(|line| line.starts_with("Z Fire fitted MINING"))
        );
        assert!(
            Panel::flight(None, None)
                .lines
                .iter()
                .any(|line| line.contains("no fitted front laser"))
        );
        let station = Panel::station(Screen::Showroom, None, false, false);
        assert!(
            !station
                .lines
                .iter()
                .any(|line| line.contains("Fire fitted"))
        );
        assert!(
            station
                .lines
                .iter()
                .any(|line| line.contains("exchange is unavailable"))
        );
    }

    #[test]
    fn flight_help_names_the_laser_on_the_selected_mount() {
        use meridian_core::FlightView;
        for (weapon, expected) in [
            (Some("Beam Laser"), "Z Fire LEFT Beam Laser"),
            (Some("Naval Laser"), "Z Fire LEFT Naval Laser"),
            (None, "Z: no laser fitted to LEFT"),
        ] {
            let panel = Panel::flight_facing(weapon, None, FlightView::Left);
            assert_eq!(panel.lines[1], expected);
        }
    }

    #[test]
    fn flight_help_with_review_and_host_audio_fits_minimum_viewport() {
        use meridian_core::ModuleKind;
        for weapon in [
            None,
            Some(ModuleKind::PulseLaser.name()),
            Some(ModuleKind::MiningLaser.name()),
            Some(ModuleKind::BeamLaser.name()),
            Some(ModuleKind::MilitaryLaser.name()),
            Some(ModuleKind::NavalLaser.name()),
        ] {
            for review in [None, Some("Trade lesson review")] {
                let mut panel = Panel::flight(weapon, review);
                panel
                    .lines
                    .push("Ctrl+Shift+M Toggle sound | Audio: Unavailable".into());
                let text = panel.lines.join("\n");
                for required in [
                    "C Lesson details",
                    "I Mission details",
                    "Shift+I Target details",
                    "Tab Next scanner target",
                    "F Manual/assist",
                    "Ctrl+Shift+M Toggle sound",
                ] {
                    assert!(text.contains(required), "Missing {required}");
                }
                assert!(panel.lines.len() + 2 <= MAX_LINES);
                assert!(panel.lines.iter().all(|line| line.len() <= MAX_COLUMNS));
                assert!(panel.title.len() <= MAX_COLUMNS);
                assert!(panel.footer.len() <= MAX_COLUMNS);
                let mut vertices = Vec::new();
                crate::hud::append_help(&panel, 640, 360, &mut vertices);
                assert!(!vertices.is_empty());
                assert!(vertices.len() <= MAX_VERTICES);
            }
        }
    }
}
