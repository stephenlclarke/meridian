//! Geometry-only parity of the generic authoring package with the stock SH023 supports.
//! No campaign admission, purchased hull, renderer, or new collision kernel is exercised.

use meridian_core::{
    OriginalStationData,
    docking::{Basis3, RigidPose, Vec3},
    hull_package::HullPackage,
    mule::{EnvelopeError, SmoothPiece},
    peregrine_mk3::{
        PEREGRINE_MK3_DETAILED_FACES, PEREGRINE_MK3_DETAILED_VERTICES,
        PEREGRINE_MK3_NATIVE_VERTICES, PEREGRINE_MK3_PIECES, PeregrineMk3HullSource,
    },
};
use serde_json::{Value, json};

const VERTICES: &[u8] = include_bytes!("../../../assets/fleet/sh023/source.vertices");
const FACES: &[u8] = include_bytes!("../../../assets/fleet/sh023/source-triangles.bin");
const GROUPS: &[u8] = include_bytes!("../../../assets/fleet/sh023/source-piece-membership.bin");
const ENGINEERING: &str = include_str!("../../../assets/fleet/sh023/engineering.json");
const PIECES: &str = include_str!("../../../assets/fleet/sh023/PIECES.json");
const FIXTURES: &str = include_str!("../../../integration/sh023-peregrine-mk3/FIXTURES.json");
const STATION: &[u8] = include_bytes!("../../../assets/legacy/sh028.physics");

fn u32_at(bytes: &[u8], index: usize) -> usize {
    u32::from_le_bytes(
        bytes[index..index + 4]
            .try_into()
            .unwrap_or_else(|e| panic!("{e}")),
    ) as usize
}

fn points(bytes: &[u8]) -> Vec<[f64; 3]> {
    let (words, trailing) = bytes.as_chunks::<24>();
    assert!(trailing.is_empty());
    words
        .iter()
        .map(|word| {
            let (parts, _) = word.as_chunks::<8>();
            [parts[0], parts[1], parts[2]].map(f64::from_le_bytes)
        })
        .collect()
}

fn faces(bytes: &[u8]) -> Vec<[usize; 3]> {
    let (words, trailing) = bytes.as_chunks::<12>();
    assert!(trailing.is_empty());
    words
        .iter()
        .map(|word| [0, 4, 8].map(|offset| u32_at(word, offset)))
        .collect()
}

fn vector([x, y, z]: [f64; 3]) -> Vec3 {
    Vec3::new(x, y, z)
}

fn original_geometry_package() -> Result<HullPackage, serde_json::Error> {
    assert_eq!(&VERTICES[..8], b"MSH023V1");
    let nd = PEREGRINE_MK3_DETAILED_VERTICES;
    let nn = PEREGRINE_MK3_NATIVE_VERTICES;
    let nf = PEREGRINE_MK3_DETAILED_FACES;
    assert_eq!((u32_at(VERTICES, 8), u32_at(VERTICES, 12)), (nd, nn));
    let xyz = points(&VERTICES[16..]);
    let triangles = faces(FACES);
    assert_eq!(xyz.len(), nd + nn);
    assert_eq!(GROUPS.len(), nd + nn);
    assert_eq!(triangles.len(), nf + 512);
    let native = triangles[nf..]
        .iter()
        .map(|face| face.map(|index| index - nd))
        .collect::<Vec<_>>();
    let engineering: Value = serde_json::from_str(ENGINEERING)?;
    let tensor: [f64; 9] =
        serde_json::from_value(engineering["inertiaAboutCentreOfMassKgM2"].clone())?;
    let pieces: Value = serde_json::from_str(PIECES)?;
    // A neutral appearance is supplied only to satisfy package schema; appearance
    // and exact source materials are independently covered by the Studio export smoke.
    // Geometry, group membership and rounding below are the original bundled buffers.
    let definition = json!({
        "schema": 1, "id": "SH023", "name": "Peregrine Mk III",
        "metresPerUnit": 1, "roundingMetres": pieces["roundingMetres"],
        "detail": {"vertices": &xyz[..nd], "triangles": &triangles[..nf], "groups": &GROUPS[..nd]},
        "native": {"vertices": &xyz[nd..], "triangles": native, "groups": &GROUPS[nd..]},
        "materials": [{"name": "Geometry parity only", "rgb": [128,128,128]}],
        "faceMaterials": vec![0; nf],
        "engineering": {"massKg": engineering["dryMassKilograms"],
                        "centerOfMass": engineering["centreOfMassMetres"],
                        "inertia": [[tensor[0],tensor[1],tensor[2]], [tensor[3],tensor[4],tensor[5]], [tensor[6],tensor[7],tensor[8]]]},
        "flight": {"maxSpeed": 35.7},
        "clearances": [
            {"name":"Axial bow gap", "start":[0,-1.45,18.5], "end":[0,-1.45,8.5], "radiusMetres":0.1},
            {"name":"Lateral bow trench", "start":[2.35,-1.4,12], "end":[-2.35,-1.4,12], "radiusMetres":0.1},
            {"name":"Port shoulder", "start":[-2.25,-1.45,17.5], "end":[-2.25,-1.45,8.75], "radiusMetres":0.1},
            {"name":"Starboard shoulder", "start":[2.25,-1.45,17.5], "end":[2.25,-1.45,8.75], "radiusMetres":0.1}
        ]
    });
    HullPackage::from_json(&serde_json::to_vec(&definition)?)
}

