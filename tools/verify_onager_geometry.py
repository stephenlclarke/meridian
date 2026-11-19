"""Replay exact SH018 openings, hatch, cockpit and original SH028 contact controls."""
from fractions import Fraction
import hashlib
import json
from math import isqrt
from pathlib import Path
import struct
import tomllib

from prepare_onager_geometry import DETAIL, NATIVE, PIECE_COUNT, require


ROOT = Path(__file__).resolve().parents[1]
INTEGRATION = ROOT / 'integration/sh018-onager'
FOLDER = 'assets/fleet/sh018/'
POINTS = FOLDER + 'source.vertices'
MEMBERSHIP = FOLDER + 'source-piece-membership.bin'
FACES = FOLDER + 'source-triangles.bin'
RESOURCE = FOLDER + 'resource.toml'
STATION = 'assets/legacy/sh028.physics'
GUARD_MM = 21


def dot(a, b):
    return sum(x * y for x, y in zip(a, b, strict=True))


def points(data):
    return [tuple(Fraction(x) for x in p) for p in struct.iter_unpack('<3d', data)]


def bounds(shape):
    return tuple(tuple(f(axis) for axis in zip(*shape, strict=True)) for f in (min, max))


def separating_planes(pieces, obstacle, axes):
    distances = []
    for piece, axis in zip(pieces, axes, strict=True):
        require(all(type(v) is int for v in axis) and dot(axis, axis) > 0, 'invalid separating axis')
        gap = min(dot(axis, p) for p in obstacle) - max(dot(axis, p) for p in piece)
        require(gap > 0, 'opening sweep misses a separating plane')
        distances.append(isqrt((gap * 1000)**2 // dot(axis, axis)) - GUARD_MM)
    return min(distances)


def projected_weights(triangle, x, y):
    (ax, ay), (bx, by), (cx, cy) = triangle
    denominator = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)
    if denominator == 0:
        return None
    u = ((x - ax) * (cy - ay) - (y - ay) * (cx - ax)) / denominator
    v = ((bx - ax) * (y - ay) - (by - ay) * (x - ax)) / denominator
    return (1 - u - v, u, v)


def inside_y(point, triangles):
    x, y, z = point
    above, below = 0, 0
    for tri in triangles:
        weights = projected_weights([(a, c) for a, _, c in tri], x, z)
        if weights is None or min(weights) < 0:
            continue
        require(min(weights) > 0, 'cockpit parity ray hit a source edge')
        height = sum(w * vertex[1] for w, vertex in zip(weights, tri, strict=True))
        require(height != y, 'cockpit joint lies on a source face')
        above += int(height > y)
        below += int(height < y)
    require(above % 2 == below % 2 == 1, 'cockpit joint left the source volume')
    return (above, below)


def station_clearance(pieces, station, actor_x, actor_z):
    x = Fraction(actor_x)
    start, end = sorted(map(Fraction, actor_z))
    swept = [((x - hi[0], lo[1], start - hi[2]), (x - lo[0], hi[1], end - lo[2]))
             for lo, hi in map(bounds, pieces)]
    gap = min(max(max(c - b, a - d) for a, b, c, d in zip(lo, hi, low, high, strict=True))
              for lo, hi in swept for low, high in station)
    return gap * 1000 // 1 - GUARD_MM


def lip_intersection(triangle, vertex, actor_x, actor_z):
    z = triangle[0][2]
    require(all(v[2] == z for v in triangle), 'wrong SH028 lip plane')
    t = (z + vertex[2] - Fraction(actor_z[0])) / (Fraction(actor_z[1]) - Fraction(actor_z[0]))
    x, y = Fraction(actor_x) - vertex[0], vertex[1]
    weights = projected_weights([p[:2] for p in triangle], x, y)
    require(0 < t < 1 and weights is not None and min(weights) > 0, 'no strict source lip crossing')
    return {'fraction': float(t), 'pointMetres': list(map(float, (x, y, z))),
            'barycentrics': list(map(float, weights))}


def hatch_witness(faces, point, piece, spec):
    expected_plane = Fraction(13, 2)
    require(spec['faceIndices'] == [6, 7], 'wrong original C03 face indices')
    original_pair = [faces[i] for i in spec['faceIndices']]
    native_pair = [faces[DETAIL[1] + i] for i in spec['faceIndices']]
    require(native_pair == original_pair, 'original hatch pair differs between meshes')
    require(all(v[0] == expected_plane for tri in original_pair for v in tri),
            'changed original hatch plane')
    require(max(p[0] for p in piece) == expected_plane, 'chassis shifted the original hatch support')
    x, y, z = point
    gap = x - expected_plane
    require(0 < gap < Fraction(1, 50), 'hatch inner control leaves the rounded surface')
    weights = [projected_weights([p[1:] for p in tri], y, z) for tri in original_pair]
    require(any(w is not None and min(w) > 0 for w in weights), 'hatch control misses source interior')
    return {'originalFaceIndices': spec['faceIndices'], 'planeXMetres': float(expected_plane),
            'nominalRoundedStandOffMillimetres': 20, 'innerControlGapMillimetres': float(gap * 1000)}


def verify():
    data = json.loads((INTEGRATION / 'FIXTURES.json').read_bytes())
    frozen = {}
    for name in (POINTS, MEMBERSHIP, FACES, RESOURCE, STATION):
        blob = (ROOT / name).read_bytes()
        require(hashlib.sha256(blob).hexdigest() == data['sourcePins'][name], 'changed contact input: ' + name)
        frozen[name] = blob
    require(data['guardMillimetres'] == GUARD_MM, 'wrong certificate guard')
    vertices = points(frozen[POINTS][16:])
    require(len(vertices) == len(frozen[MEMBERSHIP]) == DETAIL[0] + NATIVE[0], 'wrong source point count')
    pieces = [[] for _ in range(PIECE_COUNT)]
    for slot, point in zip(frozen[MEMBERSHIP], vertices, strict=True):
        pieces[slot].append(point)
    clearances = {}
    for route in data['routes']:
        triangle = [tuple(map(Fraction, p)) for p in route['triangle']]
        travel = tuple(map(Fraction, route['travel']))
        sweep = triangle + [tuple(a - b for a, b in zip(p, travel, strict=True)) for p in triangle]
        clearances[route['name']] = separating_planes(pieces, sweep, route['axes'])
    require(len(clearances) == 6 and min(clearances.values()) >= 4, 'a SH018 opening closes')

    mesh = frozen[STATION]
    magic, vc, fc = struct.unpack_from('<8sII', mesh)
    require(magic == b'MPHYS001' and fc == 488, 'wrong station mesh')
    finish = 16 + 24 * vc
    xyz = points(mesh[16:finish])
    station = [[xyz[i] for i in face] for face in struct.iter_unpack('<3I', mesh[finish:])]
    boxes = list(map(bounds, station))
    dock = data['station']
    aligned = station_clearance(pieces, boxes, 0, dock['alignedZ'])
    before = station_clearance(pieces, boxes, dock['actorX'], [dock['lipCentreZ'][0]] * 2)
    require(min(aligned, before) >= 978, 'wrong SH028 clear controls')
    crossing = lip_intersection(station[dock['face']], vertices[dock['nativeGlobalVertex']],
                                dock['actorX'], dock['lipCentreZ'])

    original = tomllib.loads(frozen[RESOURCE].decode())
    detail, native = original['design'], original['nativeDesign']
    materials = detail['faceMaterials'] + native['faceMaterials']
    faces = [[vertices[i] for i in f] for f in struct.iter_unpack('<3I', frozen[FACES])]
    joint = data['cockpitJoint']
    point = tuple(map(Fraction, joint['pointMetres']))
    counts = []
    for i, limits in enumerate(joint['faceRanges']):
        model = detail if i < 2 else native
        span = slice(*limits)
        ids = set(materials[span])
        if i % 2 == 0:
            require(ids <= {0, 2, 3} and all(model['materials'][m]['colour'] & 255 == 255 for m in ids),
                    'wrong original opaque cabin body')
        else:
            require(ids == {5} and model['materials'][5]['name'] == 'glazing', 'wrong original glazing')
        counts.append(inside_y(point, faces[span]))
    hatch = data['hatch']
    hatch_result = hatch_witness(faces, tuple(map(Fraction, hatch['innerProbeMetres'])),
                                 pieces[hatch['ownerPiece']], hatch)
    return {'clearanceMillimetres': clearances, 'alignedMillimetres': aligned,
            'offsetStartMillimetres': before, 'sourceLipWitness': crossing,
            'sourcePodContactsMetres': [list(map(float, vertices[i])) for i in data['sourcePodContacts']],
            'cockpitJoint': {'pointMetres': joint['pointMetres'], 'sourceFaceHitCountsAboveBelow': counts},
            'hatch': hatch_result}


if __name__ == '__main__':
    print(json.dumps(verify(), indent=2))
