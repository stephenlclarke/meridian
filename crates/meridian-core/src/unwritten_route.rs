//! Read-only projection of the commander's existing earned Unwritten Route clues.

mod catalogue;

use crate::World;
use catalogue::BASE_CLUES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseClue {
    pub source_id: &'static str,
    pub clue_id: &'static str,
}

impl BaseClue {
    const fn new(source_id: &'static str, clue_id: &'static str) -> Self {
        Self { source_id, clue_id }
    }

    fn from_receipt(source: &'static str, clue: &'static str, receipt: &str) -> Option<Self> {
        (receipt.strip_prefix("l337.unwritten-route/clue/") == Some(clue))
            .then_some(Self::new(source, clue))
    }
}

/// Original design requirements and the first clues published by their owning reducers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClueProgress {
    pub earned: Vec<BaseClue>,
    pub missing: Vec<BaseClue>,
}

impl ClueProgress {
    pub const REQUIRED: usize = BASE_CLUES.len();

    pub fn complete(&self) -> bool {
        self.missing.is_empty()
    }
}

impl World {
    pub fn unwritten_route_clue_progress(&self) -> Option<ClueProgress> {
        let owner = self.commander_id()?.seed();
        let hull = self.player_ship_id()?.ordinal();
        let receipts = [
            self.dispatch_records()
                .iter()
                .find(|r| {
                    r.issuer.owner().seed() == owner
                        && r.issuer.ship().ordinal() == hull
                        && matches!(
                            r.terminal,
                            Some((crate::freight::DispatchAction::Receive, _))
                        )
                })
                .map(|_| {
                    BaseClue::new(
                        "l337.contract/sealed-dispatch",
                        "sealed-dispatch-family-trace",
                    )
                }),
            self.mission_snapshot()
                .and_then(|snapshot| snapshot.freight.clue)
                .map(|award| BaseClue::new(award.source_id(), award.clue_id())),
            self.flight_reception_snapshot()
                .and_then(|snapshot| snapshot.trace)
                .and_then(|trace| {
                    BaseClue::from_receipt(trace.source_id, trace.clue_id, trace.receipt_id)
                }),
            self.practice_ledger_snapshot()
                .and_then(|snapshot| snapshot.trace)
                .and_then(|trace| {
                    BaseClue::from_receipt(trace.source_id, trace.clue_id, trace.receipt_id)
                }),
            self.navigation_trade_snapshot()
                .and_then(|snapshot| snapshot.trace)
                .and_then(|trace| {
                    BaseClue::from_receipt(trace.source_id, trace.clue_id, trace.receipt_id)
                }),
            self.passenger_journey_snapshot()
                .and_then(|snapshot| snapshot.clue)
                .filter(|receipt| receipt.owner == owner && receipt.hull == hull)
                .and_then(|receipt| {
                    BaseClue::from_receipt(
                        "l337.contract/cabin-journey",
                        "cabin-journey-family-trace",
                        receipt.clue_id()?,
                    )
                }),
        ];
        // Only the owning reducers can mark a source pair as earned.
        let (mut earned, mut missing): (Vec<_>, Vec<_>) = BASE_CLUES
            .into_iter()
            .partition(|clue| receipts.contains(&Some(*clue)));
        earned.sort_by_key(|clue| clue.clue_id);
        missing.sort_by_key(|clue| clue.clue_id);
        Some(ClueProgress { earned, missing })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn original_catalogue_has_64_distinct_sources_and_clues() {
        for ids in [
            BASE_CLUES.map(|clue| clue.source_id),
            BASE_CLUES.map(|clue| clue.clue_id),
        ] {
            assert_eq!(ids.into_iter().collect::<BTreeSet<_>>().len(), 64);
        }
    }

    #[test]
    fn new_commander_progress_starts_empty_and_survives_existing_save() {
        assert!(World::new().unwritten_route_clue_progress().is_none());
        let world = World::new_commander(64171).unwrap_or_else(|error| panic!("{error:?}"));
        let save = world
            .save_bytes()
            .unwrap_or_else(|error| panic!("{error:?}"));
        let progress = world
            .unwritten_route_clue_progress()
            .unwrap_or_else(|| panic!("missing commander"));
        assert_eq!(progress.earned, []);
        assert_eq!(progress.missing.len(), 64);
        assert!(!progress.complete());
        assert_eq!(
            world
                .save_bytes()
                .unwrap_or_else(|error| panic!("{error:?}")),
            save
        );
        let resumed = World::from_save_bytes(&save).unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(resumed.unwritten_route_clue_progress(), Some(progress));
    }
}
