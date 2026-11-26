//! World-owned original G02 station contact and command lifecycle.
mod persistence;
use super::World;
use crate::freight::g02::{
    self, ActorId, ContactDefinition, ContactFacts, DockFacts, StationKind, WorldFacts,
};
use crate::navigation::{DockContext, SystemId};
use core::fmt;

/// A roster entry created by World at a genuine current dock. Presentation copies grant nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StationNpc {
    dock: DockContext,
    actor: ActorId,
    definition: ContactDefinition,
    roster_revision: u64,
}
impl StationNpc {
    pub fn actor(self) -> ActorId {
        self.actor
    }
    pub fn definition(self) -> ContactDefinition {
        self.definition
    }
    pub fn name(self) -> &'static str {
        match self.definition {
            ContactDefinition::Arlen => "Arlen",
            ContactDefinition::RenasUtilityCrew => "Renas utility receiving crew",
        }
    }
    pub fn dock(self) -> DockContext {
        self.dock
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionError {
    NoCommander,
    StaleCommand,
    NotDocked,
    WrongContact,
    Paused,
    SequenceExhausted,
    SharedCapacity,
    NavigationTrade(super::NavigationTradeError),
    Freight(g02::Error),
    Service(crate::station_services::Error),
    Equipment(crate::equipment::Error),
}
impl fmt::Display for MissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCommander => f.write_str("an owned commander profile is required"),
            Self::StaleCommand => f.write_str("station command does not match the current tick"),
            Self::NotDocked => f.write_str("a current main-orbital dock admission is required"),
            Self::WrongContact => f.write_str("contact is not in this current station roster"),
            Self::Paused => f.write_str("World is paused"),
            Self::SequenceExhausted => f.write_str("station contact sequence exhausted"),
            Self::SharedCapacity => {
                f.write_str("Original mission or active service capacity is full")
            }
            Self::NavigationTrade(error) => write!(f, "{error}"),
            Self::Freight(error) => write!(f, "freight command refused: {error:?}"),
            Self::Service(error) => write!(f, "station service refused: {error:?}"),
            Self::Equipment(error) => write!(f, "{error}"),
        }
    }
}
impl std::error::Error for MissionError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissionSnapshot {
    pub freight: g02::Snapshot,
    pub paused: bool,
    pub contact: Option<StationNpc>,
}

#[derive(Debug)]
pub(super) struct FreightSession {
    pub(super) state: g02::State,
    epoch: u64,
    paused: bool,
    roster: Option<StationNpc>,
    contact: Option<StationNpc>,
}
impl FreightSession {
    pub(super) fn fresh(dock: DockContext) -> Self {
        let mut session = Self {
            state: g02::State::new_commander(dock.owner().seed()),
            epoch: 1,
            paused: false,
            roster: None,
            contact: None,
        };
        session.rebind_dock(Some(dock), 1);
        session
    }
    pub(super) fn next_epoch(&self) -> Result<u64, MissionError> {
        self.epoch
            .checked_add(1)
            .ok_or(MissionError::SequenceExhausted)
    }
    pub(super) fn launch_epoch(&self) -> Result<u64, MissionError> {
        // Resuming and departing each invalidate the prior review authority.
        self.epoch
            .checked_add(1 + u64::from(self.paused))
            .ok_or(MissionError::SequenceExhausted)
    }
    pub(super) fn publish_launch(&mut self, epoch: u64) {
        self.paused = false;
        self.rebind_dock(None, epoch);
    }
    pub(super) fn epoch(&self) -> u64 {
        self.epoch
    }
    /// Called only at World profile creation, launch, jump arrival or physical entry publication.
    pub(super) fn rebind_dock(&mut self, dock: Option<DockContext>, epoch: u64) {
        self.epoch = epoch;
        self.contact = None;
        self.roster = dock.and_then(|dock| {
            let (entity, definition) = match dock.system() {
                SystemId::AVENOR => (2, ContactDefinition::Arlen),
                SystemId::RENAS => (3, ContactDefinition::RenasUtilityCrew),
                _ => return None,
            };
            Some(StationNpc {
                dock,
                actor: ActorId {
                    entity,
                    generation: epoch,
                },
                definition,
                roster_revision: epoch,
            })
        });
    }
}

