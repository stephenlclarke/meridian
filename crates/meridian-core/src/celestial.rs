//! Meridian-authored local solar scene and procedural surface recipe; no external art.
//! Positions are station-local world coordinates, not measured astronomy or L337 placement parity.
use crate::{
    docking::Vec3,
    navigation::{OriginalSystem, SystemId},
};
pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 128;
pub const SAMPLES: usize = WIDTH * HEIGHT;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyId {
    pub system: SystemId,
    pub visit: u64,
    pub entity: u64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Body {
    pub id: BodyId,
    pub position: Vec3,
    pub radius: f64,
}
impl Body {
    /// Conservative swept enclosing-sphere admission, including the real player hull radius.
    /// Invalid geometry blocks movement; endpoint-only tests cannot tunnel through a body.
    pub fn blocks_motion(self, before: [f32; 3], after: [f32; 3], hull_radius: f64) -> bool {
        if !hull_radius.is_finite()
            || hull_radius < 0.0
            || !self.radius.is_finite()
            || self.radius <= 0.0
        {
            return true;
        }
        let a = before.map(f64::from);
        let b = after.map(f64::from);
        let c = [self.position.x, self.position.y, self.position.z];
        if !a.into_iter().chain(b).chain(c).all(f64::is_finite) {
            return true;
        }
        let delta = std::array::from_fn::<_, 3, _>(|i| b[i] - a[i]);
        let length = delta.into_iter().map(|v| v * v).sum::<f64>();
        let t = if length > 0.0 {
            (0..3).map(|i| (c[i] - a[i]) * delta[i]).sum::<f64>() / length
        } else {
            0.0
        };
        let t = t.clamp(0.0, 1.0);
        let distance = (0..3)
            .map(|i| (a[i] + t * delta[i] - c[i]).powi(2))
            .sum::<f64>();
        !distance.is_finite() || distance <= (self.radius + hull_radius * 1.00001).powi(2)
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceRecipe {
    pub seed: u64,
    pub ocean: [f32; 3],
    pub land: [f32; 3],
    pub sea_level: f32,
    pub ice: f32,
    pub settlement_density: f32,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Snapshot {
    pub planet: Body,
    pub sun: Body,
    pub tick: u64,
    pub angle: f32,
    pub recipe: SurfaceRecipe,
}
impl Snapshot {
    pub(crate) fn blocks_motion(
        &self,
        before: [f32; 3],
        after: [f32; 3],
        hull_radius: f64,
    ) -> bool {
        [self.planet, self.sun]
            .into_iter()
            .any(|body| body.blocks_motion(before, after, hull_radius))
    }
}
pub fn scene(system: OriginalSystem, visit: u64, tick: u64, anchor: Vec3) -> Snapshot {
    let seed = mix(u64::from(system.id.galaxy) << 32 | u64::from(system.id.system));
    let mirror = if seed & 1 == 0 { 1.0 } else { -1.0 };
    let id = |entity| BodyId {
        system: system.id,
        visit,
        entity,
    };
    let position = |x, y, z| Vec3::new(anchor.x + x * mirror, anchor.y + y, anchor.z + z);
    let planet = Body {
        id: id(1),
        position: position(6000.0, -1800.0, 9500.0),
        radius: 1800.0 + (seed % 5) as f64 * 100.0,
    };
    let sun = Body {
        id: id(2),
        position: position(-8500.0, 4400.0, 14500.0),
        radius: 1300.0,
    };
    let tint = ((seed >> 8) & 255) as f32 / 255.0;
    let recipe = SurfaceRecipe {
        seed,
        ocean: [0.025, 0.10 + 0.06 * tint, 0.24 + 0.12 * tint],
        land: [
            0.18 + 0.16 * tint,
            0.22 + 0.18 * (1.0 - tint),
            0.09 + 0.08 * tint,
        ],
        sea_level: 0.43 + 0.13 * tint,
        ice: 0.78 + 0.12 * tint,
        settlement_density: 0.05 + 0.25 * tint,
    };
    Snapshot {
        planet,
        sun,
        tick,
        angle: (tick % 72000) as f32 / 72000.0 * std::f32::consts::TAU,
        recipe,
    }
}
fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}
fn value(seed: u64, x: usize, y: usize) -> f32 {
    (mix(seed ^ ((x as u64) << 32) ^ y as u64) >> 40) as f32 / 16777215.0
}
fn noise(seed: u64, x: usize, y: usize, columns: usize, rows: usize) -> f32 {
    let xx = x as f32 / WIDTH as f32 * columns as f32;
    let yy = y as f32 / (HEIGHT - 1) as f32 * rows as f32;
    let ix = xx.floor() as usize;
    let iy = yy.floor() as usize;
    let smooth = |v: f32| v * v * (3.0 - 2.0 * v);
    let tx = smooth(xx.fract());
    let ty = smooth(yy.fract());
    let a = value(seed, ix % columns, iy);
    let b = value(seed, (ix + 1) % columns, iy);
    let c = value(seed, ix % columns, (iy + 1).min(rows));
    let d = value(seed, (ix + 1) % columns, (iy + 1).min(rows));
    let top = a + (b - a) * tx;
    let bottom = c + (d - c) * tx;
    top + (bottom - top) * ty
}
/// Four bounded owned maps. Generation is deterministic by recipe; no file, GPU or clock access.
#[derive(Debug, PartialEq)]
pub struct SurfaceMaps {
    pub elevation: Vec<f32>,
    pub clouds: Vec<f32>,
    pub settlements: Vec<f32>,
    pub scatter: Vec<f32>,
}
impl SurfaceMaps {
    pub fn generate(recipe: SurfaceRecipe) -> Self {
        let mut elevation = Vec::with_capacity(SAMPLES);
        let mut clouds = Vec::with_capacity(SAMPLES);
        let mut settlements = Vec::with_capacity(SAMPLES);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let polar = ((y as f32 / (HEIGHT - 1) as f32 - 0.5) * 2.0).abs().powi(8);
                let height = 0.55 * noise(recipe.seed, x, y, 8, 4)
                    + 0.3 * noise(recipe.seed ^ 17, x, y, 16, 8)
                    + 0.15 * noise(recipe.seed ^ 31, x, y, 32, 16);
                let height = height * (1.0 - polar)
                    + value(recipe.seed, 0, usize::from(y > HEIGHT / 2)) * polar;
                let cloud = noise(recipe.seed ^ 71, x, y, 16, 8) * (1.0 - polar) + 0.5 * polar;
                let city = if height > recipe.sea_level && height < 0.78 && polar < 0.5 {
                    ((noise(recipe.seed ^ 97, x, y, 64, 32) - (1.0 - recipe.settlement_density))
                        * 8.0)
                        .clamp(0.0, 1.0)
                } else {
                    0.0
                };
                elevation.push(height);
                clouds.push(cloud);
                settlements.push(city);
            }
        }
        let mut scatter = vec![0.0; SAMPLES];
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let mut total = 0.0;
                for dy in -2_isize..=2 {
                    for dx in -2_isize..=2 {
                        let xx = (x as isize + dx).rem_euclid(WIDTH as isize) as usize;
                        let yy = (y as isize + dy).clamp(0, HEIGHT as isize - 1) as usize;
                        total += settlements[yy * WIDTH + xx];
                    }
                }
                scatter[y * WIDTH + x] = total / 25.0;
            }
        }
        Self {
            elevation,
            clouds,
            settlements,
            scatter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn system() -> OriginalSystem {
        crate::navigation::OriginalNavigation::bundled()
            .unwrap_or_else(|e| panic!("{e}"))
            .system(SystemId::AVENOR)
            .unwrap_or_else(|e| panic!("{e}"))
    }
    #[test]
    fn derived_bodies_are_finite_clear_of_birth_and_rebound_on_system_visit() {
        let s = scene(system(), 1, 0, Vec3::new(0.0, 0.0, 0.0));
        assert_ne!(s.planet.id, s.sun.id);
        assert!(!s.planet.blocks_motion([0.0; 3], [0.0, 0.0, 1000.0], 100.0));
        assert!(!s.sun.blocks_motion([0.0; 3], [0.0, 0.0, 1000.0], 100.0));
        let next = scene(system(), 2, 0, Vec3::new(0.0, 0.0, 0.0));
        assert_ne!(next.planet.id, s.planet.id);
        assert_eq!(next.recipe, s.recipe);
        let other = crate::navigation::OriginalNavigation::bundled()
            .unwrap_or_else(|e| panic!("{e}"))
            .system(SystemId::RENAS)
            .unwrap_or_else(|e| panic!("{e}"));
        let arrived = scene(other, 3, 600, Vec3::new(0.0, 0.0, 0.0));
        assert_ne!(arrived.recipe, s.recipe);
        assert_eq!(arrived.planet.id.system, SystemId::RENAS);
    }
    #[test]
    fn every_current_navigation_system_has_finite_safe_birth_geometry_and_owned_recipe() {
        let catalogue =
            crate::navigation::OriginalNavigation::bundled().unwrap_or_else(|e| panic!("{e}"));
        for index in 0..256 {
            let system = catalogue
                .system(SystemId {
                    galaxy: 0,
                    system: index,
                })
                .unwrap_or_else(|e| panic!("{e}"));
            let frame = scene(system, 1, u64::MAX, Vec3::new(0.0, 0.0, 0.0));
            assert!(frame.angle.is_finite());
            assert!((0.0..std::f32::consts::TAU).contains(&frame.angle));
            for b in [frame.planet, frame.sun] {
                assert!(
                    [b.position.x, b.position.y, b.position.z, b.radius]
                        .into_iter()
                        .all(f64::is_finite)
                );
                assert!(b.radius > 0.0);
                assert!(!b.blocks_motion([0.0; 3], [0.0, 0.0, 1000.0], 100.0));
            }
            assert_eq!(frame.planet.id.system, system.id);
            assert!((0.0..1.0).contains(&frame.recipe.sea_level));
        }
    }
    #[test]
    fn swept_sphere_blocks_tunnelling_tangent_and_invalid_geometry() {
        let mut body = scene(system(), 1, 0, Vec3::new(0.0, 0.0, 0.0)).planet;
        body.position = Vec3::new(0.0, 0.0, 0.0);
        body.radius = 10.0;
        assert!(body.blocks_motion([-100.0, 0.0, 0.0], [100.0, 0.0, 0.0], 1.0));
        assert!(body.blocks_motion([-100.0, 11.0, 0.0], [100.0, 11.0, 0.0], 1.0));
        assert!(!body.blocks_motion([-100.0, 12.0, 0.0], [100.0, 12.0, 0.0], 1.0));
        assert!(body.blocks_motion([f32::NAN, 0.0, 0.0], [0.0; 3], 1.0));
    }
    #[test]
    fn authored_maps_are_repeatable_bounded_seam_safe_and_poles_have_no_longitude_spikes() {
        let recipe = scene(system(), 1, 0, Vec3::new(0.0, 0.0, 0.0)).recipe;
        let a = SurfaceMaps::generate(recipe);
        let b = SurfaceMaps::generate(recipe);
        assert_eq!(a, b);
        for map in [&a.elevation, &a.clouds, &a.settlements, &a.scatter] {
            assert_eq!(map.len(), SAMPLES);
            assert!(map.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)));
        }
        for y in [0, HEIGHT - 1] {
            assert!(
                a.elevation[y * WIDTH..(y + 1) * WIDTH]
                    .iter()
                    .all(|v| *v == a.elevation[y * WIDTH])
            );
        }
        assert!(a.elevation.windows(2).any(|v| v[0] != v[1]));
        for y in 0..HEIGHT {
            assert!((a.elevation[y * WIDTH] - a.elevation[y * WIDTH + WIDTH - 1]).abs() < 0.1);
        }
    }
}
