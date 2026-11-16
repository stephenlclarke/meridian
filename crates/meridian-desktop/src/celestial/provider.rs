use super::{BodyId, Palette, Planet, Scene, Sun, SurfaceData};
use meridian_core::{
    FlightMode,
    celestial::{Snapshot as CoreScene, SurfaceMaps, SurfaceRecipe},
};
use std::sync::Arc;
/// Bounded presentation cache for the actual core-owned system visit and recipe.
#[derive(Default)]
pub struct WorldAdapter {
    cached: Option<(BodyId, SurfaceRecipe, Arc<SurfaceData>)>,
}
impl WorldAdapter {
    pub fn scene(
        &mut self,
        source: Option<CoreScene>,
        mode: FlightMode,
    ) -> Result<Option<Scene>, &'static str> {
        let Some(source) = source else {
            self.cached = None;
            return Ok(None);
        };
        if mode == FlightMode::Docked {
            return Ok(None);
        }
        self.material_scene(source)
    }
    pub fn preview(
        &mut self,
        source: Option<CoreScene>,
        system: meridian_core::navigation::SystemId,
    ) -> Result<Option<Scene>, &'static str> {
        let Some(source) = source.filter(|s| s.planet.id.system == system) else {
            return Ok(None);
        };
        self.material_scene(source)
    }
    fn material_scene(&mut self, source: CoreScene) -> Result<Option<Scene>, &'static str> {
        if !self
            .cached
            .as_ref()
            .is_some_and(|(id, recipe, _)| *id == source.planet.id && *recipe == source.recipe)
        {
            let recipe = source.recipe;
            let maps = SurfaceMaps::generate(recipe);
            let palette = Palette {
                ocean: recipe.ocean,
                land: recipe.land,
                ice: recipe.ice,
                sea_level: recipe.sea_level,
                highland: [0.42, 0.36, 0.29],
                highland_strength: 0.65,
                vegetation: [0.10, 0.28, 0.08],
                vegetation_strength: 0.45,
                cloud: [0.86, 0.90, 0.94],
                cloud_low: 0.46,
                cloud_high: 0.68,
                cloud_opacity: 0.70,
                atmosphere: [0.14, 0.34, 0.70],
                atmosphere_strength: 0.40,
                surface_haze: 0.035,
                rim_width: 0.06,
                rim_opacity: 0.60,
                night: [1.0, 0.64, 0.28],
                night_intensity: 0.72,
                night_scatter: 0.20,
            };
            let surface = SurfaceData::new(
                source.planet.id,
                palette,
                [
                    &maps.elevation,
                    &maps.clouds,
                    &maps.settlements,
                    &maps.scatter,
                ],
            )?;
            self.cached = Some((source.planet.id, recipe, surface));
        }
        let surface = Arc::clone(
            &self
                .cached
                .as_ref()
                .ok_or("missing prepared celestial surface")?
                .2,
        );
        Ok(Some(Scene {
            planet: Some(Planet {
                id: source.planet.id,
                position: source.planet.position,
                radius: source.planet.radius,
                angle: source.angle,
                surface,
            }),
            sun: Some(Sun {
                id: source.sun.id,
                position: source.sun.position,
                radius: source.sun.radius,
            }),
        }))
    }
}