impl World {
    pub fn mission_snapshot(&self) -> Option<MissionSnapshot> {
        self.freight_session
            .as_ref()
            .map(|session| MissionSnapshot {
                freight: session.state.snapshot(),
                paused: session.paused,
                contact: session.contact,
            })
    }

    /// Existing validated custody handle for presentation; it grants no contact or receiving consent.
    pub fn active_freight_job(&self) -> Option<g02::JobId> {
        self.freight_session
            .as_ref()
            .and_then(|session| session.state.active_job())
    }

    pub fn station_contacts(&self) -> Vec<StationNpc> {
        let Some(session) = &self.freight_session else {
            return Vec::new();
        };
        session
            .roster
            .filter(|npc| self.dock_context_is_current(&npc.dock))
            .into_iter()
            .collect()
    }

    pub(super) fn station_command_admission(
        &self,
        tick: u64,
    ) -> Result<&FreightSession, MissionError> {
        if tick != self.state.tick {
            return Err(MissionError::StaleCommand);
        }
        let session = self
            .freight_session
            .as_ref()
            .ok_or(MissionError::NoCommander)?;
        if session.paused {
            return Err(MissionError::Paused);
        }
        Ok(session)
    }

    pub fn request_contact(
        &mut self,
        npc: StationNpc,
        expected_tick: u64,
    ) -> Result<(), MissionError> {
        let session = self.station_command_admission(expected_tick)?;
        if !self.dock_context_is_current(&npc.dock) {
            return Err(MissionError::NotDocked);
        }
        if session.roster != Some(npc) {
            return Err(MissionError::WrongContact);
        }
        let epoch = session.next_epoch()?;
        let session = self
            .freight_session
            .as_mut()
            .ok_or(MissionError::NoCommander)?;
        session.epoch = epoch;
        session.contact = Some(npc);
        Ok(())
    }

    /// Normal application pause command; it cannot supply mission consent or location.
    pub fn request_pause(&mut self, paused: bool, expected_tick: u64) -> Result<(), MissionError> {
        if expected_tick != self.state.tick {
            return Err(MissionError::StaleCommand);
        }
        let session = self
            .freight_session
            .as_mut()
            .ok_or(MissionError::NoCommander)?;
        let epoch = session.next_epoch()?;
        session.epoch = epoch;
        session.paused = paused;
        if paused {
            self.cancel_auto_docking();
            self.cancel_targeting();
        }
        Ok(())
    }

    pub(super) fn is_paused(&self) -> bool {
        self.freight_session
            .as_ref()
            .is_some_and(|session| session.paused)
    }

    fn freight_facts(&self) -> Result<WorldFacts, MissionError> {
        let session = self
            .freight_session
            .as_ref()
            .ok_or(MissionError::NoCommander)?;
        let dock = self.current_dock_context();
        let station_actor = dock.map(|context| ActorId {
            entity: context.actor_entity(),
            generation: context.visit(),
        });
        let contact = session
            .contact
            .filter(|npc| dock == Some(npc.dock) && session.roster == Some(*npc));
        Ok(WorldFacts {
            epoch: session.epoch,
            paused: session.paused,
            content_digest: Some(g02::CONTENT_DIGEST),
            dock: dock
                .zip(station_actor)
                .map(|(context, station_actor)| DockFacts {
                    owner: context.owner().seed(),
                    hull: context.ship().ordinal(),
                    system: g02::System {
                        galaxy: context.system().galaxy,
                        system: context.system().system,
                    },
                    kind: StationKind::MainOrbital,
                    station_actor,
                    arrival_sequence: context.sequence(),
                }),
            contact: contact
                .zip(station_actor)
                .map(|(npc, station_actor)| ContactFacts {
                    actor: npc.actor,
                    station_actor,
                    definition: npc.definition,
                    roster_revision: npc.roster_revision,
                }),
        })
    }

