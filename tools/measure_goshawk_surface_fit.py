"""Source-only SH008 envelope audit. Run with NumPy, SciPy, trimesh and rtree."""
import argparse
from collections import Counter
from fractions import Fraction
import hashlib
from itertools import pairwise
import json
from math import isqrt
from pathlib import Path
import struct

import numpy as np
from scipy.spatial import ConvexHull
import trimesh


RADIUS = 0.02
GRID = 2**20
DEPTH = 6
ASSETS = Path(__file__).resolve().parents[1] / 'assets/fleet/sh008'


def load(folder):
    manifest = json.loads((folder / 'MANIFEST.json').read_bytes())
    inputs = {}
    for key in ['source.vertices', 'source-triangles.bin', 'source-piece-membership.bin',
                'COMPONENT-PIECES.json']:
        blob = (folder / key).read_bytes()
        pin = manifest['files'][key]
        assert len(blob) == pin['bytes'] and hashlib.sha256(blob).hexdigest() == pin['sha256'], key
        inputs[key] = blob
    header = struct.pack('<8sII', b'MSH008V1', 2072, 536)
    assert inputs['source.vertices'].startswith(header)
    vertices = np.frombuffer(inputs['source.vertices'], dtype='<f8', offset=len(header)).reshape(-1, 3)
    faces = np.frombuffer(inputs['source-triangles.bin'], dtype='<u4').reshape(-1, 3)
    membership = np.frombuffer(inputs['source-piece-membership.bin'], dtype='u1')
    names = json.loads(inputs['COMPONENT-PIECES.json'])['pieces']
    assert vertices.shape == (2608, 3) and faces.shape == (4448, 3)
    assert len(membership) == len(vertices) and len(names) == 69
    assert np.isfinite(vertices).all() and np.max(np.abs(vertices)) < 32
    assert set(membership) == set(range(len(names)))
    assert np.all(faces[:3484] < 2072) and np.all(faces[3484:] >= 2072)
    assert np.all(membership[faces] == membership[faces[:, :1]])
    return manifest, names, vertices, faces, membership


def dot(x, y):
    return sum(a * b for a, b in zip(x, y))


def sub(x, y):
    return tuple(a - b for a, b in zip(x, y))


def cross(x, y):
    a, b, c = x
    d, e, f = y
    return b * f - c * e, c * d - a * f, a * e - b * d


