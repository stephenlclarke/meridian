//! Desktop selections hold only opaque World-issued values; they grant no authority.
use meridian_core::freight::g02::presentation;
use meridian_core::freight::g02::{Choice, Command, ContactDefinition, Event, JobId, Review};
use meridian_core::navigation::SystemId;
use meridian_core::station_services::{self, CommodityId};
use meridian_core::{
    FlightMode, PRACTICE_QUIET_TICKS, PracticeClosureReview, PracticeOutcome,
    PracticeReceptionReview, PracticeReview, PracticeStatus, World,
};

#[derive(Clone, Copy)]
pub enum Action {
    CycleMissileLock,
    ToggleMissileArm,
    FireMissile,
    ClearMissile,
    Contact,
    Review,
    ReviewAll,
    ReviewAbandonment,
    MissionDetails,
    ReviewPractice,
    ReviewPracticeReception,
    ReviewPracticeClosure,
    Confirm,
    Back,
    Refuel,
    ToggleMarket,
    ToggleEquipment,
    PurchaseCargoBay,
    FitCargoBay,
    UnfitCargoBay,
    PreviousCommodity,
    NextCommodity,
    BuySelected,
    SellSelected,
    Jump,
    Launch,
}

#[derive(Clone, Copy)]
pub(crate) enum PracticeSelection {
    Consent(PracticeReview),
    Reception(PracticeReceptionReview),
    Closure(PracticeClosureReview),
}

pub struct Session {
    job: Option<JobId>,
    pending: Option<(Review, Choice)>,
    pub(crate) pending_practice: Option<PracticeSelection>,
    show_objective: bool,
    market_open: bool,
    equipment_open: bool,
    selected_commodity: CommodityId,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            job: None,
            pending: None,
            pending_practice: None,
            show_objective: false,
            market_open: false,
            equipment_open: false,
            selected_commodity: CommodityId::Food,
        }
    }
}

impl Session {
    pub fn from_world(world: &World) -> Self {
        Self {
            job: world.active_freight_job(),
            pending: None,
            ..Self::default()
        }
    }

    pub fn invalidate(&mut self) {
        self.pending = None;
        self.pending_practice = None;
        self.show_objective = false;
        self.market_open = false;
        self.equipment_open = false;
    }

    fn freight(&mut self, world: &mut World, command: Command) -> Result<String, String> {
        let event = world
            .request_freight(command, world.snapshot().tick)
            .map_err(|e| e.to_string())?;
        match event {
            Event::Reviewed(review) => {
                let choice = match command {
                    Command::ReviewOffer { .. } => Choice::Accept,
                    Command::ReviewAbandonment { .. } => Choice::Abandon,
                    Command::ReviewReceiving { .. } => Choice::ReceiveOne(
                        review.next_unit().ok_or("No identified unit to receive")?,
                    ),
                    _ => return Err("Unexpected freight review response".into()),
                };
                self.pending = Some((review, choice));
                Ok("Review ready. Enter confirms; B cancels.".into())
            }
            Event::Accepted { job, .. } => {
                self.job = Some(job);
                Ok("Eight identified tonnes allocated. Destination: Renas utility crew.".into())
            }
            Event::Received {
                remaining,
                completion,
                ..
            } => {
                if completion.is_some() {
                    self.job = None;
                    Ok("Freight complete. 730 CR settled. First family clue recorded; no repeat settlement.".into())
                } else {
                    Ok(format!(
                        "One tonne received. {remaining} remain; R reviews the next unit."
                    ))
                }
            }
            Event::Back => Ok("Review cancelled.".into()),
            Event::Abandoned { .. } => {
                self.job = None;
                Ok("Freight abandoned. Allocated units are not restocked.".into())
            }
        }
    }

    fn review_command(&mut self, world: &mut World, action: Action) -> Result<String, String> {
        self.invalidate();
        let npc = world
            .mission_snapshot()
            .and_then(|m| m.contact)
            .ok_or("T selects the current station contact first")?;
        let command = match (action, npc.definition()) {
            (Action::ReviewAbandonment, ContactDefinition::Arlen) => Command::ReviewAbandonment {
                actor: npc.actor(),
                job: self.job.ok_or("No active freight job to abandon")?,
            },
            (Action::Review, ContactDefinition::Arlen) => {
                if self.job.is_some() {
                    return Err("Complete the current freight job before accepting another".into());
                }
                Command::ReviewOffer { actor: npc.actor() }
            }
            (Action::Review | Action::ReviewAll, ContactDefinition::RenasUtilityCrew) => {
                Command::ReviewReceiving {
                    actor: npc.actor(),
                    job: self.job.ok_or("No accepted freight job in this session")?,
                }
            }
            _ => {
                return Err(
                    "V reviews receiving at Renas; X reviews abandonment with Arlen".into(),
                );
            }
        };
        let message = self.freight(world, command)?;
        if matches!(action, Action::ReviewAll)
            && let Some((_, choice)) = self.pending.as_mut()
        {
            *choice = Choice::ReceiveAll;
        }
        Ok(message)
    }