fn pose(position: [f64; 3], inward: bool) -> RigidPose {
    RigidPose {
        position: vector(position),
        orientation: if inward {
            Basis3 {
                x: Vec3::new(-1.0, 0.0, 0.0),
                y: Vec3::new(0.0, 1.0, 0.0),
                z: Vec3::new(0.0, 0.0, -1.0),
            }
        } else {
            Basis3::IDENTITY
        },
    }
}

fn sweep(
    pieces: &[SmoothPiece],
    start: RigidPose,
    end: RigidPose,
    triangle: [Vec3; 3],
) -> Result<bool, EnvelopeError> {
    for piece in pieces {
        if !piece.sweep_clears_triangle(start, end, 0.0, triangle)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn station_triangles() -> Vec<[Vec3; 3]> {
    assert_eq!(&STATION[..8], b"MPHYS001");
    let count = u32_at(STATION, 8);
    let nfaces = u32_at(STATION, 12);
    let split = 16 + count * 24;
    let xyz = points(&STATION[16..split]);
    let triangles = faces(&STATION[split..]);
    assert_eq!(xyz.len(), count);
    assert_eq!(triangles.len(), nfaces);
    triangles
        .iter()
        .map(|face| face.map(|index| vector(xyz[index])))
        .collect()
}

#[test]
fn generic_sh023_package_matches_stock_triangle_and_station_clearance()
-> Result<(), Box<dyn std::error::Error>> {
    let package = original_geometry_package()?;
    assert_eq!(package.id(), "SH023");
    let custom = package
        .pieces()
        .unwrap_or_else(|report| panic!("invalid SH023 package: {report:?}"));
    let stock = PeregrineMk3HullSource::bundled()?;
    assert_eq!(custom.len(), PEREGRINE_MK3_PIECES);
    for (a, b) in custom.iter().zip(stock.pieces()) {
        assert_eq!(a.rounding_metres(), b.rounding_metres());
        assert_eq!(a.source_points().len(), b.source_points().len());
        // JSON f64 parsing may differ by a last bit from direct binary f64 decoding.
        for (p, q) in a.source_points().iter().zip(b.source_points()) {
            assert!(
                (p.x - q.x).abs() <= 1e-12
                    && (p.y - q.y).abs() <= 1e-12
                    && (p.z - q.z).abs() <= 1e-12
            );
        }
    }

    let fixtures: Value = serde_json::from_str(FIXTURES)?;
    let routes = fixtures["routes"]
        .as_array()
        .unwrap_or_else(|| panic!("missing retained routes"));
    assert_eq!(routes.len(), 4);
    let origin = pose([0.0; 3], false);
    for route in routes {
        let triangle: [[f64; 3]; 3] = serde_json::from_value(route["triangle"].clone())?;
        let travel: [f64; 3] = serde_json::from_value(route["travel"].clone())?;
        let triangle = triangle.map(vector);
        let end = pose(travel, false);
        assert!(
            stock.sweep_clears_triangle(origin, end, 0.0, triangle)?,
            "stock route {}",
            route["name"]
        );
        assert!(
            sweep(&custom, origin, end, triangle)?,
            "generic route {}",
            route["name"]
        );
    }
    let vertices = points(&VERTICES[16..]);
    let solids: Vec<usize> = serde_json::from_value(fixtures["sourceSolidContacts"].clone())?;
    assert_eq!(solids, [112, 680, 1728]);
    for index in solids {
        let [x, y, z] = vertices[index];
        let triangle = [[x, y, z], [x + 0.001, y, z], [x, y + 0.001, z]].map(vector);
        assert!(!stock.sweep_clears_triangle(origin, origin, 0.0, triangle)?);
        assert!(
            !sweep(&custom, origin, origin, triangle)?,
            "generic source solid {index}"
        );
    }

    // The public stock station method applies the same triangle kernel to its
    // authenticated native mesh. Its generic station helper is private; feed
    // the existing public triangle kernel the very same bundled SH028 bytes.
    let station = OriginalStationData::bundled()?;
    let triangles = station_triangles();
    assert_eq!(triangles.len(), station.station_triangle_count());
    let [aligned_before, aligned_after]: [f64; 2] =
        serde_json::from_value(fixtures["station"]["alignedZ"].clone())?;
    let [lip_before, lip_after]: [f64; 2] =
        serde_json::from_value(fixtures["station"]["lipCentreZ"].clone())?;
    let offset_x: f64 = serde_json::from_value(fixtures["station"]["actorX"].clone())?;
    for (label, start, end, expected) in [
        (
            "aligned aperture",
            pose([0.0, 0.0, aligned_before], true),
            pose([0.0, 0.0, aligned_after], true),
            true,
        ),
        (
            "offset opening",
            pose([offset_x, 0.0, lip_before], true),
            pose([offset_x, 0.0, lip_before], true),
            true,
        ),
        (
            "offset lip contact",
            pose([offset_x, 0.0, lip_before], true),
            pose([offset_x, 0.0, lip_after], true),
            false,
        ),
    ] {
        let stock_result = station.peregrine_mk3_motion_clear(&stock, start, end, 0.0);
        let custom_result = triangles
            .iter()
            .all(|triangle| sweep(&custom, start, end, *triangle) == Ok(true));
        assert_eq!(stock_result, expected, "stock {label}");
        assert_eq!(custom_result, stock_result, "generic {label}");
    }
    Ok(())
}
