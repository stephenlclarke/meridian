//! Finite released ore identities. No cargo credit or scoop installation is inferred.
use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OreId {
    asteroid: AsteroidId,
    fragment: u8,
}
impl OreId {
    pub fn asteroid(self) -> AsteroidId {
        self.asteroid
    }
    pub fn fragment(self) -> u8 {
        self.fragment
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OreSnapshot {
    pub id: OreId,
    pub commodity: CommodityId,
    pub quantity: u8,
    pub position: Vec3,
    pub velocity: Vec3,
    pub released_tick: u64,
    pub released_physical_tick: u64,
}
impl State {
    pub(crate) fn pending_count(&self) -> usize {
        self.bodies()
            .into_iter()
            .filter(|b| self.expired[usize::from(b.id.ordinal)].is_none())
            .map(|b| usize::from(b.extracted_units))
            .sum()
    }
    pub(crate) fn pending_ore(&self, physical_tick: u64) -> Vec<OreSnapshot> {
        let mut ore = Vec::with_capacity(self.pending_count());
        for body in self.bodies() {
            let slot = usize::from(body.id.ordinal);
            if self.expired[slot].is_some() {
                continue;
            }
            let Some(release) = self.terminal_stroke(slot) else {
                continue;
            };
            let elapsed = physical_tick.saturating_sub(release.physical_tick) as f64 / 60.0;
            let columns = body.initial_units.min(5);
            let rows = body.initial_units / columns;
            for fragment in 0..body.initial_units {
                // Original MiningLedger.yield grid: cargo fallback radius48 => spacing112.
                let center = Vec3::new(
                    f64::from(columns - 1) * 6.0,
                    f64::from(rows - 1) * 6.0,
                    10.0,
                );
                let offset = Vec3::new(
                    (f64::from(fragment % columns) - f64::from(columns - 1) / 2.0) * 112.0,
                    (f64::from(fragment / columns) - f64::from(rows - 1) / 2.0) * 112.0,
                    0.0,
                );
                let velocity = Vec3::new(f64::from(fragment % 5), 2.0, 3.0);
                ore.push(OreSnapshot {
                    id: OreId {
                        asteroid: body.id,
                        fragment,
                    },
                    commodity: body.commodity,
                    quantity: 1,
                    position: add(
                        add(body.position, add(center, offset)),
                        scale(velocity, elapsed),
                    ),
                    velocity,
                    released_tick: release.world_tick,
                    released_physical_tick: release.physical_tick,
                });
            }
        }
        ore
    }
}