    /// Executes a confirmed domain command only against this World's actual current facts.
    /// This synchronous transaction publishes memory only; no disk persistence is claimed.
    pub fn request_freight(
        &mut self,
        command: g02::Command,
        expected_tick: u64,
    ) -> Result<g02::Event, MissionError> {
        self.station_command_admission(expected_tick)?;
        if matches!(
            command,
            g02::Command::Confirm {
                choice: g02::Choice::Accept,
                ..
            }
        ) && !self.original_service_has_capacity()
        {
            return Err(MissionError::SharedCapacity);
        }
        if matches!(
            command,
            g02::Command::Confirm {
                choice: g02::Choice::Accept,
                ..
            }
        ) {
            let p = self
                .reserved_service_projection()
                .map_err(MissionError::Service)?;
            if u32::from(p.used_tonnes) + 8 > u32::from(p.hold_capacity_tonnes) {
                return Err(MissionError::Freight(g02::Error::HoldFull));
            }
        }
        let facts = self.freight_facts()?;
        let cargo_dock = self.current_dock_context().ok_or(MissionError::NotDocked)?;
        let account = self
            .commander_account
            .as_mut()
            .ok_or(MissionError::NoCommander)?;
        let session = self
            .freight_session
            .as_mut()
            .ok_or(MissionError::NoCommander)?;
        let plan = session
            .state
            .prepare(account, &facts, command)
            .map_err(MissionError::Freight)?;
        let sequence = session.state.cargo_sequence();
        let event = session
            .state
            .commit(account, &facts, plan)
            .map_err(MissionError::Freight)?;
        if session.state.cargo_sequence() != sequence {
            self.modular.record_freight(
                account.revision(),
                session.state.cargo_sequence(),
                cargo_dock,
            );
        }
        Ok(event)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::freight::g02::{Choice, Command, Error, Event, GoodsId, JobId, Review};
    use crate::navigation::{BASIC_SPOOL_TICKS, DockOrigin};
    use crate::{FlightInput, FlightMode, StepError};

    fn command(world: &mut World, command: Command) -> Result<Event, MissionError> {
        world.request_freight(command, world.snapshot().tick)
    }
    fn reviewed(world: &mut World, command_value: Command) -> Review {
        match command(world, command_value).unwrap() {
            Event::Reviewed(review) => review,
            event => panic!("expected review, got {event:?}"),
        }
    }
    fn accept(world: &mut World) -> (JobId, [GoodsId; 8]) {
        let npc = world.station_contacts()[0];
        world.request_contact(npc, world.snapshot().tick).unwrap();
        let review = reviewed(world, Command::ReviewOffer { actor: npc.actor() });
        match command(
            world,
            Command::Confirm {
                review,
                choice: Choice::Accept,
            },
        )
        .unwrap()
        {
            Event::Accepted { job, units } => (job, units),
            event => panic!("expected allocation, got {event:?}"),
        }
    }
    fn fingerprint(
        world: &World,
    ) -> (
        crate::Snapshot,
        Option<MissionSnapshot>,
        Option<u32>,
        Vec<StationNpc>,
    ) {
        (
            world.snapshot(),
            world.mission_snapshot(),
            world.credits_tenths(),
            world.station_contacts(),
        )
    }
    fn refuses(world: &mut World, value: Command, error: Error) {
        let before = fingerprint(world);
        assert_eq!(command(world, value), Err(MissionError::Freight(error)));
        assert_eq!(fingerprint(world), before);
    }
    fn travel_to_receiving_station(world: &mut World) {
        let old = world.current_dock_context().unwrap();
        world.request_launch(world.snapshot().tick).unwrap();
        assert!(world.station_contacts().is_empty());
        assert!(!world.dock_context_is_current(&old));
        world.step(FlightInput::default()).unwrap();
        world
            .request_jump(SystemId::RENAS, world.snapshot().tick)
            .unwrap();
        for _ in 1..BASIC_SPOOL_TICKS {
            world.step(FlightInput::default()).unwrap();
        }
        assert_eq!(world.snapshot().navigation.unwrap().fuel_units, 130);
        assert_eq!(
            world
                .snapshot()
                .navigation
                .unwrap()
                .travel
                .unwrap()
                .remaining_ticks,
            1
        );
        world.step(FlightInput::default()).unwrap();
        assert_eq!(world.snapshot().position, [0.0, 0.0, 16000.0]);
        assert_eq!(world.snapshot().navigation.unwrap().fuel_units, 96);
        assert_eq!(world.snapshot().mode, FlightMode::Flight);
        assert!(world.current_dock_context().is_none());
        assert!(world.station_contacts().is_empty());
        assert_eq!(
            world.request_refuel(world.snapshot().tick),
            Err(MissionError::NotDocked)
        );
        // Same public control approach as the independent navigation test: no pose injection.
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
                .unwrap();
            assert_eq!(world.snapshot().motion_refusal, None);
        }
        assert_eq!(world.snapshot().mode, FlightMode::Docked);
        let dock = world.current_dock_context().unwrap();
        assert_eq!(dock.origin(), DockOrigin::PhysicalEntry);
        assert_eq!(dock.system(), SystemId::RENAS);
        assert_eq!((dock.owner(), dock.ship()), (old.owner(), old.ship()));
        assert_eq!(world.snapshot().navigation.unwrap().fuel_units, 96);
    }

