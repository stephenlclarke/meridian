"""Replay exact SH016 aperture, collar, cockpit and original SH028 contact controls."""
from fractions import Fraction
import hashlib
import json
from math import isqrt
from pathlib import Path
import struct
import tomllib

from prepare_mole_miner_geometry import DETAIL, NATIVE, PIECE_COUNT, require


ROOT = Path(__file__).resolve().parents[1]
INTEGRATION = ROOT / 'integration/sh016-mole-miner'
FOLDER = 'assets/fleet/sh016/'
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


def support_indices(data):
    words = [word for word, in struct.iter_unpack('<H', data)]
    result = []
    cursor = 0
    for _ in range(PIECE_COUNT):
        end = cursor + 1 + words[cursor]
        result.append(words[cursor + 1:end])
        cursor = end
    require(cursor == len(words), 'wrong support stream length')
    return result


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


def verify():
    data = json.loads((INTEGRATION / 'FIXTURES.json').read_bytes())
    frozen = {}
    for name in (POINTS, MEMBERSHIP, FACES, RESOURCE, STATION):
        blob = (ROOT / name).read_bytes()
        require(hashlib.sha256(blob).hexdigest() == data['sourcePins'][name], 'changed contact input: ' + name)
        frozen[name] = blob
    require(data['guardMillimetres'] == GUARD_MM, 'wrong certificate guard')
    vertices = points(frozen[POINTS][16:])
    require(len(vertices) == DETAIL[0] + NATIVE[0], 'wrong source point count')
    piece_indices = support_indices(frozen[MEMBERSHIP])
    pieces = [[vertices[i] for i in group] for group in piece_indices]
    clearances = {}
    for route in data['routes']:
        triangle = [tuple(map(Fraction, p)) for p in route['triangle']]
        step = Fraction(route['translationZ'])
        sweep = triangle + [(x, y, z - step) for x, y, z in triangle]
        clearances[route['name']] = separating_planes(pieces, sweep, route['axes'])
    require(len(clearances) == 5 and min(clearances.values()) >= 2, 'a SH016 opening closes')
    head_contacts = data['headContacts']
    require(len(head_contacts) == 2 and all(any(i in part for part in piece_indices) for i in head_contacts),
            'missing original boring-head solid controls')

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
    face_materials = detail['faceMaterials'] + native['faceMaterials']
    shell = [[vertices[i] for i in face] for face in struct.iter_unpack('<3I', frozen[FACES])]
    require(all(model['materials'][m]['colour'] & 255 == 255 for model in (detail, native)
                for m in (0, 2)), 'body and crossmember must be opaque')
    joint_checks = []
    for joint in data['glazingJoints']:
        point = tuple(map(Fraction, joint['pointMetres']))
        counts = []
        for limits, material in zip(joint['faceRanges'], joint['materials'], strict=True):
            span = slice(*limits)
            require(set(face_materials[span]) == {material}, 'changed source joint material')
            counts.append(inside_y(point, shell[span]))
        joint_checks.append({'name': joint['name'], 'pointMetres': joint['pointMetres'],
                             'sourceFaceHitCountsAboveBelow': counts})
    return {'clearanceMillimetres': clearances, 'alignedMillimetres': aligned,
            'offsetStartMillimetres': before, 'sourceLipWitness': crossing,
            'sourceHeadContactsMetres': [list(map(float, vertices[i])) for i in head_contacts],
            'glazingJoints': joint_checks}


if __name__ == '__main__':
    print(json.dumps(verify(), indent=2))
