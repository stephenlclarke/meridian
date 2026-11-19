"""Exact SH036/SH028 fixed-orientation translational sweep certificates."""
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import struct

from prepare_zebu_studio import COUNTS, NAMES, require

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / 'integration/sh036-zebu/STATION-FIXTURE.json'
GUARD_MM = 21


def points(data):
    return [tuple(Fraction(value) for value in point) for point in struct.iter_unpack('<3d', data)]


def bounds(shape):
    return tuple(tuple(fn(axis) for axis in zip(*shape, strict=True)) for fn in (min, max))


def clearance(pieces, station, x, positions):
    """Every point of the rounded swept support is enclosed by these swept boxes."""
    start, end = sorted(map(Fraction, positions))
    actor_x = Fraction(x)
    swept = [((actor_x-hi[0], lo[1], start-hi[2]), (actor_x-lo[0], hi[1], end-lo[2])) for lo, hi in map(bounds, pieces)]
    gap, piece, face = min((max(max(c-b, a-d) for a,b,c,d in zip(lo,hi,low,high,strict=True)), i, j) for i,(lo,hi) in enumerate(swept) for j,(low,high) in enumerate(station))
    return {'netMillimetres': gap * 1000 // 1 - GUARD_MM, 'limitingPiece': piece, 'limitingStationFace': face}


def lip_intersection(triangle, vertex, x, positions):
    z = triangle[0][2]
    require(all(p[2] == z for p in triangle), 'Wrong station lip plane')
    start, end = map(Fraction, positions)
    t = (z + vertex[2] - start) / (end-start)
    x, y = Fraction(x) - vertex[0], vertex[1]
    (ax, ay), (bx, by), (cx, cy) = [p[:2] for p in triangle]
    det = (bx-ax)*(cy-ay) - (by-ay)*(cx-ax)
    require(det != 0, 'Degenerate lip face')
    u = ((x-ax)*(cy-ay) - (y-ay)*(cx-ax)) / det
    v = ((bx-ax)*(y-ay) - (by-ay)*(x-ax)) / det
    weights = (1-u-v, u, v)
    require(0 < t < 1 and min(weights) > 0, 'No strict original source lip crossing')
    return {'fraction': float(t), 'actorCentreZMetres': float(start+t*(end-start)), 'pointMetres': list(map(float, (x,y,z))), 'barycentrics': list(map(float, weights))}


def verify(root=ROOT, fixture=FIXTURE):
    spec = json.loads(fixture.read_bytes())
    frozen = {}
    for name, expected in spec['sourcePins'].items():
        blob = (root / name).read_bytes()
        require(hashlib.sha256(blob).hexdigest() == expected, f'Changed station sweep input {name}')
        frozen[name] = blob
    require(spec['station'] == 'SH028' and spec['yawHalfTurn'] and spec['guardMillimetres'] == GUARD_MM, 'Wrong station or pose convention')
    source = frozen['assets/fleet/sh036/source.vertices']
    detail, native = COUNTS[0][0], COUNTS[1][0]
    require(source[:16] == struct.pack('<8sII', b'MSH036V1', detail, native), 'Wrong hull stream')
    vertices = points(source[16:])
    membership = frozen['assets/fleet/sh036/source-piece-membership.bin']
    require(len(vertices) == len(membership) == detail+native and set(membership) == set(range(len(NAMES))), 'Wrong hull support membership')
    pieces = [[] for _ in NAMES]
    for index, point in zip(membership, vertices, strict=True):
        pieces[index].append(point)
    station = frozen['assets/legacy/sh028.physics']
    magic, vc, fc = struct.unpack_from('<8sII', station)
    require(magic == b'MPHYS001' and (vc,fc) == (272,488), 'Wrong station physics mesh')
    end = 16+24*vc
    xyz = points(station[16:end])
    triangles = [[xyz[i] for i in face] for face in struct.iter_unpack('<3I', station[end:])]
    require(len(triangles) == fc, 'Wrong station face records')
    boxes = list(map(bounds, triangles))
    aligned, offset = spec['aligned'], spec['offset']
    lip = [point for tri in triangles[480:488] for point in tri]
    lip_low, lip_high = bounds(lip)
    hull_low, hull_high = bounds(vertices)
    start, finish = map(Fraction, aligned['actorCentreZMetres'])
    guard = Fraction(GUARD_MM, 1000)
    start_front = start-hull_high[2]-guard
    finish_rear = finish-hull_low[2]+guard
    require(start_front > lip_high[2] and finish_rear < lip_low[2], 'Aligned hull does not fully traverse the original lip band')
    clear = clearance(pieces, boxes, aligned['actorXMetres'], aligned['actorCentreZMetres'])
    before = clearance(pieces, boxes, offset['actorXMetres'], [offset['actorCentreZMetres'][0]]*2)
    require(clear['netMillimetres'] > 0 and before['netMillimetres'] > 0, 'Aligned or initial offset pose is not certified clear')
    native_index = offset['nativeLocalVertex']
    require(0 <= native_index < native, 'Witness is not an original native vertex')
    crossing = lip_intersection(triangles[offset['stationFace']], vertices[detail+native_index], offset['actorXMetres'], offset['actorCentreZMetres'])
    return {'hull': 'SH036', 'station': 'SH028', 'method': 'Exact original binary64 rational AABB separating axes for full translation; rounded supports plus 1 mm guard. Exact half-turn native vertex intersection with station lip triangle interior.', 'aligned': {**clear, 'stationLipZMetres': [float(lip_low[2]),float(lip_high[2])], 'guardedFirstLeadingEdgeZMetres': float(start_front), 'guardedLastTrailingEdgeZMetres': float(finish_rear)}, 'offsetStart': before, 'nativeLipCrossing': {'nativeLocalVertex': native_index, 'nativeGlobalVertex': detail+native_index, 'stationFace': offset['stationFace'], **crossing}}


if __name__ == '__main__':
    print(json.dumps(verify(), indent=2))
