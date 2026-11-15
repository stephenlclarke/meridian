//! Read-only presentation of the original G02 journey, never command authority.
//!
//! Original dialogue is from immutable L337 a11fe169 (MIT); retain
//! assets/legacy/Elite-L337-MIT.txt. See docs/G02-PRESENTATION-SOURCES.json.
//! New Rust projection code follows this repository's LICENSE.

use super::{Choice, Purpose, Review};
use crate::World;

pub const DESTINATION: &str = "Renas main orbital / utility receiving crew";
pub const OFFER_BRIEFING: &str = "ARLEN: Reserve eight original Machinery utility-parts tonnes at Avenor, from one finite sixteen-tonne batch. Deliver those same units to the consenting utility receiving crew at Renas. Frozen reward 730 CR / L1 for the actual native route; H0, no deposit, no berths, no deadline. No restock after loss, abandonment, update or reinstall. Back changes nothing.";
pub const ACCEPTED_JOURNAL: &str = "ARLEN: The eight original utility-parts tonnes are aboard. Source stock is allocated once. Deliver these same units to the consenting Renas utility receiving crew. No deadline.";
pub const RECEIVING_DIALOGUE: &str = "Renas utility receiving crew: we consent to receive these same identified utility parts. Confirm one original tonne or all remaining units. Seven tonnes do not settle the job; the eighth commits the frozen payment and first family clue. Back transfers nothing.";
pub const ABANDONMENT_DIALOGUE: &str = "ARLEN: Confirm abandonment for zero payment. These allocated original units never return to available supplier stock. Neither a quote nor a lost crate creates replacement stock. Back changes nothing.";
pub const IDENTITY_NOTICE: &str =
    "Losses and abandonment never replenish stock; ordinary Machinery cannot replace these units.";
