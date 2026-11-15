//! Desktop selections hold only opaque World-issued values; they grant no authority.
use meridian_core::freight::g02::{Choice, Command, ContactDefinition, Event, JobId, Review};
use meridian_core::navigation::SystemId;
use meridian_core::station_services;
use meridian_core::{FlightMode, World};

#[derive(Clone, Copy)]
pub enum Action {
    Contact,
    Review,
    ReviewAll,
    ReviewAbandonment,
    Confirm,
    Back,
    Refuel,
    BuyFood,
    SellFood,
    Jump,
    Launch,
}

#[derive(Default)]
pub struct Session {
    job: Option<JobId>,
    pending: Option<(Review, Choice)>,
}

impl Session {
    pub fn from_world(world: &World) -> Self {
        Self {
            job: world.active_freight_job(),
            pending: None,
        }
    }

    pub fn invalidate(&mut self) {
        self.pending = None;
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
                    Ok("Freight complete. 730 CR and trusted clue awarded once.".into())
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

    pub fn command(&mut self, world: &mut World, action: Action) -> Result<String, String> {
        match action {
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
                self.invalidate();
                let npc = world
                    .mission_snapshot()
                    .and_then(|m| m.contact)
                    .ok_or("T selects the current station contact first")?;
                let command = match (action, npc.definition()) {
                    (Action::ReviewAbandonment, ContactDefinition::Arlen) => {
                        Command::ReviewAbandonment {
                            actor: npc.actor(),
                            job: self.job.ok_or("No active freight job to abandon")?,
                        }
                    }
                    (Action::Review, ContactDefinition::Arlen) => {
                        if self.job.is_some() {
                            return Err(
                                "Complete the current freight job before accepting another".into(),
                            );
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
            Action::BuyFood | Action::SellFood => {
                self.invalidate();
                let command = match action {
                    Action::BuyFood => station_services::Command::BuyFood,
                    _ => station_services::Command::SellFood,
                };
                let event = world
                    .request_station_service(command, world.snapshot().tick)
                    .map_err(|e| e.to_string())?;
                Ok(match event {
                    station_services::Event::BoughtFood { paid_tenths, .. } => format!(
                        "Bought one Food tonne for {}.{} CR.",
                        paid_tenths / 10,
                        paid_tenths % 10
                    ),
                    station_services::Event::SoldFood {
                        credited_tenths, ..
                    } => format!(
                        "Sold one Food tonne for {}.{} CR.",
                        credited_tenths / 10,
                        credited_tenths % 10
                    ),
                    station_services::Event::Refuelled { .. } => "Fuel service complete.".into(),
                })
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
        let review_line = if let Some((review, choice)) = self.pending {
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
        lines.push(review_line);
        lines.push(
            "L LAUNCH | J JUMP | T CONTACT | R REVIEW | V RECEIVE ALL | X ABANDON | ENTER CONFIRM | B BACK".into(),
        );
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
            lines.push(match mission.contact {
                Some(npc) => format!("CONTACT: {}", npc.name()),
                None => "T: CONTACT THE CURRENT STATION CREW WHEN DOCKED".into(),
            });
        }
        if let Some(service) = world.service_snapshot() {
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
                "FOOD {}T | HOLD {}/92T | {market}",
                service.food_tonnes, service.used_tonnes
            ));
        }
        lines.push("W/S THRUST | ARROWS PITCH | A/D YAW | Q/E ROLL | SPACE BRAKE".into());
        lines.push("G BUY 1 FOOD | H SELL 1 FOOD | F REFUEL | P PAUSE | F11 FULLSCREEN".into());
        lines.push("F5 SAVE | F9 RELOAD SAVE | ESC SAVE AND QUIT".into());
        lines
    }
}

#[cfg(test)]
mod tests {
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
        assert!(ui.command(&mut world, Action::SellFood).is_err());
        assert_eq!(initial, (world.service_snapshot(), world.credits_tenths()));
        ui.command(&mut world, Action::BuyFood)
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
        ui.command(&mut world, Action::BuyFood)
            .unwrap_or_else(|e| panic!("{e}"));
        ui.command(&mut world, Action::SellFood)
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
}
