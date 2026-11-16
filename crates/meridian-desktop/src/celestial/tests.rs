use super::*;
fn projection() -> Projection {
    Projection {
        viewport: [640, 360],
        clip: [0.0, 0.0, 640.0, 360.0],
        center: [320.0, 180.0],
        focal: 288.0,
        near: 1.0,
        far: 20000.0,
    }
}
fn palette() -> Palette {
    Palette {
        ocean: [0.0, 0.1, 0.4],
        land: [0.3, 0.4, 0.1],
        ice: 0.8,
        sea_level: 0.5,
        highland: [0.5; 3],
        highland_strength: 0.6,
        vegetation: [0.0, 0.4, 0.0],
        vegetation_strength: 0.4,
        cloud: [1.0; 3],
        cloud_low: 0.4,
        cloud_high: 0.7,
        cloud_opacity: 0.5,
        atmosphere: [0.1, 0.3, 0.8],
        atmosphere_strength: 0.4,
        surface_haze: 0.05,
        rim_width: 0.06,
        rim_opacity: 0.7,
        night: [1.0, 0.63, 0.27],
        night_intensity: 0.5,
        night_scatter: 0.2,
    }
}
fn world() -> meridian_core::World {
    meridian_core::World::new_commander(90022).unwrap_or_else(|e| panic!("{e}"))
}
fn id(snapshot: Snapshot, entity: u64) -> BodyId {
    let nav = snapshot
        .navigation
        .unwrap_or_else(|| panic!("missing navigation"));
    BodyId {
        system: nav.system,
        visit: nav.visit,
        entity,
    }
}
fn surface(id: BodyId) -> Arc<SurfaceData> {
    let map = vec![0.5; MAP_SAMPLES];
    SurfaceData::new(id, palette(), [&map; 4]).unwrap_or_else(|e| panic!("{e}"))
}
#[test]
fn wgsl_compiles_and_validates_with_current_native_shader_frontend() {
    let module = wgpu::naga::front::wgsl::parse_str(include_str!("celestial.wgsl"))
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(include_str!("celestial.wgsl"))));
    wgpu::naga::valid::Validator::new(
        wgpu::naga::valid::ValidationFlags::all(),
        wgpu::naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap_or_else(|e| panic!("{e}"));
    let entries: Vec<_> = module
        .entry_points
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    assert_eq!(entries, ["vertex_main", "surface_main", "halo_main"]);
    let ty = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some("Uniforms"))
        .map(|(_, t)| &t.inner)
        .unwrap_or_else(|| panic!("missing uniforms"));
    let wgpu::naga::TypeInner::Struct { members, span } = ty else {
        panic!("uniforms must be a struct")
    };
    assert_eq!(*span as usize, std::mem::size_of::<Uniforms>());
    assert_eq!(members.len(), 13);
    assert_eq!(
        members.iter().map(|m| m.offset).collect::<Vec<_>>(),
        (0..13).map(|i| i * 16).collect::<Vec<_>>()
    );
}
#[test]
fn camera_relative_light_has_real_day_night_and_missing_sun_is_unlit() {
    let planet = Vec3::new(0.0, 0.0, 10.0);
    assert_eq!(
        flight_light(planet, Some(Vec3::new(0.0, 0.0, 0.0))),
        [0.0, 0.0, 1.0, 1.0]
    );
    assert_eq!(
        flight_light(planet, Some(Vec3::new(0.0, 0.0, 20.0))),
        [0.0, 0.0, -1.0, 1.0]
    );
    assert_eq!(flight_light(planet, None), [0.0; 4]);
    assert_eq!(flight_light(planet, Some(planet)), [0.0; 4]);
    assert_eq!(
        flight_light(planet, Some(Vec3::new(f64::NAN, 0.0, 0.0))),
        [0.0; 4]
    );
    let right = flight_light(Vec3::new(10.0, 0.0, 0.0), Some(Vec3::new(0.0, 0.0, 0.0)));
    assert!(right.into_iter().all(f32::is_finite));
    assert_eq!(right[2], 1.0);
}
#[test]
fn actual_camera_transform_keeps_ship_motion_and_all_views_consistent() {
    let world = world();
    let mut snapshot = world.snapshot();
    snapshot.position = [10.0, 20.0, 30.0];
    snapshot.orientation = [0.0, 0.0, 0.0, 1.0];
    let body = Vec3::new(10.0, 20.0, 130.0);
    let get = |v| view_position(snapshot, v, body).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(get(FlightView::Front), Vec3::new(0.0, 0.0, 100.0));
    assert_eq!(get(FlightView::Rear), Vec3::new(0.0, 0.0, -100.0));
    assert_eq!(get(FlightView::Left), Vec3::new(100.0, 0.0, 0.0));
    assert_eq!(get(FlightView::Right), Vec3::new(-100.0, 0.0, 0.0));
    snapshot.orientation = [0.0; 4];
    assert!(view_position(snapshot, FlightView::Front, body).is_err());
}
#[test]
fn projection_keeps_edge_limbs_culls_behind_and_bounds_interiors_and_close_approach() {
    let p = projection();
    let disk = project_sphere(Vec3::new(0.0, 0.0, 100.0), 10.0, p, 1.0)
        .unwrap_or_else(|e| panic!("{e}"))
        .unwrap_or_else(|| panic!("missing sphere"));
    assert_eq!(disk.center, [320.0, 180.0]);
    assert!((disk.radius - 28.8).abs() < 0.0001);
    assert!(
        project_sphere(Vec3::new(13.5, 0.0, 10.0), 3.0, p, 1.0)
            .unwrap_or_else(|e| panic!("{e}"))
            .is_some()
    );
    assert!(
        project_sphere(Vec3::new(100.0, 0.0, 10.0), 1.0, p, 1.15)
            .unwrap_or_else(|e| panic!("{e}"))
            .is_none()
    );
    assert!(
        project_sphere(Vec3::new(0.0, 0.0, -10.0), 1.0, p, 1.15)
            .unwrap_or_else(|e| panic!("{e}"))
            .is_none()
    );
    let inside = project_sphere(Vec3::new(0.0, 0.0, 0.0), 100.0, p, 1.0)
        .unwrap_or_else(|e| panic!("{e}"))
        .unwrap_or_else(|| panic!("missing interior"));
    assert_eq!(inside.center, [320.0, 180.0]);
    assert!(inside.radius <= 4.0 * 640.0);
    for r in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(project_sphere(Vec3::new(0.0, 0.0, 10.0), r, p, 1.0).is_err());
    }
}
#[test]
fn invalid_projection_and_extreme_coordinates_refuse_without_allocating_an_image() {
    let mut p = projection();
    p.clip[2] = 641.0;
    assert!(project_sphere(Vec3::new(0.0, 0.0, 10.0), 1.0, p, 1.0).is_err());
    p = projection();
    p.near = p.far;
    assert!(!p.valid());
    p = projection();
    p.viewport = [0, 360];
    assert!(!p.valid());
    assert!(project_sphere(Vec3::new(f64::INFINITY, 0.0, 10.0), 1.0, projection(), 1.0).is_err());
}
#[test]
fn maps_are_exactly_bounded_interleaved_and_bad_samples_or_degenerate_profiles_refuse() {
    let id = id(world().snapshot(), 1);
    let values = vec![0.5; MAP_SAMPLES];
    let tiny = [0.0];
    let surface = SurfaceData::new(id, palette(), [&values; 4]).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(surface.samples.len(), MAP_SAMPLES);
    assert_eq!(surface.samples[0], [0.5; 4]);
    assert!(SurfaceData::new(id, palette(), [&tiny; 4]).is_err());
    let mut bad = values.clone();
    bad[3] = f32::NAN;
    assert!(SurfaceData::new(id, palette(), [&values, &bad, &values, &values]).is_err());
    let mut c = palette();
    c.rim_width = 0.0;
    assert!(SurfaceData::new(id, c, [&values; 4]).is_err());
    c = palette();
    c.cloud_high = c.cloud_low;
    assert!(SurfaceData::new(id, c, [&values; 4]).is_err());
    c = palette();
    c.sea_level = 0.9;
    assert!(SurfaceData::new(id, c, [&values; 4]).is_err());
}
#[test]
fn retained_body_or_foreign_surface_cannot_render_as_the_current_system_visit() {
    let world = world();
    let snapshot = world.snapshot();
    let body_id = id(snapshot, 1);
    let mut scene = Scene {
        planet: Some(Planet {
            id: body_id,
            position: Vec3::new(0.0, 0.0, 100.0),
            radius: 10.0,
            angle: 0.0,
            surface: surface(body_id),
        }),
        sun: None,
    };
    assert!(scene.validate(snapshot).is_ok());
    let mut foreign = body_id;
    foreign.visit += 1;
    if let Some(p) = scene.planet.as_mut() {
        p.surface = surface(foreign);
    }
    assert!(scene.validate(snapshot).is_err());
    if let Some(p) = scene.planet.as_mut() {
        p.surface = surface(body_id);
        p.id = foreign;
    }
    assert!(scene.validate(snapshot).is_err());
    let empty = Scene {
        planet: None,
        sun: None,
    };
    assert!(empty.validate(snapshot).is_ok());
}
#[test]
fn named_uniforms_keep_palette_light_map_semantics_and_sun_has_no_decorative_terrain() {
    let disk = Disk {
        center: [20.0, 30.0],
        radius: 40.0,
        view_depth: 1000.0,
        world_radius: 100.0,
    };
    let u = Uniforms::new(
        disk,
        projection(),
        Some(palette()),
        1.0,
        [0.0, 0.0, -1.0, 1.0],
    );
    assert_eq!(u.light, [0.0, 0.0, -1.0, 1.0]);
    assert_eq!(u.geometry, [40.0, 1.0, 1000.0, 100.0]);
    assert_eq!(u.land[3], palette().sea_level);
    assert_eq!(u.night_levels, [0.5, 0.2, 0.7, 0.0]);
    assert_eq!(u.depth[2], 0.0);
    let sun = Uniforms::new(disk, projection(), None, 0.0, [0.0; 4]);
    assert_eq!(sun.depth[2..], [1.0, 1.15]);
    assert_eq!(sun.ocean, [0.0; 4]);
}