    pub fn command(&mut self, world: &mut World, action: Action) -> Result<String, String> {
        if self.pending_practice.take().is_some() && matches!(action, Action::Back) {
            return Ok("Practice review cancelled.".into());
        }
        match action {
            Action::CycleMissileLock
            | Action::ToggleMissileArm
            | Action::FireMissile
            | Action::ClearMissile => {
                self.invalidate();
                missile_command(world, action)
            }
            Action::ReviewPractice => {
                self.invalidate();
                let review = world
                    .review_practice(world.snapshot().tick)
                    .map_err(|error| error.to_string())?;
                self.pending_practice = Some(PracticeSelection::Consent(review));
                Ok("Avenor safe gunnery practice".into())
            }
            Action::ReviewPracticeReception => {
                self.invalidate();
                let review = world
                    .review_practice_reception(world.snapshot().tick)
                    .map_err(|error| error.to_string())?;
                self.pending_practice = Some(PracticeSelection::Reception(review));
                Ok("Avenor practice reception".into())
            }
            Action::ReviewPracticeClosure => {
                self.invalidate();
                let review = world
                    .review_practice_closure(world.snapshot().tick)
                    .map_err(|error| error.to_string())?;
                self.pending_practice = Some(PracticeSelection::Closure(review));
                Ok("Avenor practice closure review".into())
            }
            Action::MissionDetails => {
                self.invalidate();
                self.show_objective = true;
                Ok("Mission details. T contacts crew; R opens a new review.".into())
            }
            Action::Contact => {
                self.invalidate();
                let npc = world
                    .station_contacts()
                    .into_iter()
                    .next()
                    .ok_or("No contact at this berth")?;
                world
                    .request_contact(npc, world.snapshot().tick)
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "Speaking with {}. R reviews freight terms.",
                    npc.name()
                ))
            }
            Action::Review | Action::ReviewAll | Action::ReviewAbandonment => {
                self.review_command(world, action)
            }
            Action::Confirm => {
                let (review, choice) = self
                    .pending
                    .take()
                    .ok_or("R opens a fresh review before confirmation")?;
                self.freight(world, Command::Confirm { review, choice })
            }
            Action::Back => {
                self.invalidate();
                self.freight(world, Command::Back)
            }
            Action::ToggleEquipment => {
                let open = !self.equipment_open;
                self.invalidate();
                self.equipment_open = open;
                Ok(if open {
                    "Equipment: K purchases and fits; U stores; Y refits the owned cargo bay."
                } else {
                    "Equipment closed."
                }
                .into())
            }
            Action::PurchaseCargoBay | Action::FitCargoBay | Action::UnfitCargoBay => {
                if !self.equipment_open {
                    return Err("O opens equipment before a purchase or fitting action".into());
                }
                self.pending = None;
                crate::equipment::command(world, action)
            }
            Action::ToggleMarket => {
                let open = !self.market_open;
                self.invalidate();
                self.market_open = open;
                Ok(if open {
                    "Market: [ and ] select; G buys one native unit; H sells one native unit."
                } else {
                    "Market closed."
                }
                .into())
            }
            Action::PreviousCommodity | Action::NextCommodity => {
                self.invalidate();
                let order = CommodityId::display_order().map_err(crate::market::content_error)?;
                let index = order
                    .iter()
                    .position(|id| *id == self.selected_commodity)
                    .ok_or("Selected commodity is absent from the original market order")?;
                let offset = if matches!(action, Action::NextCommodity) {
                    1
                } else {
                    order.len() - 1
                };
                self.selected_commodity = order[(index + offset) % order.len()];
                self.market_open = true;
                Ok("Commodity selected; G buys one native unit, H sells one native unit.".into())
            }
            Action::BuySelected | Action::SellSelected => {
                self.invalidate();
                self.market_open = true;
                let command = match action {
                    Action::BuySelected => {
                        station_services::Command::BuyCommodity(self.selected_commodity)
                    }
                    _ => station_services::Command::SellCommodity(self.selected_commodity),
                };
                let event = world
                    .request_station_service(command, world.snapshot().tick)
                    .map_err(|e| e.to_string())?;
                crate::market::service_message(event)
            }
            Action::Refuel => {
                self.invalidate();
                world
                    .request_refuel(world.snapshot().tick)
                    .map(|cost| format!("Paid {}.{} CR for fuel.", cost / 10, cost % 10))
                    .map_err(|e| e.to_string())
            }
            Action::Jump => {
                self.invalidate();
                let navigation = world
                    .snapshot()
                    .navigation
                    .ok_or("No commander navigation")?;
                let target = if navigation.system == SystemId::RENAS {
                    SystemId::AVENOR
                } else {
                    SystemId::RENAS
                };
                world
                    .request_jump(target, world.snapshot().tick)
                    .map(|_| "Jump spooling. Fuel debits on arrival.".into())
                    .map_err(|e| e.to_string())
            }
            Action::Launch => {
                self.invalidate();
                world
                    .request_launch(world.snapshot().tick)
                    .map(|_| "Launch admitted. Follow the exit corridor.".into())
                    .map_err(|e| e.to_string())
            }
        }
    }

    fn append_mission_panel(&self, world: &World, lines: &mut Vec<String>) -> bool {
        if let Some((review, choice)) = self.pending
            && let Some(dialogue) = presentation::review(review, choice)
        {
            append_paragraph(lines, dialogue.body);
            append_paragraph(lines, dialogue.notice);
            return true;
        }
        if self.show_objective
            && let Some(objective) = presentation::objective(world)
        {
            append_paragraph(lines, objective.description);
            if matches!(
                objective.kind,
                presentation::ObjectiveKind::DeliverOriginals
            ) {
                append_paragraph(lines, presentation::ACCEPTED_JOURNAL);
            }
            append_paragraph(lines, presentation::IDENTITY_NOTICE);
            lines.push(format!(
                "ORIGINAL TONNES HELD {} | UNALLOCATED SOURCE {} | COMPLETED JOBS {}",
                objective.held_original_tonnes,
                objective.available_source_units,
                objective.completed_jobs
            ));
            if let Some(ledger) = world.practice_ledger_snapshot() {
                let count = |outcome| {
                    ledger
                        .attempts
                        .iter()
                        .filter(|a| a.outcome == Some(outcome))
                        .count()
                };
                lines.push(format!(
                    "PRACTICE HISTORY | COMPLETE {} | FAILED {} | ABANDONED {} | OFFERS LEFT {}",
                    count(PracticeOutcome::Complete),
                    count(PracticeOutcome::Failed),
                    count(PracticeOutcome::Abandoned),
                    ledger.offers_remaining
                ));
                lines.push(if ledger.trace.is_some() {
                    "SAFE GUNNERY FAMILY TRACE RECORDED ONCE".into()
                } else {
                    "SAFE GUNNERY FAMILY TRACE NOT YET RECORDED".into()
                });
            }
            return true;
        }
        false
    }

    pub fn lines(&self, world: &World, active: bool, notice: Option<&str>) -> Vec<String> {
        let snapshot = world.snapshot();
        let name = snapshot
            .navigation
            .map(|n| match n.system {
                SystemId::AVENOR => "AVENOR",
                SystemId::RENAS => "RENAS",
                _ => "UNKNOWN SYSTEM",
            })
            .unwrap_or("NO COMMANDER");
        let mode = match snapshot.mode {
            FlightMode::Docked => "DOCKED",
            FlightMode::Launching => "LAUNCHING",
            FlightMode::Flight => "FLIGHT",
        };
        let mut lines = vec![format!(
            "MERIDIAN | {name} | {mode} | {}",
            if active {
                "RUNNING"
            } else {
                "PAUSED / UNFOCUSED"
            }
        )];
        let review_line = if let Some(selection) = self.pending_practice {
            let title = match selection {
                PracticeSelection::Consent(_) => "PRACTICE REVIEW",
                PracticeSelection::Reception(_) => "PRACTICE RECEPTION",
                PracticeSelection::Closure(_) => "CLOSE PRACTICE ATTEMPT",
            };
            format!("{title} | ENTER CONFIRMS AND SAVES | B CANCELS")
        } else if let Some((review, choice)) = self.pending {
            match choice {
                Choice::ReceiveOne(unit) => format!(
                    "REVIEW: RECEIVE UNIT {} | {} REMAIN | ENTER CONFIRMS, B CANCELS",
                    unit.serial(),
                    review.remaining_units()
                ),
                Choice::ReceiveAll => format!(
                    "REVIEW: RECEIVE ALL {} IDENTIFIED TONNES | ENTER CONFIRMS, B CANCELS",
                    review.remaining_units()
                ),
                Choice::Abandon => {
                    "REVIEW: ABANDON FREIGHT | NO REWARD OR RESTOCK | ENTER / B".into()
                }
                Choice::Accept => {
                    "REVIEW: ACCEPT EIGHT TONNES FOR RENAS | 730 CR ON COMPLETION | ENTER / B"
                        .into()
                }
            }
        } else {
            "T SELECTS THE CURRENT CONTACT; R OPENS A REVIEW BEFORE ENTER CAN CONFIRM".into()
        };
        lines.push(
            notice
                .unwrap_or("CLICK THE FLIGHT VIEW TO FOCUS THE CONTROLS")
                .to_string(),
        );
        let compact_flight = snapshot.mode == FlightMode::Flight
            && self.pending_practice.is_none()
            && self.pending.is_none()
            && !self.show_objective
            && !self.equipment_open
            && !self.market_open;
        if !compact_flight {
            lines.push(review_line);
            lines.push(if self.equipment_open {
            "K BUY AND FIT | U STORE OWNED BAY | Y REFIT OWNED BAY | O CLOSE".into()
        } else if self.market_open {
            "[ PREVIOUS | ] NEXT | G BUY 1 UNIT | H SELL 1 UNIT | M CLOSE MARKET | F REFUEL".into()
        } else {
            "L LAUNCH | J JUMP | T CONTACT | R REVIEW | V RECEIVE ALL | X ABANDON | ENTER CONFIRM | B BACK".into()
            });
        }
        if let Some(selection) = self.pending_practice {
            let terms = match selection {
                PracticeSelection::Consent(_) => {
                    "Hit the three uncrewed beacons with the fitted front Pulse Laser. Stop firing for three uninterrupted seconds in flight, then dock at Avenor and review reception with F8. No fee or cash reward."
                }
                PracticeSelection::Reception(_) => {
                    "Confirm reception at the Avenor main berth. The first Safe Gunnery Family Trace stays recorded once. No cash reward."
                }
                PracticeSelection::Closure(review) => match review.outcome() {
                    PracticeOutcome::Failed => {
                        "Close this failed attempt. Its ordinary hit and all weapon history stay recorded. No reward, refund or ammunition refill. F10 can review a fresh attempt after closure. B cancels."
                    }
                    _ => {
                        "Abandon this incomplete attempt. Its history stays recorded. No reward, refund or ammunition refill. F10 can review a fresh attempt after closure. B cancels."
                    }
                },
            };
            append_paragraph(&mut lines, terms);
            return lines;
        }
        if self.append_mission_panel(world, &mut lines) {
            return lines;
        }
        if self.equipment_open {
            if let Err(error) = crate::equipment::append_panel(world, &mut lines) {
                append_paragraph(&mut lines, &error);
            }
            return lines;
        }
        if self.market_open {
            if let Err(error) =
                crate::market::append_panel(world, self.selected_commodity, &mut lines)
            {
                append_paragraph(&mut lines, &error);
            }
            return lines;
        }
        if let Some(missiles) = world.missile_snapshot() {
            let remaining = missiles.rounds.iter().flatten().count();
            let pending = missiles
                .projectiles
                .iter()
                .flatten()
                .filter(|p| p.ended.is_none())
                .count();
            let lock = missiles.lock.map_or_else(
                || "NONE".into(),
                |id| format!("BEACON {}", id.ordinal() + 1),
            );
            if snapshot.mode == FlightMode::Flight {
                lines.push(format!(
                    "MISSILES {remaining}/3 | {} | LOCK {lock} | IN FLIGHT {pending} | COOLDOWN {}",
                    if missiles.armed { "ARMED" } else { "SAFE" },
                    missiles.cooldown_ticks
                ));
                lines.push("F1 NEXT LOCK | F2 ARM/SAFE | F3 LAUNCH | F4 SAFE AND UNLOCK".into());
            }
            lines.push("F5 SAVES COMPLETE MISSILE SESSION | F9 CONTINUES SAVED STATE".into());
        }
        if let Some(line) = practice_line(world) {
            lines.push(line);
        }
        let credit = world.credits_tenths().unwrap_or(0);
        if let Some(navigation) = snapshot.navigation {
            lines.push(format!(
                "SPEED {:.1} | FUEL {:.1}/13.0 | CREDITS {}.{} CR | TICK {}",
                snapshot.speed,
                f32::from(navigation.fuel_units) / 10.0,
                credit / 10,
                credit % 10,
                snapshot.tick
            ));
            if let Some(travel) = navigation.travel {
                lines.push(format!(
                    "JUMP COUNTDOWN: {:.1} SECONDS",
                    travel.remaining_ticks as f32 / 60.0
                ));
            }
        }
        lines.push(format!(
            "POSITION {:.0} {:.0} {:.0} | DOCK: ALIGN WITH ENTRANCE, APPROACH AT SPEED 4",
            snapshot.position[0], snapshot.position[1], snapshot.position[2]
        ));
        if let Some(mission) = world.mission_snapshot() {
            lines.push(format!(
                "FREIGHT HELD {} TONNES | COMPLETED {} | TRUSTED CLUE {}",
                mission.freight.held_tonnes,
                mission.freight.completed_jobs,
                if mission.freight.clue.is_some() {
                    "AWARDED"
                } else {
                    "NONE"
                }
            ));
            if !compact_flight {
                lines.push(match mission.contact {
                    Some(npc) => format!("CONTACT: {}", npc.name()),
                    None => "T: CONTACT THE CURRENT STATION CREW WHEN DOCKED".into(),
                });
            }
        }
        if !compact_flight && let Some(service) = world.service_snapshot() {
            let market = service
                .current_market
                .map(|quote| {
                    format!(
                        "BUY {}.{} / SELL {}.{} CR | STOCK {}",
                        quote.ask_tenths / 10,
                        quote.ask_tenths % 10,
                        quote.bid_tenths / 10,
                        quote.bid_tenths % 10,
                        quote.stock
                    )
                })
                .unwrap_or_else(|| "MARKET REQUIRES A SUPPORTED CURRENT BERTH".into());
            lines.push(format!(
                "FOOD {}T | HOLD {}/{}T | {market}",
                service.food_tonnes, service.used_tonnes, service.hold_capacity_tonnes
            ));
        }
        if snapshot.mode == FlightMode::Flight
            && snapshot.navigation.is_some_and(|nav| nav.travel.is_none())
            && let Some(laser) = world.pulse_laser_snapshot()
        {
            lines.push(format!(
                "Z FIRE FRONT PULSE | ENERGY {}.{:02} | HEAT {}.{} | COOLDOWN {} TICKS",
                laser.energy_hundredths / 100,
                laser.energy_hundredths % 100,
                laser.heat_fifths / 5,
                (laser.heat_fifths % 5) * 2,
                laser.cooldown_ticks
            ));
        }
        lines.push("W/S THRUST | ARROWS PITCH | A/D YAW | Q/E ROLL | SPACE BRAKE | J JUMP".into());
        lines.push(
            "O EQUIPMENT | M MARKET | G BUY | H SELL | F REFUEL | P PAUSE | F11 FULLSCREEN".into(),
        );
        lines.push("I MISSION DETAILS | F5 SAVE | F9 RELOAD SAVE | ESC SAVE AND QUIT".into());
        lines
    }
}