def dyadic(vectors):
    ratios = [[float(x).as_integer_ratio() for x in p] for p in vectors]
    scale = max(d for p in ratios for _, d in p)
    return [tuple(n * (scale // d) for n, d in p) for p in ratios], scale


def split(triangles, integer=False):
    a, b, c = triangles.transpose(1, 0, 2)
    if integer:
        ab, ac, bc = (a + b) // 2, (a + c) // 2, (b + c) // 2
    else:
        ab, ac, bc = (a + b) / 2, (a + c) / 2, (b + c) / 2
    return np.concatenate([np.stack(t, axis=1) for t in
                           [(a, ab, ac), (ab, b, bc), (ac, bc, c), (ab, bc, ac)]])


def nearest(mesh, positions):
    batches = [mesh.nearest.on_surface(positions[i:i + 300]) for i in range(0, len(positions), 300)]
    return tuple(np.concatenate(columns) for columns in zip(*batches))


def audit_fan(vertices):
    """Check exact radial coverage and the outward error of the triangulated fan."""
    hull = ConvexHull(vertices)
    points, scale = dyadic(vertices)
    center_sum = tuple(map(sum, zip(*points)))
    edges = Counter()
    radial_error = Fraction(0)
    support_faces = []
    for face in hull.simplices:
        a, b, c = map(int, face)
        normal = cross(sub(points[b], points[a]), sub(points[c], points[a]))
        h = dot(normal, points[a])
        from_center = h * len(points) - dot(normal, center_sum)
        assert from_center != 0
        if from_center < 0:
            b, c = c, b
            normal = tuple(-x for x in normal)
            h, from_center = -h, -from_center
        for edge in pairwise((a, b, c, a)):
            edges[edge] += 1
        excess = max(dot(normal, p) for p in points) - h
        if excess == 0:
            unit = np.asarray(normal, dtype=float)
            unit /= np.linalg.norm(unit)
            support_faces.append(((a, b, c), unit))
        radial_error = max(radial_error, Fraction(excess * len(points), from_center))
    assert all(v == edges[(b, a)] == 1 for (a, b), v in edges.items())
    assert len(support_faces) > 0
    # The positive, closed radial projection has nonzero degree, hence covers the sphere.
    # Any farther hull point on the same ray obeys the exact maximum face projection.
    # Bound each ray's length by the piece's L1 bounding-box diameter.
    l1_diameter = sum(max(col) - min(col) for col in zip(*points))
    assert radial_error * l1_diameter / scale < Fraction(1, 1_000_000)
    normals = hull.equations[:, :3]
    offsets = (vertices @ normals.T).max(axis=0)
    return {
        'vertices': vertices,
        'triangles': vertices[hull.simplices],
        'supportFaces': support_faces,
        'normals': normals,
        'offsets': offsets,
        'min': vertices.min(axis=0) - RADIUS,
        'max': vertices.max(axis=0) + RADIUS,
    }


def exposed_samples(piece, others):
    face_ids = np.asarray([face for face, _ in piece['supportFaces']])
    triangles = piece['vertices'][face_ids]
    normals = np.array([normal for _, normal in piece['supportFaces']])
    face_grid = split(split(triangles))
    points = np.concatenate([
        face_grid.mean(axis=1) + RADIUS * np.tile(normals, (16, 1)),
        (triangles + RADIUS * normals[:, None, :]).reshape(-1, 3),
    ])
    keep = np.ones(len(points), dtype=bool)
    for other in others:
        if other is piece:
            continue
        candidates = keep & np.all((points >= other['min'] - 1e-8) & (points <= other['max'] + 1e-8), axis=1)
        keep[candidates] = np.any(points[candidates] @ other['normals'].T > other['offsets'] + RADIUS + 1e-8,
                                  axis=1)
    # Integer barycentrics of the same sixteen patch centroids and three original vertices.
    lattice = 4 * np.eye(3, dtype=np.int64)[None, :, :]
    centroid_weights = split(split(lattice, integer=True), integer=True).sum(axis=1)
    weights = np.concatenate((np.repeat(centroid_weights, len(triangles), axis=0),
                              np.tile(12 * np.eye(3, dtype=np.int64), (len(triangles), 1))))
    supports = np.concatenate((np.tile(face_ids, (16, 1)), np.repeat(face_ids, 3, axis=0)))
    return points[keep], supports[keep], weights[keep]


def worst_sample(mesh, points):
    if not len(points):
        return None, None
    nearest_points, distances, faces = nearest(mesh, points)
    i = int(np.argmax(distances))
    distance = float(distances[i])
    return {'metres': distance, 'beyondPaddingMetres': max(0.0, distance - RADIUS),
            'point': points[i].tolist(), 'closestPoint': nearest_points[i].tolist(), 'face': int(faces[i])}, i


def patch_upper_squared(mesh, grid_vertices, patches):
    # Floating nearest queries only nominate a triangle and barycentric witnesses.
    # The bound is calculated from actual integer convex combinations on that triangle.
    local = patches.reshape(-1, 3)
    _, _, face_ids = nearest(mesh, patches.mean(axis=1) / GRID)
    faces = np.repeat(face_ids, 3)
    triangles = mesh.triangles[faces]
    close = trimesh.triangles.closest_point(triangles, local / GRID)
    weights = trimesh.triangles.points_to_barycentric(triangles, close)[:, :2]
    weights = np.nan_to_num(weights, nan=0.0).clip(0.0, 1.0)
    weights[:, 1] = np.minimum(weights[:, 1], 1.0 - weights[:, 0])
    dyadic_weights = np.floor(weights * GRID).astype(np.int64)
    dyadic_weights = np.column_stack((dyadic_weights, GRID - dyadic_weights.sum(axis=1)))
    assert np.all(dyadic_weights >= 0)
    witnesses = np.einsum('ijk,ij->ik', grid_vertices[mesh.faces[faces]], dyadic_weights) // GRID
    delta = witnesses - local
    return np.einsum('ij,ij->i', delta, delta).reshape(-1, 3).max(axis=1)


def certified_upper(mesh, piece, target_mm):
    grid_vertices = np.rint(mesh.vertices * GRID).astype(np.int64)
    patches = np.rint(piece['triangles'] * GRID).astype(np.int64)
    upper = 0
    for depth in range(DEPTH + 1):
        if not len(patches):
            break
        squares = patch_upper_squared(mesh, grid_vertices, patches)
        # A requested cutoff only controls refinement. Terminal patches always report their bound.
        # Budget 2 grid units per (depth + 4) for both grids and witness quantization.
        # The fan audit allows <1 um radial error. Round that and the 2 cm radius outward.
        margin_grid = 2 * (depth + 4) + 2 + (GRID + 49) // 50
        cutoff_grid = target_mm * GRID // 1000 - margin_grid
        done = squares <= cutoff_grid**2 if cutoff_grid >= 0 else np.zeros(len(squares), dtype=bool)
        if depth == DEPTH:
            done[:] = True
        if np.any(done):
            max_squared = int(squares[done].max())
            upper = max(upper, isqrt(max_squared) + 1 + margin_grid)
        patches = split(patches[~done], integer=True)
    # The maximum of distances to one fixed triangle at patch corners bounds the complete patch.
    return (upper * 1000 + GRID - 1) // GRID


def closest_face_record(mesh, point):
    return worst_sample(mesh, np.asarray([point]))[0]


def farther_than(point, a, b, c, limit_squared):
    """Exact squared point/triangle comparison over the original binary64 values."""
    ab, ac, ap = sub(b, a), sub(c, a), sub(point, a)
    aa, bb, cc = dot(ab, ab), dot(ab, ac), dot(ac, ac)
    ap_ab, ap_ac = dot(ap, ab), dot(ap, ac)
    denom = aa * cc - bb * bb
    u, v = cc * ap_ab - bb * ap_ac, aa * ap_ac - bb * ap_ab
    if denom > 0 and u >= 0 and v >= 0 and u + v <= denom:
        height = dot(ap, cross(ab, ac))
        return height * height > limit_squared * denom
    for start, end in [(a, b), (b, c), (c, a)]:
        ray, offset = sub(end, start), sub(point, start)
        length, advance = dot(ray, ray), dot(ray, offset)
        if advance <= 0:
            if dot(offset, offset) <= limit_squared:
                return False
        elif advance >= length:
            offset = sub(point, end)
            if dot(offset, offset) <= limit_squared:
                return False
        elif dot(offset, offset) * length - advance**2 <= limit_squared * length:
            return False
    return True


def integer_source(vertices, meshes):
    exact, scale = dyadic(vertices)
    return exact, scale, {
        name: [tuple(exact[int(i) + offset] for i in face) for face in mesh.faces]
        for name, (offset, mesh) in meshes.items()
    }


def verify_lower(point, model, scale, mm):
    squared = (Fraction(mm, 1000) * scale)**2
    assert all(farther_than(point, *face, squared) for face in model), mm


def certify_exposed_sample(record, index, samples, piece, pieces, faces):
    xyz, supports, weights = samples
    sample = xyz[index]
    ids = supports[index]
    ws = weights[index]
    triangle = piece['vertices'][ids]
    exact, scale = dyadic(np.vstack((triangle, sample)))
    a, b, c, reported = exact
    normal = cross(sub(b, a), sub(c, a))
    # The oriented face was checked against all original member points in audit_fan.
    # Enclose the rounded face point by comparing each coordinate with an interval
    # for the exact normalized outward offset of its rational barycentric base.
    root_grid = 2**20
    root_lower = Fraction(isqrt(dot(normal, normal) * root_grid**2), root_grid)
    root_upper = root_lower + Fraction(1, root_grid)
    uncertainty = Fraction(1, 1_000_000)
    for i, component in enumerate(normal):
        base = Fraction(sum(int(w) * p[i] for w, p in zip(ws, (a, b, c))), 12 * scale)
        interval = sorted([base + Fraction(component, 50) / length
                           for length in (root_lower, root_upper)])
        assert max(abs(Fraction(reported[i], scale) - bound) for bound in interval) < uncertainty / 2

    for other in pieces:
        if other is piece:
            continue
        vertices = other['vertices']
        axes = np.concatenate((other['normals'], np.eye(3), -np.eye(3)))
        direction = axes[np.argmax(sample @ axes.T - (vertices @ axes.T).max(axis=0))]
        axis = tuple(int(n) for n in np.rint(direction * GRID))
        points, axis_scale = dyadic(np.vstack((sample, vertices)))
        gap = dot(axis, points[0]) - max(dot(axis, p) for p in points[1:])
        assert gap > 0 and gap**2 > ((Fraction(1, 50) + uncertainty) * axis_scale)**2 * dot(axis, axis)

    exact_sample, sample_scale = dyadic(sample[None, :])
    # The sampled nearest distance only chooses a lower-bound threshold. Every
    # triangle must be farther by the additional uncertainty of the rounded position.
    lower_mm = int(record['metres'] * 1000) - 1
    triangles, triangle_scale = faces
    shared_scale = max(sample_scale, triangle_scale)
    point = tuple(x * (shared_scale // sample_scale) for x in exact_sample[0])
    factor = shared_scale // triangle_scale
    if factor != 1:
        triangles = [tuple(tuple(x * factor for x in p) for p in face) for face in triangles]
    limit_squared = ((Fraction(lower_mm, 1000) + uncertainty) * shared_scale)**2
    assert all(farther_than(point, *face, limit_squared) for face in triangles)
    record['certifiedLowerMillimetres'] = lower_mm
    record['supportFacetGlobalVertices'] = piece['sourceIds'][ids].tolist()
    record['supportFacetWeightNumeratorsOver12'] = ws.tolist()


def channel_control(pieces):
    # Relative obstacle endpoints from the accepted forebody sweep. Convexity covers the sweep.
    travel = np.array([[x, y, z] for z in (4., 14.) for x, y in [(-.15, -1.65), (.15, -1.65), (0., -1.35)]])
    slacks = []
    for piece in pieces:
        pts = piece['vertices']
        axes = np.concatenate((piece['normals'], -piece['normals'], np.eye(3), -np.eye(3)))
        axis = axes[np.argmax((travel @ axes.T).min(axis=0) - (pts @ axes.T).max(axis=0))]
        # Certify with integer plane coefficients and exact projections. The authored decimals
        # for this fixture differ from their binary64 representation by less than 1 um.
        axis = tuple(int(n) for n in np.rint(axis * GRID))
        both, scale = dyadic(np.concatenate((travel, pts)))
        gap = min(dot(axis, p) for p in both[:6]) - max(dot(axis, p) for p in both[6:])
        length_squared = dot(axis, axis)
        lower_mm = (isqrt(gap**2 // length_squared) * 1000) // scale - 21
        assert gap > 0 and lower_mm >= 740
        slacks.append(lower_mm)
    return {'minimumCertifiedClearanceMillimetres': min(slacks), 'limitingPiece': int(np.argmin(slacks)),
            'relativeObstacleEndpointsMetres': travel.tolist()}


def well_control(vertices, exact, scale, meshes, model_triangles, name, offset):
    mouth = vertices[offset:offset + 4]
    floor = vertices[offset + 4:offset + 8]
    center = mouth.mean(axis=0)
    # Exact mean of four source vertices lies in the current support. Scale all triangle
    # coordinates by four to compare the rational mouth centre without rounding.
    exact_center = tuple(map(sum, zip(*exact[offset:offset + 4])))
    distances = {}
    for source, (_, mesh) in meshes.items():
        record = closest_face_record(mesh, center)
        del record['beyondPaddingMetres']
        lower_mm = int(record['metres'] * 1000) - 1
        faces = [tuple(tuple(4 * v for v in p) for p in face) for face in model_triangles[source]]
        verify_lower(exact_center, faces, scale * 4, lower_mm)
        record['certifiedLowerMillimetres'] = lower_mm
        distances[source] = record
    normal = np.cross(mouth[1] - mouth[0], mouth[3] - mouth[0])
    normal /= np.linalg.norm(normal)
    return {'source': name, 'startVertex': offset - meshes[name][0], 'mouthPointMetres': center.tolist(),
            'mouthEdgeMetres': [float(np.linalg.norm(mouth[i] - mouth[0])) for i in (1, 3)],
            'projectedFloorDepthMetres': float(abs((floor.mean(axis=0) - center) @ normal)),
            'mouthCenterToSources': distances,
            'insideCurrentEnvelope': 'Certified by equal convex weights on four assigned original rim vertices.'}


def well_decomposition_control(vertices, exact, scale, faces, membership, names, original_pieces):
    # A source-preserving geometric feasibility check. It does not change the shipped partition.
    starts = [1288, 2072 + 308, 236, 2072 + 80]
    prop = dict(zip(names, original_pieces))
    replaced = []
    for start in starts:
        slot = membership[start]
        family = names[slot]
        del prop[family]
        replaced.append(family)
        component = np.flatnonzero(membership == slot)
        groups = [(f'wall-{i}', start + np.array([i, j, i + 4, j + 4, i + 8, j + 8]))
                  for i, j in [(0, 1), (1, 2), (2, 3), (3, 0)]]
        groups.append(('back-and-floor', np.arange(start + 4, start + 12)))
        struts = component[12:]
        assert len(struts) in (0, 24)
        groups.extend((f'strut-{i // 8}', struts[i:i + 8]) for i in range(0, len(struts), 8))
        # Each original triangle stays completely enclosed by at least one replacement.
        # Every replacement uses a subset of its current piece, including separate struts.
        groups_sets = [set(ids) for _, ids in groups]
        source = set(component)
        assert all(group <= source for group in groups_sets)
        assert all(any(set(face) <= group for group in groups_sets)
                   for face in faces[membership[faces[:, 0]] == slot])
        for name, ids in groups:
            prop[f'{family}/{name}'] = audit_fan(vertices[ids])

    certified = []
    for start in starts:
        center = vertices[start:start + 4].mean(axis=0)
        center4 = tuple(map(sum, zip(*exact[start:start + 4])))
        separations = {}
        for name, piece in prop.items():
            pts = piece['vertices']
            axes = np.concatenate((piece['normals'], np.eye(3), -np.eye(3)))
            direction = axes[np.argmax(center @ axes.T - (pts @ axes.T).max(axis=0))]
            axis = tuple(int(n) for n in np.rint(direction * GRID))
            integer_points, point_scale = dyadic(pts)
            assert scale % point_scale == 0
            gap = dot(axis, center4) - max(dot(axis, p) for p in integer_points) * (4 * scale // point_scale)
            assert gap > 0
            unrounded_mm = isqrt(gap**2 // dot(axis, axis)) * 1000 // (4 * scale)
            separations[name] = unrounded_mm - 20
        limiting_piece = min(separations, key=separations.get)
        assert separations[limiting_piece] > 0
        certified.append({'globalRimStartVertex': start, 'limitingPiece': limiting_piece,
                          'certifiedMouthClearanceMillimetres': separations[limiting_piece]})
    return {
        'status': 'SOURCE_ONLY_FEASIBILITY;NOT_RUNTIME_GEOMETRY',
        'prototypePieces': len(prop),
        'replacedPieces': replaced,
        'sourceContainment': 'Every original triangle of the four replaced pieces fits one replacement formed from its original vertices.',
        'controls': certified,
    }


def measure(folder):
    manifest, names, xyz, faces, membership = load(folder)
    models = {name: (offset, trimesh.Trimesh(xyz[offset:end], faces[rows] - offset, process=False))
              for name, offset, end, rows in [('detailed', 0, 2072, slice(3484)),
                                                ('nativeCandidate', 2072, 2608, slice(3484, None))]}
    pieces = []
    for i in range(len(names)):
        ids = np.flatnonzero(membership == i)
        points, first = np.unique(xyz[ids], axis=0, return_index=True)
        piece = audit_fan(points)
        piece['sourceIds'] = ids[first]
        pieces.append(piece)
    exact, scale, exact_faces = integer_source(xyz, models)
    cross_source = {}
    for name, other, rows in [('detailed', 'nativeCandidate', slice(2072)),
                              ('nativeCandidate', 'detailed', slice(2072, None))]:
        positions = xyz[rows]
        closest, distances, nearest_faces = nearest(models[other][1], positions)
        output = []
        for index in np.argsort(-distances, kind='stable')[:8]:
            global_index = int(index) + models[name][0]
            lower_mm = int(distances[index] * 1000) - 1
            verify_lower(exact[global_index], exact_faces[other], scale, lower_mm)
            output.append({'sourceVertex': int(index), 'piece': names[membership[global_index]],
                           'pointMetres': xyz[global_index].tolist(), 'nearestFace': int(nearest_faces[index]),
                           'closestPointMetres': closest[index].tolist(), 'sampledMetres': float(distances[index]),
                           'certifiedLowerMillimetres': lower_mm})
        cross_source[name + 'VerticesTo' + other[0].upper() + other[1:]] = output
    result = []
    witnesses = {source: [] for source in models}
    targets = {'detailed': 580, 'nativeCandidate': 1110}
    for name, piece in zip(names, pieces):
        samples = exposed_samples(piece, pieces)
        comparisons = {}
        for source, (_, mesh) in models.items():
            record, i = worst_sample(mesh, samples[0])
            comparisons[source] = {'largestExposedSample': record,
                                   'fullPieceSurfaceUpperMillimetres': certified_upper(mesh, piece, targets[source])}
            if record is not None:
                witnesses[source].append((record, i, samples, piece))
        result.append({'piece': name, 'exposedSamples': len(samples[0]), 'sourceComparisons': comparisons})
    for source, records in witnesses.items():
        largest = sorted(records, key=lambda row: -row[0]['metres'])[:2]
        for record, i, samples, piece in largest:
            certify_exposed_sample(record, i, samples, piece, pieces, (exact_faces[source], scale))
    return {
        'sourceRevision': manifest['sourceRevision'],
        'sourcePins': manifest['files'],
        'nominalPaddingMetres': RADIUS,
        'method': {
            'exposure': 'Exact supporting facets, rounded outward by 2 cm. Hull-plane separation with a 10 nm numerical guard removes potentially occluded points. A zero count is unresolved.',
            'samples': 'Nearest Euclidean distance to each source triangle set independently. The two largest per source have exact exposed lower certificates. Samples do not certify a maximum.',
            'upper': 'Exact dyadic fan checks, fixed-point patch/triangle witnesses and outward-rounded geometric budgets cover even unsampled edge/vertex offsets and occluded piece surfaces.',
            'mandatoryVertices': 'Exact point-triangle threshold checks against every opposite-source triangle. These bound occupied source points, independent of exposed-boundary sampling.',
        },
        'channelControl': channel_control(pieces),
        'wellControls': [
            well_control(xyz, exact, scale, models, exact_faces, 'detailed', 1288),
            well_control(xyz, exact, scale, models, exact_faces, 'nativeCandidate', 2072 + 308),
            well_control(xyz, exact, scale, models, exact_faces, 'detailed', 236),
        ],
        'wellDecompositionPrototype': well_decomposition_control(xyz, exact, scale, faces, membership, names, pieces),
        'mandatoryCrossSourceVertices': cross_source,
        'pieces': result,
    }


def main():
    if not __debug__:
        raise RuntimeError("The geometric certificate checks require Python assertions")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ASSETS)
    args = parser.parse_args()
    print(json.dumps(measure(args.assets), indent=2, allow_nan=False))


if __name__ == '__main__':
    main()