    #[test]
    fn real_world_source_consent_jump_physical_entry_and_eighth_unit_settle_once() {
        let mut world = World::new_commander(501).unwrap();
        let source = world.station_contacts()[0];
        assert_eq!(source.definition(), ContactDefinition::Arlen);
        assert_eq!(source.name(), "Arlen");
        assert_eq!(source.dock().origin(), DockOrigin::InitialStart);
        refuses(
            &mut world,
            Command::ReviewOffer {
                actor: source.actor(),
            },
            Error::WrongActor,
        );
        let (job, units) = accept(&mut world);
        assert_eq!(world.credits_tenths(), Some(7200));
        assert_eq!(world.mission_snapshot().unwrap().freight.held_tonnes, 8);
        refuses(
            &mut world,
            Command::ReviewReceiving {
                actor: source.actor(),
                job,
            },
            Error::WrongStation,
        );
        travel_to_receiving_station(&mut world);
        assert_eq!(
            world.request_contact(source, world.snapshot().tick),
            Err(MissionError::NotDocked)
        );
        let crew = world.station_contacts()[0];
        assert_eq!(crew.definition(), ContactDefinition::RenasUtilityCrew);
        assert_ne!(crew.actor(), source.actor());
        refuses(
            &mut world,
            Command::ReviewReceiving {
                actor: crew.actor(),
                job,
            },
            Error::WrongActor,
        );
        world.request_contact(crew, world.snapshot().tick).unwrap();
        refuses(
            &mut world,
            Command::ReviewOffer {
                actor: source.actor(),
            },
            Error::WrongStation,
        );
        let (_, foreign) = accept(&mut World::new_commander(901).unwrap());
        let review = reviewed(
            &mut world,
            Command::ReviewReceiving {
                actor: crew.actor(),
                job,
            },
        );
        refuses(
            &mut world,
            Command::Confirm {
                review,
                choice: Choice::ReceiveOne(foreign[0]),
            },
            Error::ForeignUnit,
        );
        refuses(
            &mut world,
            Command::Confirm {
                review,
                choice: Choice::ReceiveOne(units[1]),
            },
            Error::OutOfOrder,
        );
        let mut last_review = review;
        for (index, unit) in units.iter().enumerate() {
            let review = if index == 0 {
                review
            } else {
                reviewed(
                    &mut world,
                    Command::ReviewReceiving {
                        actor: crew.actor(),
                        job,
                    },
                )
            };
            last_review = review;
            assert_eq!(review.next_unit(), Some(*unit));
            let result = command(
                &mut world,
                Command::Confirm {
                    review,
                    choice: Choice::ReceiveOne(*unit),
                },
            )
            .unwrap();
            let Event::Received {
                units: received,
                remaining,
                completion,
                clue_awarded,
                ..
            } = result
            else {
                panic!("expected custody transfer");
            };
            assert_eq!(
                received.iter().flatten().copied().collect::<Vec<_>>(),
                vec![*unit]
            );
            assert_eq!(remaining, 7 - index as u8);
            assert_eq!(completion.is_some(), index == 7);
            assert_eq!(clue_awarded, index == 7);
            assert_eq!(
                world.credits_tenths(),
                Some(if index == 7 { 14500 } else { 7200 })
            );
            assert_eq!(
                world.mission_snapshot().unwrap().freight.clue.is_some(),
                index == 7
            );
            if index == 0 {
                let next = reviewed(
                    &mut world,
                    Command::ReviewReceiving {
                        actor: crew.actor(),
                        job,
                    },
                );
                refuses(
                    &mut world,
                    Command::Confirm {
                        review: next,
                        choice: Choice::ReceiveOne(*unit),
                    },
                    Error::AlreadyTransferred,
                );
            }
        }
        let done = world.mission_snapshot().unwrap();
        assert_eq!(done.freight.held_tonnes, 0);
        assert_eq!(done.freight.available_source_units, 8);
        assert_eq!(done.freight.completed_jobs, 1);
        assert_eq!(done.freight.clue.unwrap().completion().job(), job);
        refuses(
            &mut world,
            Command::Confirm {
                review: last_review,
                choice: Choice::ReceiveAll,
            },
            Error::StaleReview,
        );
        refuses(
            &mut world,
            Command::ReviewReceiving {
                actor: crew.actor(),
                job,
            },
            Error::NotActive,
        );
        assert_eq!(world.credits_tenths(), Some(14500));
    }

