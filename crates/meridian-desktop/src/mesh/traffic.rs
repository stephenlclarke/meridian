//! Actual admitted SH025 hulls and retained beam segments, in the shared station camera.
use super::{StationRender, Vertex, append_mesh, append_segment, camera_for};
use meridian_core::traffic::{
    BeamSource, MAX_TRAFFIC_ACTORS, NpcPhase, Population, TrafficSnapshot,
};
use meridian_core::{PulseShot, Snapshot};

/// Four lifetime bodies, including destroyed solids, and one retained latest discharge.
pub fn vertex_capacity(ship_vertices: usize) -> Option<usize> {
    ship_vertices
        .checked_mul(MAX_TRAFFIC_ACTORS)?
        .checked_add(super::PULSE_VERTEX_COUNT)
}

fn recent_beam(scene: &TrafficSnapshot, tick: u64) -> Option<&meridian_core::traffic::TrafficBeam> {
    scene
        .last_beam
        .as_ref()
        .filter(|beam| tick.checked_sub(beam.world_tick).is_some_and(|age| age < 5))
}

/// The same committed player discharge can appear in both immutable projections.
pub fn contains_pulse(scene: &TrafficSnapshot, tick: u64, pulse: &PulseShot) -> bool {
    recent_beam(scene, tick).is_some_and(|beam| {
        matches!(beam.source, BeamSource::Player { item, .. } if item == pulse.fitted_item_id)
            && beam.world_tick == pulse.tick
            && beam.origin == pulse.origin
            && beam.point == pulse.end
    })
}

/// `ship` is the renderer's existing, decoded SH025 mesh, including its original colors.
/// This consumes a World projection; it never creates an actor or reconstructs a hit.
pub fn append(
    snapshot: &Snapshot,
    station: StationRender<'_>,
    ship: &[Vertex],
    scene: &TrafficSnapshot,
    output: &mut Vec<Vertex>,
) -> Result<(), String> {
    if scene.actors.len() > MAX_TRAFFIC_ACTORS {
        return Err("Traffic exceeds the admitted body bound".into());
    }
    let system = snapshot
        .navigation
        .ok_or("Traffic requires its actual navigation frame")?
        .system;
    for actor in &scene.actors {
        if actor.id.system() != system
            || actor.hull_id != "SH025"
            || !matches!(actor.population, Population::Trader | Population::Pirate)
        {
            return Err("Traffic has no admitted render hull in this frame".into());
        }
    }
    let beam = recent_beam(scene, snapshot.tick);
    if beam.is_some_and(|beam| {
        beam.victim.system() != system || !scene.actors.iter().any(|a| a.id == beam.victim)
    }) {
        return Err("Traffic beam has no retained victim in this frame".into());
    }
    let camera = camera_for(snapshot, station)?;
    let opening = output.len();
    let result = (|| {
        for actor in &scene.actors {
            if actor.phase != NpcPhase::Complete {
                append_mesh(ship, actor.pose, &camera, output)?;
            }
        }
        if let Some(beam) = beam {
            // The actual contact point is retained at discharge time. Current victim
            // motion, its phase or a UI target never extends or redirects this segment.
            append_segment(&camera, beam.origin, beam.point, output)?;
        }
        Ok(())
    })();
    if result.is_err() {
        output.truncate(opening);
    }
    result
}

#[cfg(test)]
mod tests;
