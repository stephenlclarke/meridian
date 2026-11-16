//! Read-only projection of the genuine World mining provider. It grants no travel or work.
use super::{NavigationBody, NavigationBodyId};
use meridian_core::{World, navigation::SystemId};

/// Navigation-only address for the provider's belt centre. Real asteroid ordinals stay unchanged.
/// This sentinel must never be converted into an AsteroidId or a mining capability.
pub const MINING_BELT_CUE_ORDINAL: u64 = u64::MAX;

pub fn mining_navigation_bodies(world: &World, selected: SystemId) -> Vec<NavigationBody> {
    let Some(destination) = world.mining_destination(selected) else {
        return Vec::new();
    };
    let state = world.snapshot();
    let local = state.navigation.is_some_and(|nav| nav.system == selected);
    let origin = state.position.map(f64::from);
    let project = |ordinal, name, position: [f64; 3]| {
        let finite = position.into_iter().chain(origin).all(f64::is_finite);
        NavigationBody {
            id: NavigationBodyId {
                system: destination.system,
                ordinal,
            },
            name,
            position_world: position,
            distance_world: (local && finite).then(|| {
                (position[0] - origin[0])
                    .hypot(position[1] - origin[1])
                    .hypot(position[2] - origin[2])
            }),
            can_target: local && finite,
            refusal: if !local {
                Some("Travel to this system before selecting a local position cue".into())
            } else if !finite {
                Some("Local navigation position is unavailable".into())
            } else {
                None
            },
        }
    };
    let centre = destination.position;
    let mut result = vec![project(
        MINING_BELT_CUE_ORDINAL,
        destination.name.into(),
        [centre.x, centre.y, centre.z],
    )];
    if let Some(belt) = world
        .mining_belt_snapshot()
        .filter(|belt| local && belt.destination == destination)
    {
        result.extend(
            belt.bodies
                .into_iter()
                .filter(|body| body.id.system() == destination.system)
                .map(|body| {
                    project(
                        u64::from(body.id.ordinal()),
                        format!("{} / rock {:02}", destination.name, body.id.ordinal() + 1),
                        [body.position.x, body.position.y, body.position.z],
                    )
                }),
        );
    }
    result
}
