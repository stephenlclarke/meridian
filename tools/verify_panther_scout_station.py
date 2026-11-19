"""Independent exact rational SH033 motion witnesses against original SH028 triangles."""

from fractions import Fraction
import hashlib
import json
import struct

from prepare_panther_scout_studio import OUTPUT, ROOT, definition, require


STATION = ROOT / 'assets/legacy/sh028.physics'
STATION_SHA = 'cd54d30cab638f6b33feacf1fa78c9136f8ce346e8cc6133259bfc0472c259d5'
OUTPUT_REPORT = OUTPUT.with_name('STATION-CONTACT.json')
GUARD_MM = 21  # Rounds the 20 mm analytic support radius outward by another mm.


def bounds(points):
    return tuple(tuple(f(axis) for axis in zip(*points, strict=True)) for f in (min, max))


def projected_weights(triangle, x, y):
    (ax, ay), (bx, by), (cx, cy) = triangle
    denominator = (bx-ax)*(cy-ay) - (by-ay)*(cx-ax)
    require(denominator != 0, 'Degenerate source lip triangle')
    u = ((x-ax)*(cy-ay) - (y-ay)*(cx-ax)) / denominator
    v = ((bx-ax)*(y-ay) - (by-ay)*(x-ax)) / denominator
    return 1-u-v, u, v


def station_triangles():
    blob = STATION.read_bytes()
    require(hashlib.sha256(blob).hexdigest() == STATION_SHA, 'Changed original station physics')
    magic, count, faces = struct.unpack_from('<8sII', blob)
    require(magic == b'MPHYS001' and (count, faces) == (272, 488), 'Wrong original station mesh')
    split = 16 + 24 * count
    xyz = [tuple(map(Fraction, point)) for point in struct.iter_unpack('<3d', blob[16:split])]
    return [[xyz[i] for i in face] for face in struct.iter_unpack('<3I', blob[split:])]


def sweep_lower_mm(pieces, triangles, actor_x, actor_z):
    """Axis-separated AABBs certify the whole swept rounded support, not only a pose."""
    x = Fraction(actor_x)
    start, end = sorted(map(Fraction, actor_z))
    swept = [((x-hi[0], lo[1], start-hi[2]), (x-lo[0], hi[1], end-lo[2]))
             for lo, hi in map(bounds, pieces)]
    gap = min(max(max(c-b, a-d) for a,b,c,d in zip(lo,hi,low,high,strict=True))
              for lo,hi in swept for low,high in map(bounds, triangles))
    return int(gap * 1000 // 1) - GUARD_MM


def verify():
    package = definition()
    require(package == json.loads(OUTPUT.read_bytes()), 'Changed Scout package')
    groups = [[] for _ in package['provenance']['groupNames']]
    for mesh in (package['detail'], package['native']):
        for point, group in zip(mesh['vertices'], mesh['groups'], strict=True):
            groups[group].append(tuple(map(Fraction, point)))
    triangles = station_triangles()
    aligned = [137.32, 135.32]
    offset_x, lip = 69.5, [154.32, 152.32]
    aligned_mm = sweep_lower_mm(groups, triangles, 0, aligned)
    before_mm = sweep_lower_mm(groups, triangles, offset_x, [lip[0]]*2)
    require(aligned_mm >= 43_000 and before_mm >= 978, 'Scout initial or aligned path lacks physical triangle separation')

    # Variant-specific bundled native keel extremum crosses a strict interior
    # point of the actual original lip triangle under the inward 180° yaw.
    index = 13
    vertex = tuple(map(Fraction, package['native']['vertices'][index]))
    require(vertex == tuple(map(Fraction, (0.0, -.524, 17.0))), 'Changed Scout native keel witness')
    require(package['provenance']['groupNames'][package['native']['groups'][index]] == 'keel', 'Wrong source witness group')
    triangle = triangles[482]
    z = triangle[0][2]
    require(all(v[2] == z for v in triangle), 'Wrong SH028 face plane')
    t = (z + vertex[2] - Fraction(lip[0])) / (Fraction(lip[1]) - Fraction(lip[0]))
    x, y = Fraction(offset_x)-vertex[0], vertex[1]
    weights = projected_weights([p[:2] for p in triangle], x, y)
    require(0 < t < 1 and min(weights) > 0, 'No interior source lip crossing')
    require(all(sum(w*p[i] for w,p in zip(weights, triangle, strict=True)) == q
                for i,q in enumerate((x,y,z))), 'Failed exact intersection reconstruction')
    return {'hull': 'SH033', 'stationSHA256': STATION_SHA, 'packageSHA256': hashlib.sha256(OUTPUT.read_bytes()).hexdigest(),
            'orientation': 'inward: basis (-X,+Y,-Z)', 'guardMillimetres': GUARD_MM,
            'aligned': {'actorX': 0, 'actorCentreZ': aligned, 'minimumSeparationMillimetres': aligned_mm, 'expectedClear': True},
            'offsetOpening': {'actorX': offset_x, 'actorCentreZ': [lip[0]]*2,
                              'minimumSeparationMillimetres': before_mm, 'expectedClear': True},
            'offsetLipContact': {'actorX': offset_x, 'actorCentreZ': lip, 'stationFace': 482,
                                 'nativeSupportVertex': index, 'nativePointMetres': list(map(float,vertex)),
                                 'crossingFraction': float(t), 'stationPointMetres': list(map(float,(x,y,z))),
                                 'barycentrics': list(map(float,weights)), 'expectedClear': False},
            'scope': 'All 488 original station triangles and every rounded local support tested by exact rational separation; strict actual source vertex/triangle crossing proves contact. No World/native admission.'}


if __name__ == '__main__':
    print(json.dumps(verify(), indent=2))
