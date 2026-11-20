"""Replay SH012 geometric witnesses without a game build or science dependencies."""
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import struct

from prepare_sable_liner_geometry import require


ROOT = Path(__file__).resolve().parents[1]
ASSET = 'assets/fleet/sh012'
FIXTURE = 'integration/sh012-sable-liner/FIXTURES.json'
STATION = 'assets/legacy/sh028.physics'
GUARD_MM = 21


def vertices(raw):
    return [tuple(Fraction(v) for v in xyz) for xyz in struct.iter_unpack('<3d', raw)]


def bounds(points):
    return tuple(tuple(op(coords) for coords in zip(*points, strict=True)) for op in (min, max))


def coordinate_margin(a, b):
    a_min, a_max = a
    b_min, b_max = b
    return max(max(c - y, x - d) for x, y, c, d in zip(a_min, a_max, b_min, b_max, strict=True))


def rounded_mm(gap):
    return gap * 1000 // 1 - GUARD_MM


def read_supports(data, points):
    magic, count = struct.unpack_from('<8sI', data)
    require(magic == b'MSH12PC1' and count == 47, 'wrong SH012 shield format')
    offset = 12
    pieces = []
    for _ in range(count):
        size = struct.unpack_from('<I', data, offset)[0]
        offset += 4
        ids = struct.unpack_from(f'<{size}I', data, offset)
        offset += 4 * size
        require(len(set(ids)) == size and all(i < len(points) for i in ids), 'bad SH012 support indices')
        pieces.append([points[i] for i in ids])
    require(offset == len(data), 'trailing SH012 shield data')
    return pieces


def lip_witness(station, face, local, x, start, end):
    a, b, c = [station[i] for i in face]
    require(a[2] == b[2] == c[2], 'lip fixture face is not planar in Z')
    x, y = Fraction(x) - local[0], local[1]
    t = (a[2] + local[2] - Fraction(start)) / (Fraction(end) - Fraction(start))
    cross = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    require(0 < t < 1 and cross != 0, 'no interior sweep-plane crossing')
    u = ((x - a[0]) * (c[1] - a[1]) - (y - a[1]) * (c[0] - a[0])) / cross
    v = ((b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])) / cross
    require(min(u, v, 1 - u - v) > 0, 'rail witness lies outside strict lip interior')
    return {'faceVertices': list(face), 'metres': list(map(float, (x, y, a[2]))),
            'fraction': float(t), 'barycentric': list(map(float, (1 - u - v, u, v)))}


def swept_station_margin(pieces, faces, x, start, end):
    x, start, end = map(Fraction, (x, start, end))
    lo, hi = sorted((start, end))
    # Inbound pose reverses X and Z. All source vertices and complete faces are enclosed.
    transformed = [((x - b[0], a[1], lo - b[2]), (x - a[0], b[1], hi - a[2]))
                   for a, b in map(bounds, pieces)]
    return rounded_mm(min(coordinate_margin(piece, tri) for piece in transformed for tri in faces))


def verify():
    data = json.loads((ROOT / FIXTURE).read_bytes())
    inputs = {}
    for path in (f'{ASSET}/source.vertices', f'{ASSET}/piece-indices.bin', STATION):
        raw = (ROOT / path).read_bytes()
        require(hashlib.sha256(raw).hexdigest() == data['sourceSha256'][path], 'source mismatch: ' + path)
        inputs[path] = raw
    source_points = vertices(inputs[f'{ASSET}/source.vertices'][16:])
    pieces = read_supports(inputs[f'{ASSET}/piece-indices.bin'], source_points)
    tri = [tuple(map(Fraction, xyz)) for xyz in data['opening']['triangleMetres']]
    offset = Fraction(data['opening']['hullTranslationX'])
    travelled = bounds(tri + [(x - offset, y, z) for x, y, z in tri])
    aperture = rounded_mm(min(coordinate_margin(bounds(piece), travelled) for piece in pieces))
    require(aperture >= 825, 'crescent opening clearance below witness')
    require(list(map(float, source_points[data['opening']['rimVertex']])) ==
            data['opening']['rimPointMetres'], 'wrong crescent contact witness')

    source = inputs[STATION]
    magic, count, total = struct.unpack_from('<8sII', source)
    require(magic == b'MPHYS001' and total == 488, 'wrong SH028 mesh')
    finish = 16 + count * 24
    station = vertices(source[16:finish])
    faces = list(struct.iter_unpack('<3I', source[finish:]))
    boxes = [bounds([station[i] for i in tri]) for tri in faces]
    dock = data['station']
    x, pair = dock['actorX'], dock['actorZ']
    start = swept_station_margin(pieces, boxes, x, pair[0], pair[0])
    aligned = swept_station_margin(pieces, boxes, 0, *dock['alignedZ'])
    require(start >= 978 and aligned > 30_000, 'SH028 clear controls no longer certified')
    hit = lip_witness(station, faces[dock['stationFace']], source_points[dock['sourceVertex']], x, *pair)
    return {'openingMillimetres': aperture, 'lipStartMillimetres': start,
            'alignedStationMillimetres': aligned, 'lipIntersection': hit}


if __name__ == '__main__':
    print(json.dumps(verify(), indent=2))
