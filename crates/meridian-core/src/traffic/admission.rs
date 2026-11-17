//! Native station approach has right of way; traffic must yield before collision admission.
use super::*;
const YIELD_OFFSET: f64 = 1200.0;
#[cfg(test)]
mod tests;
pub(super) fn player_approaching(context: &TickContext<'_>) -> bool {
    let width = context.data.player_clearance_radius() * 2.0 + 64.0;
    [context.player_before, context.player_after]
        .into_iter()
        .any(|p| {
            p.position.z > 0.0
                && p.position.z <= 16000.0 + width
                && p.position.x.hypot(p.position.y) <= width
                && p.orientation.z.z < -0.95
        })
}
pub(super) fn yield_goal(actor: Actor, goal: Vec3, context: &TickContext<'_>) -> Option<Vec3> {
    if !player_approaching(context) || actor.position.z <= 0.0 || actor.position.z > 16000.0 {
        return None;
    }
    if actor.position.x.abs() >= YIELD_OFFSET && goal.x.abs() >= YIELD_OFFSET {
        return None;
    }
    let side = if actor.position.x < 0.0 { -1.0 } else { 1.0 };
    Some(Vec3::new(
        side * YIELD_OFFSET,
        actor.position.y,
        actor.position.z,
    ))
}
pub(super) fn lane_clear(before: Vec3, after: Vec3, context: &TickContext<'_>) -> bool {
    if !player_approaching(context) {
        return true;
    }
    let width = context.data.player_clearance_radius() * 2.0 + 64.0;
    let a = Vec3::new(before.x, before.y, 0.0);
    let b = Vec3::new(after.x, after.y, 0.0);
    // An actor already in the lane must physically leave it, not disappear. Existing
    // station/player/peer swept clearance still checks every one of those movements.
    if length(a) <= width {
        return length(b) >= length(a);
    }
    segment_distance(a, b, Vec3::new(0.0, 0.0, 0.0)) > width
}
