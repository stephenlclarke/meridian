"""Replay SH009 swept-clearance fixtures over exact calibrated binary64 coordinates."""
from fractions import Fraction
import hashlib
import json
from math import isqrt
from pathlib import Path
import struct


ROOT = Path(__file__).resolve().parents[1]
FIXTURES = 'integration/sh009-goshawk-mk2/SH009-FIXTURES.json'
VERTICES = 'assets/fleet/sh009/source.vertices'
MEMBERSHIP = 'assets/fleet/sh009/source-piece-membership.bin'
STATION = 'assets/legacy/sh028.physics'
PADDING_MM = 21


def require(test, reason):
    if not test:
        raise ValueError(reason)


def dot(axis, point):
    return sum(a * p for a, p in zip(axis, point, strict=True))


def points(raw):
    return [tuple(Fraction(x) for x in p) for p in struct.iter_unpack('<3d', raw)]


def swept_planes_mm(pieces, obstacle, axes):
    lower = []
    for piece, axis in zip(pieces, axes, strict=True):
        require(all(type(x) is int for x in axis) and dot(axis, axis) > 0, 'invalid axis')
        gap = min(dot(axis, point) for point in obstacle) - max(dot(axis, point) for point in piece)
        require(gap > 0, 'axis supplies no separating plane')
        lower.append(isqrt((gap * 1000)**2 // dot(axis, axis)) - PADDING_MM)
    return min(lower)


def bounds(data):
    return [tuple(op(axis) for axis in zip(*data, strict=True)) for op in (min, max)]


def station_boxes_mm(pieces, triangle_bounds, x, z_pair):
    # The inbound half-turn reverses X and Z. Linear motion extrema are at the endpoints.
    x = Fraction(x)
    z0, z1 = sorted(Fraction(z) for z in z_pair)
    boxes = [
        ((x - h[0], low[1], z0 - h[2]), (x - low[0], h[1], z1 - low[2]))
        for low, h in map(bounds, pieces)
    ]
    gap, face = min(
        (max(max(a - d, c - b) for a, b, c, d in zip(lo, hi, low, high, strict=True)), i)
        for lo, hi in boxes for i, (low, high) in enumerate(triangle_bounds)
    )
    return {'guardedClearanceMillimetres': gap * 1000 // 1 - PADDING_MM, 'limitingFace': face}


def lip_contact(station, indices, local, x, z_pair):
    a, b, c = [station[i] for i in indices]
    require(a[2] == b[2] == c[2], 'lip fixture is not in a station Z plane')
    x, y = Fraction(x) - local[0], local[1]
    t = (a[2] + local[2] - Fraction(z_pair[0])) / (Fraction(z_pair[1]) - Fraction(z_pair[0]))
    cross = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    require(0 < t < 1 and cross != 0, 'lip fixture does not cross its station face plane')
    u = ((x - a[0]) * (c[1] - a[1]) - (y - a[1]) * (c[0] - a[0])) / cross
    v = ((b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])) / cross
    require(min(1 - u - v, u, v) > 0, 'rail trajectory misses the strict lip interior')
    return {'triangleVertices': list(indices), 'intersectionMetres': list(map(float, (x, y, a[2]))),
            'sweepFraction': float(t), 'barycentric': list(map(float, (1 - u - v, u, v)))}


def verify(root=ROOT):
    data = json.loads((root / FIXTURES).read_bytes())
    require(data['hull'] == 'SH009' and data['station'] == 'SH028', 'wrong hull or station')
    require(data['paddingForProofMetres'] == .021, 'wrong guarded padding')
    blobs = {}
    for name in (VERTICES, MEMBERSHIP, STATION):
        raw = (root / name).read_bytes()
        require(hashlib.sha256(raw).hexdigest() == data['sourcePins'][name], 'wrong source pin: ' + name)
        blobs[name] = raw
    vertices = points(blobs[VERTICES][16:])
    slots = blobs[MEMBERSHIP]
    require(len(vertices) == len(slots) == 2672 and set(slots) == set(range(73)), 'wrong hull shape')
    pieces = [[] for _ in range(73)]
    for slot, point in zip(slots, vertices, strict=True):
        pieces[slot].append(point)

    channel = data['channel']
    triangle = [tuple(map(Fraction, p)) for p in channel['triangleMetres']]
    offset = Fraction(channel['hullTranslationZMetres'])
    swept = triangle + [(x, y, z - offset) for x, y, z in triangle]
    channel_margin = swept_planes_mm(pieces, swept, channel['separatingAxes'])
    require(channel_margin >= 1460, 'forebody channel clearance fell below its bound')
    require(list(map(float, vertices[channel['armsContactVertex']])) == channel['armsContactMetres'],
            'wrong drive arm witness')

    ventral = []
    for row in data['ventralMergeControls']:
        a, b = row['sourceVertices']
        require(slots[a] != slots[b], 'ventral bracket and runner were merged')
        midpoint = tuple((x + y) / 2 for x, y in zip(vertices[a], vertices[b], strict=True))
        require(list(map(float, midpoint)) == row['pointMetres'], 'wrong ventral gap witness')
        ventral.append(swept_planes_mm(pieces, [midpoint], row['separatingAxes']))
    require(len(ventral) == 2 and min(ventral) >= 613, 'ventral clearance lost')

    magic, count, face_count = struct.unpack_from('<8sII', blobs[STATION])
    require(magic == b'MPHYS001' and face_count == 488, 'wrong station mesh')
    end = 16 + count * 24
    station = points(blobs[STATION][16:end])
    faces = list(struct.iter_unpack('<3I', blobs[STATION][end:]))
    boxes = [bounds([station[i] for i in face]) for face in faces]
    contact = data['stationContact']
    z_pair, x = contact['lipActorZMetres'], contact['lipActorXMetres']
    exterior = station_boxes_mm(pieces, boxes, x, [z_pair[0]] * 2)
    aligned = station_boxes_mm(pieces, boxes, 0.0, contact['alignedActorZMetres'])
    hit = lip_contact(station, faces[contact['face']], vertices[contact['hullVertex']], x, z_pair)
    require(exterior['guardedClearanceMillimetres'] >= 978, 'lip start lacks strict clearance')
    require(aligned['guardedClearanceMillimetres'] >= 44_400, 'aligned station route lost clearance')
    return {'channelClearanceMillimetres': channel_margin, 'ventralClearanceMillimetres': ventral,
            'lipStart': exterior, 'alignedStationSweep': aligned, 'lipIntersection': hit}


if __name__ == '__main__':
    print(json.dumps(verify(), indent=2))