#[test]
fn complete_frame_refuses_bad_second_body_and_empty_or_subpixel_scene_stays_empty() {
    let world = world();
    let snapshot = world.snapshot();
    let body_id = id(snapshot, 1);
    let mut scene = Scene {
        planet: Some(Planet {
            id: body_id,
            position: Vec3::new(0.0, 0.0, 1000.0),
            radius: 100.0,
            angle: std::f32::consts::TAU * 3.0,
            surface: surface(body_id),
        }),
        sun: Some(Sun {
            id: id(snapshot, 2),
            position: Vec3::new(1000.0, 0.0, 2000.0),
            radius: f64::NAN,
        }),
    };
    assert!(frame_plan(&scene, snapshot, FlightView::Front, projection()).is_err());
    scene.sun = None;
    let plan = frame_plan(&scene, snapshot, FlightView::Front, projection())
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(plan.uniforms[0].is_some());
    assert!(plan.uniforms[1].is_none());
    assert_eq!(plan.uniforms[0].map(|u| u.light), Some([0.0; 4]));
    let mut p = projection();
    p.clip = [0.8, 0.8, 0.1, 0.1];
    assert!(
        frame_plan(&scene, snapshot, FlightView::Front, p)
            .unwrap_or_else(|e| panic!("{e}"))
            .uniforms
            .into_iter()
            .all(|v| v.is_none())
    );
    let empty = Scene {
        planet: None,
        sun: None,
    };
    assert!(
        frame_plan(&empty, snapshot, FlightView::Front, projection())
            .unwrap_or_else(|e| panic!("{e}"))
            .uniforms
            .into_iter()
            .all(|v| v.is_none())
    );
}
