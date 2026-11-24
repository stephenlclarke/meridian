"""Replay the frozen Eagle trench and structural witnesses with exact arithmetic."""
from fractions import Fraction
import json
from math import isqrt
import struct
import sys
import tomllib

from prepare_eagle_cruiser_geometry import ASSETS, DETAIL, NATIVE, PIECE_COUNT, ROOT, document, prepare, require

INTEGRATION = ROOT / 'integration/sh043-eagle-cruiser'
GUARD_MM = 21


def dot(a, b):
    return sum(x * y for x, y in zip(a, b, strict=True))


def points(data):
    return [tuple(Fraction(x) for x in p) for p in struct.iter_unpack('<3d', data)]


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
        require(min(weights) > 0, 'joint parity ray hit a source edge')
        height = sum(w * vertex[1] for w, vertex in zip(weights, tri, strict=True))
        require(height != y, 'joint lies on a source face')
        above += int(height > y)
        below += int(height < y)
    require(above % 2 == below % 2 == 1, 'joint left the source volume')
    return (above, below)


def verify():
    data = json.loads((INTEGRATION / 'CONTACT-CANDIDATE.json').read_bytes())
    encoded = prepare(ASSETS)
    for path in ('source.vertices', 'source-piece-membership.bin', 'source-triangles.bin'):
        require((ASSETS / path).read_bytes() == encoded[path], 'changed Eagle contact input: ' + path)
    vertices = points(encoded['source.vertices'][16:])
    require(len(vertices) == DETAIL[0] + NATIVE[0], 'wrong exact source vertex count')
    pieces = [[] for _ in range(PIECE_COUNT)]
    for slot, point in zip(encoded['source-piece-membership.bin'], vertices, strict=True):
        pieces[slot].append(point)
    routes = {}
    for route in data['routes']:
        triangle = [tuple(map(Fraction, p)) for p in route['triangle']]
        travel = tuple(map(Fraction, route['travel']))
        sweep = triangle + [tuple(a - b for a, b in zip(point, travel, strict=True)) for point in triangle]
        clearance = separating_planes(pieces, sweep, route['axes'])
        require(clearance == route['clearanceMillimetres'] and clearance >= 950, 'changed Eagle witness')
        routes[route['name']] = clearance
    require(len(routes) == 7, 'wrong Eagle trench route count')
    resource = tomllib.loads((ASSETS / 'resource.toml').read_text())
    detail, native = resource['design'], resource['nativeDesign']
    require(all(model['materials'][m]['colour'] & 255 == 255 for model in (detail, native)
                for m in (0, 1, 2, 3)), 'structural supports must use opaque source materials')
    material = detail['faceMaterials'] + native['faceMaterials']
    shoulder_wall = ({1, 2, 3}, {0}, {1}, {0})
    shoulder_support = ({1, 2, 3}, {2}, {1}, {2})
    bridge = ({0, 2, 3}, {5}, {0}, {5})
    expected = [shoulder_wall, shoulder_support, shoulder_wall, shoulder_support, bridge]
    faces = [[vertices[i] for i in face] for face in struct.iter_unpack('<3I', encoded['source-triangles.bin'])]
    for joint, materials in zip(data['originalJoints'], expected, strict=True):
        point = tuple(map(Fraction, joint['pointMetres']))
        counts = []
        for span, expected_materials in zip(joint['faceRanges'], materials, strict=True):
            section = slice(*span)
            require(set(material[section]) == expected_materials, 'changed source joint material')
            counts.append(list(inside_y(point, faces[section])))
        require(counts == joint['faceHitCountsAboveBelow'], 'changed Eagle joint witness')
    return {'clearanceMillimetres': routes, 'originalJoints': data['originalJoints']}


if __name__ == '__main__':
    sys.stdout.buffer.write(document(verify()))