    #[test]
    fn renas_mule_is_the_same_actor_on_the_first_completed_freight_sortie_after_continue() {
        let mut world = World::new_commander(501).unwrap();
        let (job, units) = accept(&mut world);
        travel_to_receiving_station(&mut world);
        let receiver = world.station_contacts()[0];
        world
            .request_contact(receiver, world.snapshot().tick)
            .unwrap();
        for (index, unit) in units.into_iter().enumerate() {
            let review = reviewed(
                &mut world,
                Command::ReviewReceiving {
                    actor: receiver.actor(),
                    job,
                },
            );
            assert_eq!(review.next_unit(), Some(unit));
            assert!(matches!(
                command(
                    &mut world,
                    Command::Confirm {
                        review,
                        choice: Choice::ReceiveOne(unit),
                    }
                ),
                Ok(Event::Received { .. })
            ));
            if index == 6 {
                let mut seven = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
                seven.request_launch(seven.snapshot().tick).unwrap();
                assert!(seven.mule_actor_snapshot().is_none());
                assert_eq!(seven.credits_tenths(), Some(7200));
                assert!(seven.mission_snapshot().unwrap().freight.clue.is_none());
            }
        }

        let dock = world.current_dock_context().unwrap();
        let completed = world.mission_snapshot().unwrap().freight;
        let clue = completed.clue.unwrap();
        assert_eq!(clue.completion().job(), job);
        assert_eq!(completed.completed_jobs, 1);
        assert_eq!(world.credits_tenths(), Some(14500));
        assert!(world.mule_actor_snapshot().is_none());
        let docked_bytes = world.save_bytes().unwrap();
        assert_eq!(
            World::decode_saved_world(&docked_bytes)
                .unwrap()
                .save_bytes()
                .unwrap(),
            docked_bytes
        );
        let mut cold = World::from_save_bytes(&docked_bytes).unwrap();
        assert_eq!(cold.current_dock_context(), Some(dock));
        assert!(cold.mule_actor_snapshot().is_none());
        assert_eq!(
            cold.request_launch(cold.snapshot().tick + 1),
            Err(StepError::StaleCommand)
        );
        assert!(cold.mule_actor_snapshot().is_none());
        assert_eq!(cold.current_dock_context(), Some(dock));

        cold.request_launch(cold.snapshot().tick).unwrap();
        let first = cold.mule_actor_snapshot().unwrap();
        assert_eq!(first.actor().id.system(), SystemId::RENAS);
        assert_eq!(first.actor().id.profile(), dock.owner().seed());
        assert_eq!(first.actor().hull_id, "SH001");
        assert_eq!(first.generation(), dock.visit() + 1);
        assert_eq!(first.crew_aboard(), Some(2));
        assert!(first.is_renas_encounter());
        assert!(cold.mule_snapshot_is_current(&first));
        assert!(cold.mule_observer_pose(&first).is_none());
        assert_eq!(
            cold.request_launch(cold.snapshot().tick),
            Err(StepError::InvalidMode)
        );
        assert!(cold.mule_snapshot_is_current(&first));

        let flight_bytes = cold.save_bytes().unwrap();
        assert_eq!(
            World::decode_saved_world(&flight_bytes)
                .unwrap()
                .save_bytes()
                .unwrap(),
            flight_bytes
        );
        let mut resumed = World::from_save_bytes(&flight_bytes).unwrap();
        let restored = resumed.mule_actor_snapshot().unwrap();
        assert_eq!(restored.actor(), first.actor());
        assert_eq!(restored.presentation(), first.presentation());
        assert_eq!(restored.generation(), first.generation());
        assert!(resumed.mule_snapshot_is_current(&restored));
        assert!(!resumed.mule_snapshot_is_current(&first));
        assert_eq!(resumed.credits_tenths(), Some(14500));
        assert_eq!(resumed.mission_snapshot().unwrap().freight.clue, Some(clue));

        return_to_renas_berth(&mut resumed);
        assert_ne!(resumed.current_dock_context(), Some(dock));
        assert!(resumed.mule_actor_snapshot().is_none());
        let mut returned = World::from_save_bytes(&resumed.save_bytes().unwrap()).unwrap();
        assert!(returned.mule_actor_snapshot().is_none());
        returned.request_launch(returned.snapshot().tick).unwrap();
        assert!(returned.mule_actor_snapshot().is_none());
        assert_eq!(returned.credits_tenths(), Some(14500));
        assert_eq!(
            returned.mission_snapshot().unwrap().freight.clue,
            Some(clue)
        );
        assert!(
            World::from_save_bytes(&returned.save_bytes().unwrap())
                .unwrap()
                .mule_actor_snapshot()
                .is_none()
        );
    }

