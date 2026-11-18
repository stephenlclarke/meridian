//! Adapt a current World publication for the ordinary flight or isolated camera.
use crate::mule_render::{Placement, Scene};
use meridian_core::{
    FlightMode, Snapshot, World,
    mule::{MULE_CONTRACT_SHA256, MULE_RESOURCE_SHA256},
    traffic::MuleActorSnapshot,
};

pub(super) fn current_placement<'a>(
    world: &World,
    snapshot: Snapshot,
    frame: Option<&'a MuleActorSnapshot>,
) -> Option<Placement<'a>> {
    let frame = frame?;
    if frame.world_tick() != snapshot.tick
        || frame.source_identity() != (MULE_RESOURCE_SHA256, MULE_CONTRACT_SHA256)
        || !world.mule_snapshot_is_current(frame)
    {
        return None;
    }

    let actor = frame.actor();
    let envelope = frame.presentation().as_ref();
    if frame.is_renas_encounter() {
        return (snapshot.mode == FlightMode::Flight)
            .then_some(Placement::Ordinary { actor, envelope });
    }
    Some(Placement::Internal(Scene {
        actor,
        envelope,
        observer: world.mule_observer_pose(frame)?,
    }))
}

#[cfg(test)]
mod tests;
