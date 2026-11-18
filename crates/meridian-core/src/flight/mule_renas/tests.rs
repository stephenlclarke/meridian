use super::World;
use crate::{FlightInput, FlightMode};

const LEGACY_OVERLAP: &[u8] =
    include_bytes!("../../../tests/fixtures/renas-mule-pre-pr-overlap-v17.bin");

#[test]
fn genuine_pre_pr_v17_overlap_continues_in_place_and_releases_the_same_mule_after_escape() {
    let checked = World::decode_saved_world(LEGACY_OVERLAP).unwrap();
    assert_eq!(checked.save_bytes().unwrap(), LEGACY_OVERLAP);
    let mut world = World::from_save_bytes(LEGACY_OVERLAP).unwrap();
    let opening = world.snapshot();
    assert_eq!(opening.mode, FlightMode::Flight);
    assert_eq!(opening.position, [133.50058, 10.411001, 1324.3611]);
    assert_eq!(opening.orientation, [0.0, 0.0, 0.0, 1.0]);
    assert!(world.mule_actor_snapshot().is_none());
    let deferred = world.mule_renas.as_ref().unwrap();
    assert!(deferred.renas_is_deferred());
    let intended_pose = deferred.pose();
    let intended_generation = opening.navigation.unwrap().visit;

    let freight = world.mission_snapshot().unwrap().freight;
    assert_eq!(freight.completed_jobs, 1);
    assert!(freight.clue.is_some());
    assert_eq!(world.credits_tenths(), Some(14500));

    // A second cold continuation while still intersecting must not publish it.
    let mut replay = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    assert_eq!(replay.snapshot(), opening);
    assert!(replay.mule_actor_snapshot().is_none());

    let mut escaped = None;
    for _ in 0..128 {
        world.step(FlightInput::default()).unwrap();
        replay.step(FlightInput::default()).unwrap();
        assert_eq!(world.snapshot(), replay.snapshot());
        assert_eq!(world.snapshot().motion_refusal, None);
        assert_ne!(world.snapshot().position, opening.position);

        let current = world.mule_actor_snapshot();
        let cold = replay.mule_actor_snapshot();
        assert_eq!(current.is_some(), cold.is_some());
        if let Some(current) = current {
            let cold = cold.unwrap();
            assert_eq!(current.actor(), cold.actor());
            assert_eq!(current.generation(), cold.generation());
            assert_eq!(current.actor().pose, intended_pose);
            assert_eq!(current.generation(), intended_generation);
            assert_eq!(current.crew_aboard(), Some(2));
            assert!(world.mule_snapshot_is_current(&current));
            assert!(!replay.mule_snapshot_is_current(&current));
            escaped = Some(current);
            break;
        }
    }

    let escaped = escaped.unwrap_or_else(|| panic!("ordinary flight never cleared the Mule berth"));
    let again = World::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    let restored = again.mule_actor_snapshot().unwrap();
    assert_eq!(restored.actor(), escaped.actor());
    assert_eq!(restored.presentation(), escaped.presentation());
    assert_eq!(restored.generation(), escaped.generation());
    assert_eq!(again.mission_snapshot().unwrap().freight, freight);
    assert_eq!(again.credits_tenths(), Some(14500));
}