    fn return_to_renas_berth(world: &mut World) {
        use super::super::{FIXED_DELTA_SECONDS, YAW_RATE};
        for _ in 0..80 {
            if world.snapshot().speed == 0.0 {
                break;
            }
            world
                .step(FlightInput {
                    brake: true,
                    ..FlightInput::default()
                })
                .unwrap();
        }
        assert_eq!(world.snapshot().speed, 0.0);
        let yaw_ticks = std::f32::consts::PI / (YAW_RATE * FIXED_DELTA_SECONDS);
        for _ in 0..yaw_ticks.floor() as u32 {
            world
                .step(FlightInput {
                    yaw: 1.0,
                    ..FlightInput::default()
                })
                .unwrap();
        }
        world
            .step(FlightInput {
                yaw: yaw_ticks.fract(),
                ..FlightInput::default()
            })
            .unwrap();
        for _ in 0..2600 {
            if world.snapshot().mode == FlightMode::Docked {
                break;
            }
            let speed = world.snapshot().speed;
            world
                .step(FlightInput {
                    throttle: ((4.0 - speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
                    ..FlightInput::default()
                })
                .unwrap();
            assert_eq!(world.snapshot().motion_refusal, None);
        }
        assert_eq!(
            world.current_dock_context().unwrap().system(),
            SystemId::RENAS
        );
    }

    #[test]
    fn public_contact_back_pause_tick_and_owner_changes_do_not_manufacture_consent() {
        let mut world = World::new_commander(51).unwrap();
        let npc = world.station_contacts()[0];
        let foreign_npc = World::new_commander(71).unwrap().station_contacts()[0];
        let before = fingerprint(&world);
        assert_eq!(
            world.request_contact(foreign_npc, 0),
            Err(MissionError::NotDocked)
        );
        assert_eq!(fingerprint(&world), before);
        world.request_contact(npc, 0).unwrap();
        let offer = Command::ReviewOffer { actor: npc.actor() };
        let review = reviewed(&mut world, offer);
        // Same first review nonce, purpose and NPC slot in a different real profile.
        let mut foreign = World::new_commander(71).unwrap();
        let foreign_contact = foreign.station_contacts()[0];
        foreign.request_contact(foreign_contact, 0).unwrap();
        let foreign_review = reviewed(
            &mut foreign,
            Command::ReviewOffer {
                actor: foreign_contact.actor(),
            },
        );
        refuses(
            &mut world,
            Command::Confirm {
                review: foreign_review,
                choice: Choice::Accept,
            },
            Error::StaleReview,
        );
        command(&mut world, Command::Back).unwrap();
        refuses(
            &mut world,
            Command::Confirm {
                review,
                choice: Choice::Accept,
            },
            Error::StaleReview,
        );
        let review = reviewed(&mut world, offer);
        world.request_contact(npc, 0).unwrap();
        refuses(
            &mut world,
            Command::Confirm {
                review,
                choice: Choice::Accept,
            },
            Error::StaleReview,
        );
        let review = reviewed(&mut world, offer);
        world.request_pause(true, 0).unwrap();
        let paused = fingerprint(&world);
        assert_eq!(command(&mut world, offer), Err(MissionError::Paused));
        assert_eq!(world.request_contact(npc, 0), Err(MissionError::Paused));
        assert_eq!(world.request_launch(0), Err(StepError::Paused));
        assert_eq!(world.step(FlightInput::default()), Err(StepError::Paused));
        assert_eq!(fingerprint(&world), paused);
        world.request_pause(false, 0).unwrap();
        refuses(
            &mut world,
            Command::Confirm {
                review,
                choice: Choice::Accept,
            },
            Error::StaleReview,
        );
        world.step(FlightInput::default()).unwrap();
        let before = fingerprint(&world);
        assert_eq!(
            world.request_freight(offer, 0),
            Err(MissionError::StaleCommand)
        );
        assert_eq!(fingerprint(&world), before);
        for mut prototype in [World::new(), World::at_original_station().unwrap()] {
            assert!(prototype.station_contacts().is_empty());
            assert_eq!(
                prototype.request_freight(Command::Back, 0),
                Err(MissionError::NoCommander)
            );
        }
    }

    #[test]
    fn real_station_fuel_purchase_updates_same_account_and_invalidates_receiving_review() {
        let mut world = World::new_commander(101).unwrap();
        assert_eq!(world.request_refuel(0), Ok(0));
        let (job, _) = accept(&mut world);
        travel_to_receiving_station(&mut world);
        let crew = world.station_contacts()[0];
        world.request_contact(crew, world.snapshot().tick).unwrap();
        let review = reviewed(
            &mut world,
            Command::ReviewReceiving {
                actor: crew.actor(),
                job,
            },
        );
        assert_eq!(world.request_refuel(world.snapshot().tick), Ok(68));
        assert_eq!(world.credits_tenths(), Some(7132));
        assert_eq!(world.snapshot().navigation.unwrap().fuel_units, 130);
        refuses(
            &mut world,
            Command::Confirm {
                review,
                choice: Choice::ReceiveAll,
            },
            Error::StaleReview,
        );
        let review = reviewed(
            &mut world,
            Command::ReviewReceiving {
                actor: crew.actor(),
                job,
            },
        );
        command(
            &mut world,
            Command::Confirm {
                review,
                choice: Choice::ReceiveAll,
            },
        )
        .unwrap();
        assert_eq!(world.credits_tenths(), Some(14432));
    }
}