pub const CLUE_NOTICE: &str = "The family clue is awarded on the first completed job only.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewKind {
    Acceptance,
    ReceiveOne,
    ReceiveAll,
    Abandonment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogueAction {
    Confirm(Choice),
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogueChoice {
    pub label: &'static str,
    pub action: DialogueAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferProgress {
    pub original_tonnes_remaining: u8,
    pub original_tonnes_on_confirmation: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewPresentation {
    pub kind: ReviewKind,
    pub speaker: &'static str,
    pub title: &'static str,
    pub body: &'static str,
    pub notice: &'static str,
    pub progress: Option<TransferProgress>,
    pub choices: [Option<DialogueChoice>; 3],
}

const BACK: Option<DialogueChoice> = Some(DialogueChoice {
    label: "Back",
    action: DialogueAction::Back,
});

fn confirmation(label: &'static str, choice: Choice) -> DialogueChoice {
    DialogueChoice {
        label,
        action: DialogueAction::Confirm(choice),
    }
}

/// Match the actual typed purpose and explicit selected choice, not a missing next unit.
/// `Some` means a descriptive match only: World must still validate the live review,
/// current contact, account and admission when Confirm/Back is actually commanded.
/// Merely rendering a stale or foreign review cannot make it current or grant consent.
pub fn review(review: Review, selected: Choice) -> Option<ReviewPresentation> {
    match (review.purpose, selected) {
        (Purpose::Accept, Choice::Accept) => Some(ReviewPresentation {
            kind: ReviewKind::Acceptance,
            speaker: "ARLEN",
            title: "Eight original utility-parts tonnes / H0 / untimed",
            body: OFFER_BRIEFING,
            notice: IDENTITY_NOTICE,
            progress: None,
            choices: [
                Some(confirmation("Accept eight original tonnes", Choice::Accept)),
                BACK,
                None,
            ],
        }),
        (Purpose::Receive(_), Choice::ReceiveOne(unit)) if review.next_unit() == Some(unit) => {
            receiving(review, ReviewKind::ReceiveOne, 1)
        }
        (Purpose::Receive(_), Choice::ReceiveAll) => {
            receiving(review, ReviewKind::ReceiveAll, review.remaining_units())
        }
        (Purpose::Abandon(_), Choice::Abandon) => Some(ReviewPresentation {
            kind: ReviewKind::Abandonment,
            speaker: "ARLEN",
            title: "Abandon this freight job / zero payment",
            body: ABANDONMENT_DIALOGUE,
            notice: IDENTITY_NOTICE,
            progress: None,
            choices: [
                Some(confirmation("Abandon for zero payment", Choice::Abandon)),
                BACK,
                None,
            ],
        }),
        _ => None,
    }
}

fn receiving(review: Review, kind: ReviewKind, transfer: u8) -> Option<ReviewPresentation> {
    let next = review.next_unit()?;
    let remaining = review.remaining_units();
    if !(1..=8).contains(&remaining) || transfer == 0 || transfer > remaining {
        return None;
    }
    Some(ReviewPresentation {
        kind,
        speaker: "Renas utility receiving crew",
        title: match kind {
            ReviewKind::ReceiveAll => "Receive all remaining original tonnes",
            _ => "Receive one original tonne",
        },
        body: RECEIVING_DIALOGUE,
        notice: CLUE_NOTICE,
        progress: Some(TransferProgress {
            original_tonnes_remaining: remaining,
            original_tonnes_on_confirmation: transfer,
        }),
        choices: [
            Some(confirmation(
                "Receive one original tonne",
                Choice::ReceiveOne(next),
            )),
            Some(confirmation(
                "Receive all remaining original tonnes",
                Choice::ReceiveAll,
            )),
            BACK,
        ],
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveKind {
    CollectOriginals,
    DeliverOriginals,
    Completed,
    SupplierExhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectivePresentation {
    pub kind: ObjectiveKind,
    pub description: &'static str,
    pub held_original_tonnes: u8,
    pub available_source_units: u8,
    pub completed_jobs: u8,
    pub trusted_clue_recorded: bool,
}

/// Read the actual owning World; aggregate counters never mint a completion or clue.
/// A later active allocation remains a delivery even if the first family clue is retained.
pub fn objective(world: &World) -> Option<ObjectivePresentation> {
    let freight = world.mission_snapshot()?.freight;
    let (kind, description) = if world.active_freight_job().is_some() {
        (
            ObjectiveKind::DeliverOriginals,
            "Deliver the same original tonnes to the consenting Renas utility receiving crew.",
        )
    } else if freight.clue.is_some() {
        (
            ObjectiveKind::Completed,
            "G02 completed; the first trusted family clue is recorded. Each completed job settles once.",
        )
    } else if freight.available_source_units >= 8 {
        (
            ObjectiveKind::CollectOriginals,
            "Contact ARLEN at Avenor main orbital to review the original utility-parts allocation.",
        )
    } else {
        (
            ObjectiveKind::SupplierExhausted,
            "No unallocated original units remain in this finite batch. Abandoned units are not restocked.",
        )
    };
    Some(ObjectivePresentation {
        kind,
        description,
        held_original_tonnes: freight.held_tonnes,
        available_source_units: freight.available_source_units,
        completed_jobs: freight.completed_jobs,
        trusted_clue_recorded: freight.clue.is_some(),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::freight::g02::{ActorId, Command, Error, Event, GoodsId, JobId};
    use crate::navigation::{BASIC_SPOOL_TICKS, DockOrigin, SystemId};
    use crate::{FlightInput, FlightMode, MissionError};

    fn command(world: &mut World, command: Command) -> Result<Event, MissionError> {
        world.request_freight(command, world.snapshot().tick)
    }

    fn issued_review(world: &mut World, command_value: Command) -> Review {
        let Event::Reviewed(review) = command(world, command_value).unwrap() else {
            panic!("expected a World-issued review");
        };
        review
    }

    fn select_contact(world: &mut World) -> ActorId {
        let npc = world.station_contacts()[0];
        world.request_contact(npc, world.snapshot().tick).unwrap();
        npc.actor()
    }

    fn accept(world: &mut World) -> JobId {
        accept_with_units(world).0
    }

    fn accept_with_units(world: &mut World) -> (JobId, [GoodsId; 8]) {
        let actor = select_contact(world);
        let token = issued_review(world, Command::ReviewOffer { actor });
        let Event::Accepted { job, units } = command(
            world,
            Command::Confirm {
                review: token,
                choice: Choice::Accept,
            },
        )
        .unwrap() else {
            panic!("expected actual original allocation");
        };
        (job, units)
    }

    fn fly_to_receiving_crew(world: &mut World) -> ActorId {
        world.request_launch(world.snapshot().tick).unwrap();
        world.step(FlightInput::default()).unwrap();
        world
            .request_jump(SystemId::RENAS, world.snapshot().tick)
            .unwrap();
        for _ in 0..BASIC_SPOOL_TICKS {
            world.step(FlightInput::default()).unwrap();
        }
        for _ in 0..9000 {
            let snapshot = world.snapshot();
            if snapshot.mode == FlightMode::Docked {
                break;
            }
            let speed = if snapshot.position[2] > 2500.0 {
                34.3
            } else {
                4.0
            };
            world
                .step(FlightInput {
                    throttle: ((speed - snapshot.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
                    ..FlightInput::default()
                })
                .unwrap();
        }
        assert_eq!(world.snapshot().mode, FlightMode::Docked);
        assert_eq!(
            world.current_dock_context().unwrap().origin(),
            DockOrigin::PhysicalEntry
        );
        assert_eq!(
            world.current_dock_context().unwrap().system(),
            SystemId::RENAS
        );
        select_contact(world)
    }

    #[test]
    fn fresh_offer_and_back_project_terms_without_allocating_or_granting_authority() {
        let mut world = World::new_commander(501).unwrap();
        let goal = objective(&world).unwrap();
        assert_eq!(goal.kind, ObjectiveKind::CollectOriginals);
        assert_eq!(
            (goal.available_source_units, goal.held_original_tonnes),
            (16, 0)
        );
        let actor = select_contact(&mut world);
        let token = issued_review(&mut world, Command::ReviewOffer { actor });
        assert_eq!(token.next_unit(), None);
        let bytes = world.save_bytes().unwrap();
        let view = review(token, Choice::Accept).unwrap();
        assert_eq!(view.kind, ReviewKind::Acceptance);
        assert_eq!(view.progress, None);
        assert_eq!(view.choices[1], BACK);
        assert_eq!(review(token, Choice::Abandon), None);
        assert_eq!(world.save_bytes().unwrap(), bytes);
        command(&mut world, Command::Back).unwrap();
        // Formatting a now-stale review cannot restore the live World permission.
        assert_eq!(
            review(token, Choice::Accept).unwrap().kind,
            ReviewKind::Acceptance
        );
        let before = world.save_bytes().unwrap();
        assert_eq!(
            command(
                &mut world,
                Command::Confirm {
                    review: token,
                    choice: Choice::Accept,
                }
            ),
            Err(MissionError::Freight(Error::StaleReview))
        );
        assert_eq!(world.save_bytes().unwrap(), before);
        assert_eq!(objective(&world).unwrap().available_source_units, 16);
    }

    #[test]
    fn abandonment_purpose_preserves_original_identity_without_restock() {
        let mut world = World::new_commander(502).unwrap();
        for remaining in [8, 0] {
            let (job, units) = accept_with_units(&mut world);
            let actor = select_contact(&mut world);
            let token = issued_review(&mut world, Command::ReviewAbandonment { actor, job });
            assert_eq!(token.next_unit(), Some(units[0]));
            assert_eq!(review(token, Choice::Accept), None);
            assert_eq!(review(token, Choice::ReceiveOne(units[0])), None);
            assert_eq!(review(token, Choice::ReceiveAll), None);
            let before = world.save_bytes().unwrap();
            let view = review(token, Choice::Abandon).unwrap();
            assert_eq!(view.kind, ReviewKind::Abandonment);
            assert_eq!(view.choices[1], BACK);
            assert_eq!(world.save_bytes().unwrap(), before);
            command(
                &mut world,
                Command::Confirm {
                    review: token,
                    choice: Choice::Abandon,
                },
            )
            .unwrap();
            let goal = objective(&world).unwrap();
            assert_eq!(goal.available_source_units, remaining);
            assert_eq!(goal.held_original_tonnes, 0);
            assert_eq!(goal.completed_jobs, 0);
            assert!(!goal.trusted_clue_recorded);
            assert_eq!(world.credits_tenths(), Some(7200));
        }
        assert_eq!(
            objective(&world).unwrap().kind,
            ObjectiveKind::SupplierExhausted
        );
    }

    #[test]
    fn actual_receiving_choices_and_seventh_eighth_cold_objectives_stay_distinct() {
        let mut world = World::new_commander(503).unwrap();
        let job = accept(&mut world);
        let actor = fly_to_receiving_crew(&mut world);
        let mut last = None;
        for handed in 0..7 {
            let token = issued_review(&mut world, Command::ReviewReceiving { actor, job });
            let unit = token.next_unit().unwrap();
            let before = world.save_bytes().unwrap();
            let one = review(token, Choice::ReceiveOne(unit)).unwrap();
            let all = review(token, Choice::ReceiveAll).unwrap();
            assert_eq!(
                (one.kind, all.kind),
                (ReviewKind::ReceiveOne, ReviewKind::ReceiveAll)
            );
            assert_eq!(one.progress.unwrap().original_tonnes_on_confirmation, 1);
            assert_eq!(
                all.progress.unwrap().original_tonnes_on_confirmation,
                8 - handed
            );
            assert_eq!(one.choices, all.choices);
            assert_eq!(one.choices[2], BACK);
            assert_eq!(review(token, Choice::Accept), None);
            assert_eq!(world.save_bytes().unwrap(), before);
            let result = command(
                &mut world,
                Command::Confirm {
                    review: token,
                    choice: Choice::ReceiveOne(unit),
                },
            )
            .unwrap();
            assert!(matches!(
                result,
                Event::Received {
                    completion: None,
                    clue_awarded: false,
                    ..
                }
            ));
            assert_eq!(world.credits_tenths(), Some(7200));
            let goal = objective(&world).unwrap();
            assert_eq!(goal.kind, ObjectiveKind::DeliverOriginals);
            assert_eq!(goal.held_original_tonnes, 7 - handed);
            assert!(!goal.trusted_clue_recorded);
            last = Some(token);
        }
        let bytes = world.save_bytes().unwrap();
        let mut cold = World::from_save_bytes(&bytes).unwrap();
        assert_eq!(objective(&cold), objective(&world));
        assert_eq!(cold.active_freight_job(), Some(job));
        assert_eq!(
            command(
                &mut cold,
                Command::Confirm {
                    review: last.unwrap(),
                    choice: Choice::ReceiveAll,
                }
            ),
            Err(MissionError::Freight(Error::StaleReview))
        );
        let crew = select_contact(&mut cold);
        let token = issued_review(&mut cold, Command::ReviewReceiving { actor: crew, job });
        assert_eq!(
            review(token, Choice::ReceiveAll)
                .unwrap()
                .progress
                .unwrap()
                .original_tonnes_remaining,
            1
        );
        let result = command(
            &mut cold,
            Command::Confirm {
                review: token,
                choice: Choice::ReceiveAll,
            },
        )
        .unwrap();
        assert!(matches!(
            result,
            Event::Received {
                completion: Some(_),
                clue_awarded: true,
                remaining: 0,
                ..
            }
        ));
        assert_eq!(cold.credits_tenths(), Some(14500));
        let goal = objective(&cold).unwrap();
        assert_eq!(goal.kind, ObjectiveKind::Completed);
        assert_eq!(goal.completed_jobs, 1);
        assert!(goal.trusted_clue_recorded);
        let completed = World::from_save_bytes(&cold.save_bytes().unwrap()).unwrap();
        assert_eq!(objective(&completed), Some(goal));
        let before = cold.save_bytes().unwrap();
        assert_eq!(
            command(
                &mut cold,
                Command::Confirm {
                    review: token,
                    choice: Choice::ReceiveAll
                }
            ),
            Err(MissionError::Freight(Error::StaleReview))
        );
        assert_eq!(cold.save_bytes().unwrap(), before);
    }

    #[test]
    fn prototype_worlds_have_no_commander_freight_objective() {
        assert_eq!(objective(&World::new()), None);
        assert_eq!(objective(&World::at_original_station().unwrap()), None);
    }
}