fn practice_line(world: &World) -> Option<String> {
    let snapshot = world.snapshot();
    let practice = world.practice_snapshot()?;
    let reception = world.practice_reception_snapshot();
    let ledger = world.practice_ledger_snapshot();
    let outcome = ledger
        .as_ref()
        .and_then(|l| l.attempts.last())
        .and_then(|a| a.outcome);
    let line = if let Some(outcome) = outcome {
        let result = match outcome {
            PracticeOutcome::Complete => "COMPLETE",
            PracticeOutcome::Failed => "FAILED",
            PracticeOutcome::Abandoned => "ABANDONED",
        };
        let next = if world.review_practice(snapshot.tick).is_ok() {
            "F10 REVIEW NEW ATTEMPT"
        } else {
            "NEW REVIEW REQUIRES AVAILABLE AVENOR OFFER"
        };
        format!("PRACTICE {result} | {next}")
    } else {
        let close = if world.review_practice_closure(snapshot.tick).is_ok() {
            " | F7 REVIEW CLOSURE"
        } else {
            ""
        };
        let state = match practice.status {
            PracticeStatus::Unaccepted => {
                if world.review_practice(snapshot.tick).is_ok() {
                    "F10 REVIEW AVENOR PRACTICE".into()
                } else {
                    "PRACTICE REVIEW REQUIRES AVENOR BERTH".into()
                }
            }
            PracticeStatus::Consented => "PRACTICE ACCEPTED | L LAUNCH".into(),
            PracticeStatus::Failed => "PRACTICE FAILED | ORDINARY TARGET HIT".into(),
            PracticeStatus::Active if practice.first_hits < 3 => {
                format!("PRACTICE BEACONS HIT {}/3 | Z FIRE", practice.first_hits)
            }
            PracticeStatus::Active => match reception {
                Some(state) if state.ready => "CEASEFIRE COMPLETE | DOCK AT AVENOR".into(),
                Some(state) => format!(
                    "CEASEFIRE {:.2} / {:.2} SEC | KEEP Z RELEASED",
                    f32::from(state.quiet_ticks) / 60.0,
                    f32::from(PRACTICE_QUIET_TICKS) / 60.0,
                ),
                _ => "PRACTICE IN PROGRESS".into(),
            },
            PracticeStatus::Retired => {
                if world.review_practice_reception(snapshot.tick).is_ok() {
                    "F8 REVIEW PRACTICE RECEPTION".into()
                } else if reception.is_some_and(|state| state.ready) {
                    "DOCK AT AVENOR FOR PRACTICE RECEPTION".into()
                } else {
                    format!("RANGE CLOSED | BEACONS HIT {}/3", practice.first_hits)
                }
            }
        };
        format!("{state}{close}")
    };
    Some(line)
}

