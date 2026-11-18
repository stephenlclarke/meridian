//! Build only a presentation boundary of the same convex source supports.
//! Facets do not approximate the 2cm ball: the shader intersects its faces/cylinders/spheres analytically.
use meridian_core::docking::Vec3;
use std::collections::BTreeMap;

const MAX_WORK: usize = 16_000_000;
const MAX_FACES: usize = 10_688;
#[derive(Clone, Copy)]
pub(super) struct Face(pub [usize; 3]);
pub(super) struct Hull {
    pub faces: Vec<Face>,
    pub error_metres: f64,
}
fn sub(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
fn add(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
fn scale(a: Vec3, s: f64) -> Vec3 {
    Vec3::new(a.x * s, a.y * s, a.z * s)
}
fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}
fn norm(a: Vec3) -> f64 {
    a.dot(a).sqrt()
}
fn plane(points: &[Vec3], f: Face) -> Result<(Vec3, f64), &'static str> {
    let [a, b, c] = f.0.map(|i| points[i]);
    let n = cross(sub(b, a), sub(c, a));
    let length = norm(n);
    if !length.is_finite() || length < 1e-12 {
        return Err("SH001 degenerate support facet");
    }
    let n = scale(n, 1.0 / length);
    Ok((n, n.dot(a)))
}
fn orient(points: &[Vec3], mut face: Face, inside: Vec3) -> Result<Face, &'static str> {
    let (n, h) = plane(points, face)?;
    if n.dot(inside) > h {
        face.0.swap(1, 2);
    }
    Ok(face)
}
fn charge(work: &mut usize, count: usize) -> Result<(), &'static str> {
    *work = work.checked_add(count).ok_or("SH001 hull work overflow")?;
    if *work > MAX_WORK {
        return Err("SH001 bounded render hull work exhausted");
    }
    Ok(())
}
pub(super) fn build(points: &[Vec3]) -> Result<Hull, &'static str> {
    if points.len() < 4
        || points.len() > 5344
        || points.iter().any(|p| {
            [p.x, p.y, p.z]
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 64.0)
        })
    {
        return Err("SH001 invalid support source");
    }
    let a = 0;
    let b = (1..points.len())
        .max_by(|i, j| {
            norm(sub(points[*i], points[a])).total_cmp(&norm(sub(points[*j], points[a])))
        })
        .ok_or("SH001 missing extent")?;
    let ab = sub(points[b], points[a]);
    let c = (0..points.len())
        .max_by(|i, j| {
            norm(cross(ab, sub(points[*i], points[a])))
                .total_cmp(&norm(cross(ab, sub(points[*j], points[a]))))
        })
        .ok_or("SH001 missing area")?;
    let (n, h) = plane(points, Face([a, b, c]))?;
    let d = (0..points.len())
        .max_by(|i, j| {
            (n.dot(points[*i]) - h)
                .abs()
                .total_cmp(&(n.dot(points[*j]) - h).abs())
        })
        .ok_or("SH001 missing volume")?;
    if (n.dot(points[d]) - h).abs() < 1e-8 {
        return Err("SH001 flat support set");
    }
    let inside = scale(
        add(add(points[a], points[b]), add(points[c], points[d])),
        0.25,
    );
    let mut faces = [[a, b, c], [a, d, b], [a, c, d], [b, d, c]]
        .into_iter()
        .map(|f| orient(points, Face(f), inside))
        .collect::<Result<Vec<_>, _>>()?;
    let epsilon = 1e-10;
    let mut work = points.len() * 6;
    for index in 0..points.len() {
        charge(&mut work, faces.len())?;
        let visible = faces
            .iter()
            .map(|f| plane(points, *f).map(|(n, h)| n.dot(points[index]) - h > epsilon))
            .collect::<Result<Vec<_>, _>>()?;
        if !visible.iter().any(|v| *v) {
            continue;
        }
        let mut horizon = BTreeMap::<(usize, usize), (usize, usize, usize)>::new();
        for (face, visible) in faces.iter().zip(&visible) {
            if *visible {
                for [u, v] in [
                    [face.0[0], face.0[1]],
                    [face.0[1], face.0[2]],
                    [face.0[2], face.0[0]],
                ] {
                    let key = (u.min(v), u.max(v));
                    let edge = horizon.entry(key).or_insert((u, v, 0));
                    edge.2 += 1;
                    if edge.2 > 2 {
                        return Err("SH001 nonmanifold visible horizon");
                    }
                }
            }
        }
        faces = faces
            .into_iter()
            .zip(visible)
            .filter_map(|(f, v)| (!v).then_some(f))
            .collect();
        for (u, v, count) in horizon.into_values() {
            if count == 1 {
                faces.push(orient(points, Face([u, v, index]), inside)?);
            }
        }
        if faces.len() > MAX_FACES {
            return Err("SH001 render facet bound exhausted");
        }
    }
    certify(points, faces, inside, &mut work)
}
fn certify(
    points: &[Vec3],
    faces: Vec<Face>,
    inside: Vec3,
    work: &mut usize,
) -> Result<Hull, &'static str> {
    let mut edges = BTreeMap::<(usize, usize), (usize, i32)>::new();
    let mut vertices = std::collections::BTreeSet::new();
    let mut rho = f64::INFINITY;
    let mut slack = 0.0f64;
    for face in &faces {
        let (n, h) = plane(points, *face)?;
        rho = rho.min(h - n.dot(inside));
        charge(work, points.len())?;
        for p in points {
            slack = slack.max(n.dot(*p) - h);
        }
        for [u, v] in [
            [face.0[0], face.0[1]],
            [face.0[1], face.0[2]],
            [face.0[2], face.0[0]],
        ] {
            vertices.insert(u);
            vertices.insert(v);
            let e = edges.entry((u.min(v), u.max(v))).or_insert((0, 0));
            e.0 += 1;
            e.1 += if u < v { 1 } else { -1 };
        }
    }
    if faces.is_empty()
        || edges.values().any(|e| *e != (2, 0))
        || vertices.len() + faces.len() != edges.len() + 2
        || !rho.is_finite()
        || rho < 1e-8
    {
        return Err("SH001 support boundary certificate failed");
    }
    let radius = points
        .iter()
        .map(|p| norm(sub(*p, inside)))
        .fold(0.0, f64::max);
    // H lies in conv(source), since every facet vertex is an original point. All source
    // points lie in H dilated about `inside` by (1+slack/rho). Thus Hausdorff error
    // <= slack*R/rho, including the numerical plane guard. Minkowski +ball preserves it.
    let error = (slack.max(0.0) + 1e-11) * radius / rho;
    if !error.is_finite() || error > 0.00001 {
        return Err("SH001 support boundary exceeds ten micrometres");
    }
    // Convexity: every boundary vertex is in each outward halfspace within the same guard;
    // source containment above includes all of these vertices. Closed oriented sphere
    // topology plus strictly interior anchor forbids an open or inverted replacement.
    Ok(Hull {
        faces,
        error_metres: error,
    })
}
