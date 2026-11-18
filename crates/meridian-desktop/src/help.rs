//! Host-only modal help. Opening a panel never submits a gameplay command.
use crate::docked_screens::Screen;

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
    fn new(title: impl Into<String>, controls: &[&str]) -> Self {
        Self {
            title: title.into(),
            lines: controls.iter().map(|line| (*line).into()).collect(),
            footer: "Escape / H / ? Close help",
        }
    }

    pub fn title() -> Self {
        Self::new(
            "Commander",
            &[
                "C Continue current save | N New commander",
                "N again confirms a replacement | Escape Quit",
                "F11 Fullscreen",
            ],
        )
    }

    pub fn commander(has_berth: bool) -> Self {
        let mut panel = Self::new(
            "Commander details",
            &["F5 Save | F9 Reload | F11 Fullscreen | Escape Save/Quit"],
        );
        if has_berth {
            panel
                .lines
                .insert(0, "L Launch | 0 Commander details".into());
            panel.lines.insert(
                1,
                "1 Galaxy | 2 System | 3 Showroom | 4 Market | 5 Equipment".into(),
            );
        } else {
            panel.lines.insert(0, "Enter Resume flight".into());
        }
        panel
    }

    pub fn flight(weapon: Option<&str>, review: Option<&str>) -> Self {
        let mut panel = Self::new(
            "Flight controls",
            &[
                "W / S Accelerate / slow | Space Brake to zero",
                "Arrows Pitch | A / D Yaw | Q / E Roll",
                "0 Front | 1 Rear | 2 Left | 3 Right view",
                "F1 Next missile target | F2 Arm / safe",
                "F3 Launch one missile | F4 Safe and clear target",
                "J Jump to selected route | P Pause / resume",
                "F6 Flight lesson | F12 Cancel | C Lesson details",
                "I Mission details and missile status",
                "Hold U 10 seconds: EMP | Release U cancels unfinished charge",
                "F7 Close attempt | F8 Reception | F10 Practice review at Avenor berth",
                "4 Trade review | 5 Plan | 6 Cancel | 7 Details",
                "F5 Save | F9 Reload | F11 Fullscreen | Escape Save/Quit",
            ],
        );
        panel.lines.insert(
            2,
            weapon.map_or_else(
                || "Z unavailable: no fitted front laser".into(),
                |weapon| {
                    if weapon == "Mining Laser" {
                        "Z Mine eligible rock with fitted Mining Laser; no automatic pickup".into()
                    } else {
                        format!("Z Fire fitted {weapon}")
                    }
                },
            ),
        );
        panel.review(review, "After help: B cancels; Escape saves and quits");
        panel
    }

    pub fn docked(review: Option<&str>) -> Self {
        let mut panel = Self::new(
            "Station controls",
            &[
                "L Launch | F Buy fuel",
                "T Select crew | R Review interaction | I Mission details",
                "V Review deliveries | X Review abandonment",
                "Enter Confirm review | B Cancel review",
                "F6 Flight lesson | F12 Cancel | C Lesson details",
                "F7 Close practice | F8 Reception | F10 Practice review",
                "6 Cancel trade lesson | 7 Trade lesson details",
                "F5 Save | F9 Reload | P Pause | F11 Fullscreen | Escape Save/Quit",
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
                    "R Review route | L Local chart | Home Current location",
                    "[ / ] Previous / next galaxy | Drag Pan | Wheel Zoom",
                ],
            ),
            Screen::System => (
                "System",
                &[
                    "Up / Down Select route | Enter / R Review route",
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
                    "Trade confirmation uses the current price and native unit",
                ],
            ),
            Screen::Equipment => (
                "Equipment",
                &[
                    "] Next slot | Up / Down Select owned item or offer",
                    "G / Enter Review purchase | Y Review fit | U Review store",
                    "D Details | O Options",
                    "EMP slot: G reviews base/grade; Y fits; U stores; no free charge",
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
            "Ctrl+Shift+X XYZZY testing menu (docked commander only)".into(),
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
                .any(|line| line == "Z Fire fitted MINING")
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
}