/// Original fixed dialogue is word-wrapped before the renderer's column bound.
fn append_paragraph(lines: &mut Vec<String>, paragraph: &str) {
    let mut line = String::new();
    for word in paragraph.split_whitespace() {
        if !line.is_empty() && line.len() + 1 + word.len() > crate::hud::MAX_COLUMNS {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
}

fn missile_command(world: &mut World, action: Action) -> Result<String, String> {
    let state = world
        .missile_snapshot()
        .ok_or("An owned commander is required")?;
    let tick = world.snapshot().tick;
    match action {
        Action::CycleMissileLock => {
            let beacons = world
                .practice_snapshot()
                .and_then(|p| p.beacons)
                .ok_or("No active practice beacons to lock")?;
            let next = match state
                .lock
                .and_then(|id| beacons.iter().position(|b| b.id == id))
            {
                Some(index) => beacons.get(index + 1).map(|b| b.id),
                None => Some(beacons[0].id),
            };
            world
                .request_missile_lock(next, tick)
                .map_err(|e| e.to_string())?;
            Ok(next.map_or_else(
                || "Missile lock cleared".into(),
                |id| format!("Missile locked on beacon {}", id.ordinal() + 1),
            ))
        }
        Action::ToggleMissileArm => {
            world
                .request_missile_arm(!state.armed, tick)
                .map_err(|e| e.to_string())?;
            Ok(if state.armed {
                "Missile safe"
            } else {
                "Missile armed"
            }
            .into())
        }
        Action::FireMissile => {
            world
                .request_fire_missile(tick)
                .map_err(|e| e.to_string())?;
            Ok("Missile launched; F5 saves the owned round and projectile state".into())
        }
        Action::ClearMissile => {
            world
                .request_missile_arm(false, tick)
                .map_err(|e| e.to_string())?;
            world
                .request_missile_lock(None, tick)
                .map_err(|e| e.to_string())?;
            Ok("Missile safe and lock cleared".into())
        }
        _ => Err("Unsupported missile control".into()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn closure_review_history_and_fresh_consent_are_visible_without_reopening_old_attempt() {
        let mut world = World::new_commander(992).unwrap_or_else(|e| panic!("{e}"));
        let mut session = Session::from_world(&world);
        let review = world.review_practice(0).unwrap_or_else(|e| panic!("{e}"));
        world
            .prepare_practice_confirmation(review, 0)
            .unwrap_or_else(|e| panic!("{e}"))
            .commit();
        assert!(
            session
                .lines(&world, true, None)
                .join("\n")
                .contains("F7 REVIEW CLOSURE")
        );
        session
            .command(&mut world, Action::ReviewPracticeClosure)
            .unwrap_or_else(|e| panic!("{e}"));
        let panel = session.lines(&world, true, None).join(" ");
        for term in [
            "CLOSE PRACTICE ATTEMPT",
            "ENTER CONFIRMS AND SAVES",
            "B CANCELS",
            "Abandon",
            "No reward, refund",
            "ammunition refill",
        ] {
            assert!(panel.contains(term), "Missing closure term: {term}");
        }
        session
            .command(&mut world, Action::Back)
            .unwrap_or_else(|e| panic!("{e}"));
        let review = world
            .review_practice_closure(0)
            .unwrap_or_else(|e| panic!("{e}"));
        world
            .prepare_practice_closure(review, 0)
            .unwrap_or_else(|e| panic!("{e}"))
            .commit();
        let closed = session.lines(&world, true, None).join("\n");
        assert!(closed.contains("PRACTICE ABANDONED | F10 REVIEW NEW ATTEMPT"));
        session
            .command(&mut world, Action::MissionDetails)
            .unwrap_or_else(|e| panic!("{e}"));
        let history = session.lines(&world, true, None).join("\n");
        assert!(history.contains("ABANDONED 1"));
        assert!(history.contains("TRACE NOT YET RECORDED"));
        session.invalidate();
        let review = world.review_practice(0).unwrap_or_else(|e| panic!("{e}"));
        world
            .prepare_practice_confirmation(review, 0)
            .unwrap_or_else(|e| panic!("{e}"))
            .commit();
        let fresh = session.lines(&world, true, None).join("\n");
        assert!(fresh.contains("PRACTICE ACCEPTED | L LAUNCH"));
        assert!(!fresh.contains("PRACTICE ABANDONED"));
        assert!(!fresh.contains("F8 REVIEW"));
    }

    #[test]
    fn missile_controls_cycle_actual_locks_consume_a_round_and_continue_state() {
        fn assert_visible(session: &Session, world: &World, warning: &str) {
            let lines = session.lines(world, true, None);
            let budget = crate::hud::line_budget(640, 360);
            assert!(
                lines.len() <= budget,
                "Flight HUD has {} rows, budget {budget}",
                lines.len()
            );
            let visible = lines
                .into_iter()
                .take(budget)
                .collect::<Vec<_>>()
                .join("\n");
            for required in [
                "MISSILES",
                "PRACTICE BEACONS",
                warning,
                "Z FIRE FRONT PULSE",
                "SPACE BRAKE",
                "F5 SAVE",
                "ESC SAVE AND QUIT",
            ] {
                assert!(
                    visible.contains(required),
                    "Visible flight HUD omitted {required}"
                );
            }
        }
        let mut world = World::new_commander(991).unwrap_or_else(|error| panic!("{error}"));
        let review = world
            .review_practice(0)
            .unwrap_or_else(|error| panic!("{error}"));
        world
            .prepare_practice_confirmation(review, 0)
            .unwrap_or_else(|error| panic!("{error}"))
            .commit();
        world
            .request_launch(0)
            .unwrap_or_else(|error| panic!("{error}"));
        world
            .step(meridian_core::FlightInput::default())
            .unwrap_or_else(|error| panic!("{error}"));
        let mut session = Session::from_world(&world);
        assert_visible(&session, &world, "F5 SAVES COMPLETE MISSILE SESSION");
        let before = world
            .missile_snapshot()
            .unwrap_or_else(|| panic!("Expected missile_snapshot"));
        let beacons = world
            .practice_snapshot()
            .unwrap_or_else(|| panic!("Expected practice_snapshot"))
            .beacons
            .unwrap_or_else(|| panic!("Expected beacons"));
        for expected in [
            Some(beacons[0].id),
            Some(beacons[1].id),
            Some(beacons[2].id),
            None,
        ] {
            session
                .command(&mut world, Action::CycleMissileLock)
                .unwrap_or_else(|error| panic!("{error}"));
            assert_eq!(
                world
                    .missile_snapshot()
                    .unwrap_or_else(|| panic!("Expected missile_snapshot"))
                    .lock,
                expected
            );
        }
        session
            .command(&mut world, Action::CycleMissileLock)
            .unwrap_or_else(|error| panic!("{error}"));
        session
            .command(&mut world, Action::ToggleMissileArm)
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(world.save_bytes().is_ok());
        assert_visible(&session, &world, "F5 SAVES COMPLETE MISSILE SESSION");
        session
            .command(&mut world, Action::ClearMissile)
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(world.save_bytes().is_ok());
        session
            .command(&mut world, Action::CycleMissileLock)
            .unwrap_or_else(|error| panic!("{error}"));
        session
            .command(&mut world, Action::ToggleMissileArm)
            .unwrap_or_else(|error| panic!("{error}"));
        session
            .command(&mut world, Action::FireMissile)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_visible(&session, &world, "F5 SAVES COMPLETE MISSILE SESSION");
        let after = world
            .missile_snapshot()
            .unwrap_or_else(|| panic!("Expected missile_snapshot"));
        assert_eq!(after.rounds[2], None);
        assert_eq!(after.rounds[..2], before.rounds[..2]);
        let projectile = after
            .projectiles
            .into_iter()
            .flatten()
            .next()
            .unwrap_or_else(|| panic!("Expected next"));
        assert_eq!(
            projectile.origin.item,
            before.rounds[2]
                .unwrap_or_else(|| panic!("Expected rounds[2]"))
                .item
        );
        assert_eq!(projectile.target, beacons[0].id);
        assert!(
            session
                .lines(&world, true, None)
                .iter()
                .any(|s| s.contains("F5 SAVES COMPLETE MISSILE SESSION"))
        );
        session
            .command(&mut world, Action::ClearMissile)
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(world.save_bytes().is_ok());
        let bytes = world.save_bytes().unwrap_or_else(|error| panic!("{error}"));
        let continued = World::from_save_bytes(&bytes).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(continued.missile_snapshot(), world.missile_snapshot());
        assert_eq!(continued.practice_snapshot(), world.practice_snapshot());
        assert_eq!(continued.combat_snapshot(), world.combat_snapshot());
        let snapshot = world.missile_snapshot();
        world
            .request_pause(true, world.snapshot().tick)
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(session.command(&mut world, Action::FireMissile).is_err());
        assert_eq!(world.missile_snapshot(), snapshot);
    }

    use super::*;
    fn world() -> World {
        World::new_commander(7001).unwrap_or_else(|e| panic!("{e}"))
    }
    #[test]
    fn ui_requires_actual_contact_and_explicit_fresh_confirmation() {
        let mut world = world();
        let mut ui = Session::default();
        assert!(ui.command(&mut world, Action::Confirm).is_err());
        assert!(ui.command(&mut world, Action::Review).is_err());
        assert!(ui.command(&mut world, Action::Contact).is_ok());
        assert!(ui.command(&mut world, Action::Review).is_ok());
        assert_eq!(
            world.mission_snapshot().map(|m| m.freight.held_tonnes),
            Some(0)
        );
        assert!(ui.command(&mut world, Action::Back).is_ok());
        assert!(ui.command(&mut world, Action::Confirm).is_err());
        assert!(ui.command(&mut world, Action::Review).is_ok());
        assert!(ui.command(&mut world, Action::Confirm).is_ok());
        assert_eq!(
            world.mission_snapshot().map(|m| m.freight.held_tonnes),
            Some(8)
        );
        assert_eq!(world.credits_tenths(), Some(7200));
        assert!(ui.command(&mut world, Action::Confirm).is_err());
    }
    #[test]
    fn focus_pause_invalidates_selection_and_rendering_is_read_only() {
        let mut world = world();
        let mut ui = Session::default();
        ui.command(&mut world, Action::Contact)
            .unwrap_or_else(|e| panic!("{e}"));
        ui.command(&mut world, Action::Review)
            .unwrap_or_else(|e| panic!("{e}"));
        ui.invalidate();
        world
            .request_pause(true, world.snapshot().tick)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(ui.command(&mut world, Action::Confirm).is_err());
        let before = (
            world.snapshot(),
            world.mission_snapshot(),
            world.credits_tenths(),
        );
        let lines = ui.lines(&world, false, None);
        assert!(lines.iter().any(|l| l.contains("F5 SAVE")));
        assert_eq!(
            before,
            (
                world.snapshot(),
                world.mission_snapshot(),
                world.credits_tenths()
            )
        );
    }
    fn arrive_with_identified_freight() -> (World, Session) {
        use meridian_core::FlightInput;
        use meridian_core::navigation::{BASIC_SPOOL_TICKS, DockOrigin};
        let mut world = world();
        let mut ui = Session::default();
        for action in [
            Action::Contact,
            Action::Review,
            Action::Confirm,
            Action::Launch,
        ] {
            ui.command(&mut world, action)
                .unwrap_or_else(|e| panic!("{e}"));
        }
        world
            .step(FlightInput::default())
            .unwrap_or_else(|e| panic!("{e}"));
        ui.command(&mut world, Action::Jump)
            .unwrap_or_else(|e| panic!("{e}"));
        for _ in 0..BASIC_SPOOL_TICKS {
            world
                .step(FlightInput::default())
                .unwrap_or_else(|e| panic!("{e}"));
        }
        assert_eq!(
            world.snapshot().navigation.map(|n| n.system),
            Some(SystemId::RENAS)
        );
        assert!(world.station_contacts().is_empty());
        assert!(ui.command(&mut world, Action::Contact).is_err());
        for _ in 0..9000 {
            let before = world.snapshot();
            if before.mode == FlightMode::Docked {
                break;
            }
            let target_speed = if before.position[2] > 2500.0 {
                34.3
            } else {
                4.0
            };
            world
                .step(FlightInput {
                    throttle: ((target_speed - before.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
                    ..FlightInput::default()
                })
                .unwrap_or_else(|e| panic!("{e}"));
        }
        assert_eq!(
            world.current_dock_context().map(|d| d.origin()),
            Some(DockOrigin::PhysicalEntry)
        );
        ui.command(&mut world, Action::Contact)
            .unwrap_or_else(|e| panic!("{e}"));
        (world, ui)
    }

    #[test]
    fn actual_desktop_commands_deliver_eight_units_after_public_flight_and_entry() {
        let (mut world, mut ui) = arrive_with_identified_freight();
        for index in 0..8 {
            ui.command(&mut world, Action::Review)
                .unwrap_or_else(|e| panic!("{e}"));
            ui.command(&mut world, Action::Confirm)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(
                world.mission_snapshot().map(|m| m.freight.held_tonnes),
                Some(7 - index)
            );
            assert_eq!(
                world.credits_tenths(),
                Some(if index == 7 { 14500 } else { 7200 })
            );
        }
        assert!(
            world
                .mission_snapshot()
                .and_then(|m| m.freight.clue)
                .is_some()
        );
        assert!(ui.command(&mut world, Action::Review).is_err());
        assert_eq!(world.credits_tenths(), Some(14500));
        assert!(ui.command(&mut world, Action::Refuel).is_ok());
        assert_eq!(world.credits_tenths(), Some(14432));
        assert_eq!(world.snapshot().navigation.map(|n| n.fuel_units), Some(130));
    }
    #[test]
    fn narrow_height_keeps_current_review_refusal_and_action_keys_first() {
        let mut world = world();
        let mut ui = Session::default();
        ui.command(&mut world, Action::Contact)
            .unwrap_or_else(|e| panic!("{e}"));
        ui.command(&mut world, Action::Review)
            .unwrap_or_else(|e| panic!("{e}"));
        let lines = ui.lines(&world, false, Some("ENTRY REFUSED: SLOW DOWN"));
        let count = crate::hud::line_budget(1280, 100);
        let visible = &lines[..count];
        assert_eq!(count, 4);
        assert!(visible[1].contains("ENTRY REFUSED"));
        assert!(visible[2].contains("REVIEW: ACCEPT EIGHT"));
        assert!(visible[3].contains("ENTER CONFIRM"));
        assert!(lines.len() <= crate::hud::line_budget(640, 360));
    }
    #[test]
    fn ordinary_food_ui_changes_the_real_shared_wallet_hold_and_stock() {
        let mut world = world();
        let mut ui = Session::default();
        let initial = (world.service_snapshot(), world.credits_tenths());
        assert!(ui.command(&mut world, Action::SellSelected).is_err());
        assert_eq!(initial, (world.service_snapshot(), world.credits_tenths()));
        ui.command(&mut world, Action::BuySelected)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(world.credits_tenths(), Some(6974));
        assert_eq!(
            world.service_snapshot().map(|s| (
                s.food_tonnes,
                s.used_tonnes,
                s.current_market.map(|m| m.stock)
            )),
            Some((1, 1, Some(127)))
        );
        for action in [Action::Contact, Action::Review, Action::Confirm] {
            ui.command(&mut world, action)
                .unwrap_or_else(|e| panic!("{e}"));
        }
        assert!(ui.command(&mut world, Action::Review).is_err());
        ui.command(&mut world, Action::BuySelected)
            .unwrap_or_else(|e| panic!("{e}"));
        ui.command(&mut world, Action::SellSelected)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(world.credits_tenths(), Some(6956));
        assert_eq!(
            world
                .service_snapshot()
                .map(|s| (s.food_tonnes, s.used_tonnes)),
            Some((1, 9))
        );
        assert_eq!(
            world.mission_snapshot().map(|m| m.freight.held_tonnes),
            Some(8)
        );
        assert!(ui.lines(&world, true, None).len() <= crate::hud::line_budget(640, 360));
    }
    #[test]
    fn equipment_panel_is_read_only_and_cannot_confirm_a_hidden_freight_offer() {
        let mut world = world();
        let mut ui = Session::default();
        for action in [Action::Contact, Action::Review] {
            ui.command(&mut world, action)
                .unwrap_or_else(|e| panic!("{e}"));
        }
        let saved = world.save_bytes().unwrap_or_else(|e| panic!("{e}"));
        ui.command(&mut world, Action::ToggleEquipment)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(ui.command(&mut world, Action::Confirm).is_err());
        let lines = ui.lines(&world, true, None);
        assert!(lines.iter().any(|line| line.contains("LARGE CARGO BAY")));
        assert!(lines.iter().any(|line| line.contains("2800.0 CR")));
        assert!(lines.iter().any(|line| line.contains("HOLD 0/92T")));
        assert!(lines.len() <= crate::hud::line_budget(640, 360));
        assert!(
            lines
                .iter()
                .all(|line| line.len() <= crate::hud::MAX_COLUMNS)
        );
        assert_eq!(world.save_bytes().unwrap_or_else(|e| panic!("{e}")), saved);
        ui.command(&mut world, Action::ToggleMarket)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(!ui.equipment_open);
        assert!(ui.market_open);
        assert!(ui.command(&mut world, Action::PurchaseCargoBay).is_err());
        assert_eq!(world.save_bytes().unwrap_or_else(|e| panic!("{e}")), saved);
    }

    #[test]
    fn equipment_keys_require_open_panel_and_refusals_preserve_the_commander() {
        let mut world = world();
        let mut ui = Session::default();
        let saved = world.save_bytes().unwrap_or_else(|e| panic!("{e}"));
        for action in [
            Action::PurchaseCargoBay,
            Action::FitCargoBay,
            Action::UnfitCargoBay,
        ] {
            assert!(ui.command(&mut world, action).is_err());
        }
        ui.command(&mut world, Action::ToggleEquipment)
            .unwrap_or_else(|e| panic!("{e}"));
        let refusal = ui
            .command(&mut world, Action::PurchaseCargoBay)
            .err()
            .unwrap_or_else(|| panic!("720 CR cannot fund a 2800 CR purchase"));
        assert!(refusal.contains("InsufficientFunds"));
        assert!(ui.command(&mut world, Action::FitCargoBay).is_err());
        assert!(ui.command(&mut world, Action::UnfitCargoBay).is_err());
        assert_eq!(world.save_bytes().unwrap_or_else(|e| panic!("{e}")), saved);
        let cold = World::from_save_bytes(&saved).unwrap_or_else(|e| panic!("{e}"));
        let restored = Session::from_world(&cold);
        assert!(!restored.equipment_open);
    }

    #[test]
    fn market_selection_reaches_every_original_row_without_mutating_world() {
        let mut world = world();
        let mut ui = Session::default();
        let saved = world.save_bytes().unwrap_or_else(|e| panic!("{e}"));
        ui.command(&mut world, Action::ToggleMarket)
            .unwrap_or_else(|e| panic!("{e}"));
        let expected = [0, 11, 6, 3, 16, 9, 8, 10, 2, 7, 12, 13, 15, 14, 1, 4, 5];
        for index in expected {
            assert_eq!(ui.selected_commodity.index(), index);
            let lines = ui.lines(&world, true, None);
            assert!(lines[3].contains("G BUY 1 UNIT"));
            assert!(
                lines[5].contains(
                    ui.selected_commodity
                        .definition()
                        .unwrap_or_else(|e| panic!("{e:?}"))
                        .name
                )
            );
            assert!(lines.len() <= crate::hud::line_budget(640, 360));
            assert!(lines.iter().all(|l| l.len() <= crate::hud::MAX_COLUMNS));
            ui.command(&mut world, Action::NextCommodity)
                .unwrap_or_else(|e| panic!("{e}"));
        }
        assert_eq!(ui.selected_commodity, CommodityId::Food);
        ui.command(&mut world, Action::PreviousCommodity)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(ui.selected_commodity, CommodityId::Luxuries);
        assert_eq!(world.save_bytes().unwrap_or_else(|e| panic!("{e}")), saved);
    }

    #[test]
    fn selected_gold_trade_uses_native_kilograms_and_actual_wallet_and_stock() {
        let mut world = world();
        let mut ui = Session::default();
        for _ in 0..11 {
            ui.command(&mut world, Action::NextCommodity)
                .unwrap_or_else(|e| panic!("{e}"));
        }
        assert_eq!(ui.selected_commodity, CommodityId::Gold);
        ui.command(&mut world, Action::BuySelected)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(world.credits_tenths(), Some(2855));
        assert_eq!(world.commodity_quantity(CommodityId::Gold), Some(1));
        assert_eq!(
            world.commodity_quote(CommodityId::Gold).map(|q| q.stock),
            Some(46)
        );
        assert_eq!(world.service_snapshot().map(|s| s.used_tonnes), Some(0));
        let lines = ui.lines(&world, true, None).join(" ");
        assert!(lines.contains("HELD 1 kg | RESERVED 0 kg"));
        ui.command(&mut world, Action::SellSelected)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(world.credits_tenths(), Some(6864));
        assert_eq!(world.commodity_quantity(CommodityId::Gold), Some(0));
        assert_eq!(
            world.commodity_quote(CommodityId::Gold).map(|q| q.stock),
            Some(47)
        );
    }

    #[test]
    fn market_cancels_hidden_freight_reviews_and_refuses_trades_without_berth() {
        let mut world = world();
        let mut ui = Session::default();
        for action in [Action::Contact, Action::Review, Action::ToggleMarket] {
            ui.command(&mut world, action)
                .unwrap_or_else(|e| panic!("{e}"));
        }
        assert!(ui.command(&mut world, Action::Confirm).is_err());
        assert_eq!(
            world.mission_snapshot().map(|m| m.freight.held_tonnes),
            Some(0)
        );
        ui.command(&mut world, Action::Launch)
            .unwrap_or_else(|e| panic!("{e}"));
        let saved = world.save_bytes().unwrap_or_else(|e| panic!("{e}"));
        assert!(ui.command(&mut world, Action::BuySelected).is_err());
        assert!(
            ui.lines(&world, true, None)
                .iter()
                .any(|l| l.contains("BERTH REQUIRED"))
        );
        assert_eq!(world.save_bytes().unwrap_or_else(|e| panic!("{e}")), saved);
    }

    #[test]
    fn receive_all_requires_review_and_settles_the_real_job_once() {
        let (mut world, mut ui) = arrive_with_identified_freight();
        ui.command(&mut world, Action::ReviewAll)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            world.mission_snapshot().map(|m| m.freight.held_tonnes),
            Some(8)
        );
        assert_eq!(world.credits_tenths(), Some(7200));
        assert!(ui.lines(&world, true, None)[2].contains("RECEIVE ALL 8"));
        ui.command(&mut world, Action::Back)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(ui.command(&mut world, Action::Confirm).is_err());
        ui.command(&mut world, Action::ReviewAll)
            .unwrap_or_else(|e| panic!("{e}"));
        ui.command(&mut world, Action::Confirm)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            world.mission_snapshot().map(|m| m.freight.held_tonnes),
            Some(0)
        );
        assert_eq!(world.credits_tenths(), Some(14500));
        assert!(
            world
                .mission_snapshot()
                .and_then(|m| m.freight.clue)
                .is_some()
        );
        assert!(ui.command(&mut world, Action::Confirm).is_err());
        assert!(ui.command(&mut world, Action::ReviewAll).is_err());
        assert_eq!(world.credits_tenths(), Some(14500));
    }

    #[test]
    fn abandonment_is_an_explicit_cancellable_review_and_never_restocks() {
        let mut world = world();
        let mut ui = Session::default();
        for action in [Action::ReviewAll, Action::ReviewAbandonment] {
            assert!(ui.command(&mut world, action).is_err());
        }
        ui.command(&mut world, Action::Contact)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(ui.command(&mut world, Action::ReviewAll).is_err());
        assert!(ui.command(&mut world, Action::ReviewAbandonment).is_err());
        for index in 0..2 {
            for action in [Action::Review, Action::Confirm, Action::ReviewAbandonment] {
                ui.command(&mut world, action)
                    .unwrap_or_else(|e| panic!("{e}"));
            }
            assert_eq!(
                world.mission_snapshot().map(|m| m.freight.held_tonnes),
                Some(8)
            );
            assert!(ui.lines(&world, true, None)[2].contains("ABANDON FREIGHT"));
            ui.command(&mut world, Action::Back)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(ui.command(&mut world, Action::Confirm).is_err());
            ui.command(&mut world, Action::ReviewAbandonment)
                .unwrap_or_else(|e| panic!("{e}"));
            ui.command(&mut world, Action::Confirm)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(ui.job.is_none());
            assert_eq!(
                world.mission_snapshot().map(|m| m.freight.held_tonnes),
                Some(0)
            );
            assert_eq!(world.credits_tenths(), Some(7200));
            assert!(
                world
                    .mission_snapshot()
                    .and_then(|m| m.freight.clue)
                    .is_none()
            );
            if index == 1 {
                let saved = world.save_bytes().unwrap_or_else(|e| panic!("{e}"));
                world = World::from_save_bytes(&saved).unwrap_or_else(|e| panic!("{e}"));
                ui = Session::from_world(&world);
                ui.command(&mut world, Action::Contact)
                    .unwrap_or_else(|e| panic!("{e}"));
                assert!(ui.command(&mut world, Action::Review).is_err());
            }
        }
    }
    #[test]
    fn original_terms_fit_the_review_and_details_cannot_confirm_a_hidden_review() {
        let mut world = world();
        let mut ui = Session::default();
        for action in [Action::Contact, Action::Review] {
            ui.command(&mut world, action)
                .unwrap_or_else(|e| panic!("{e}"));
        }
        let before = (
            world.snapshot(),
            world.mission_snapshot(),
            world.credits_tenths(),
        );
        let lines = ui.lines(&world, true, None);
        assert!(lines.len() <= crate::hud::line_budget(640, 360));
        assert!(
            lines
                .iter()
                .all(|line| line.len() <= crate::hud::MAX_COLUMNS)
        );
        let terms = lines[4..].join(" ");
        assert!(terms.contains(presentation::OFFER_BRIEFING));
        assert!(terms.contains(presentation::IDENTITY_NOTICE));
        assert_eq!(
            before,
            (
                world.snapshot(),
                world.mission_snapshot(),
                world.credits_tenths()
            )
        );
        ui.command(&mut world, Action::MissionDetails)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(ui.command(&mut world, Action::Confirm).is_err());
        assert_eq!(
            world.mission_snapshot().map(|m| m.freight.held_tonnes),
            Some(0)
        );
        for action in [Action::Review, Action::Confirm, Action::MissionDetails] {
            ui.command(&mut world, action)
                .unwrap_or_else(|e| panic!("{e}"));
        }
        let details = ui.lines(&world, true, None);
        assert!(details.len() <= crate::hud::line_budget(640, 360));
        assert!(
            details
                .iter()
                .all(|line| line.len() <= crate::hud::MAX_COLUMNS)
        );
        assert!(
            details[4..]
                .join(" ")
                .contains(presentation::ACCEPTED_JOURNAL)
        );
    }
}
