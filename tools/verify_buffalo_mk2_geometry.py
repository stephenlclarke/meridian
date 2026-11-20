"""Check SH014 openings, opaque bridge attachment and actual SH028 source contact."""
from fractions import Fraction
import hashlib
from itertools import product
import json
from math import isqrt
from pathlib import Path
import struct
import tomllib

from prepare_buffalo_mk2_geometry import require


ROOT = Path(__file__).resolve().parents[1]
FOLDER = 'assets/fleet/sh014/'
SOURCE_VERTICES = FOLDER + 'source.vertices'
MEMBERSHIP = FOLDER + 'source-piece-membership.bin'
RESOURCE = FOLDER + 'resource.toml'
SH028 = 'assets/legacy/sh028.physics'
PADDING_MM = 21


def dot(a, b):
    return sum(x * y for x, y in zip(a, b, strict=True))


def vertices(data):
    return [tuple(Fraction(x) for x in p) for p in struct.iter_unpack('<3d', data)]


def bounds(points):
    return [tuple(op(axis) for axis in zip(*points, strict=True)) for op in (min, max)]


def planes_mm(pieces, obstacle, axes):
    results = []
    for piece, axis in zip(pieces, axes, strict=True):
        require(dot(axis, axis) > 0 and all(type(v) is int for v in axis), 'invalid separating axis')
        gap = min(dot(axis, p) for p in obstacle) - max(dot(axis, p) for p in piece)
        require(gap > 0, 'corridor has no separating plane')
        results.append(isqrt((gap * 1000)**2 // dot(axis, axis)) - PADDING_MM)
    return min(results)


def station_clearance(pieces, station, x, z):
    x = Fraction(x)
    start, end = sorted(map(Fraction, z))
    lo_hi = [((x - hi[0], lo[1], start - hi[2]), (x - lo[0], hi[1], end - lo[2]))
             for lo, hi in map(bounds, pieces)]
    gap = min(max(max(c - b, a - d) for a, b, c, d in zip(lo, hi, low, high, strict=True))
              for lo, hi in lo_hi for low, high in station)
    return gap * 1000 // 1 - PADDING_MM


def lip_intersection(triangle, vertex, x, z):
    a, b, c = triangle
    require(a[2] == b[2] == c[2], 'wrong station lip plane')
    t = (a[2] + vertex[2] - Fraction(z[0])) / (Fraction(z[1]) - Fraction(z[0]))
    x, y = Fraction(x) - vertex[0], vertex[1]
    denom = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    require(0 < t < 1 and denom != 0, 'no strict lip plane crossing')
    u = ((x - a[0]) * (c[1] - a[1]) - (y - a[1]) * (c[0] - a[0])) / denom
    v = ((b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])) / denom
    require(min(u, v, 1-u-v)>0, 'lip crossing misses strict source face interior')
    return {'fraction':float(t),'pointMetres':list(map(float,(x,y,a[2]))),
            'barycentrics':list(map(float,(1-u-v,u,v)))}


def opaque_plinth(data, positions, spec):
    detail, native = data['design'], data['nativeDesign']
    plinth = positions[slice(*spec['detailedVertices'])]
    require(plinth == positions[slice(*spec['nativeVerticesGlobal'])], 'plinth differs between sources')
    deck = positions[slice(*spec['deckVertices'])]
    p_low, p_high = bounds(plinth)
    d_low, d_high = bounds(deck)
    require(set(deck) == set(product(*zip(d_low, d_high, strict=True))), 'wrong deck corners')
    rings = []
    for y in (p_low[1], p_high[1]):
        ring = [(x, z) for x, h, z in plinth if h == y]
        lo, hi = bounds(ring)
        require(set(ring) == set(product(*zip(lo, hi, strict=True))), 'wrong plinth end rectangle')
        rings.append((lo, hi))

    def plinth_inside(point):
        x, y, z = point
        t = (y - p_low[1]) / (p_high[1] - p_low[1])
        slices = [
            tuple(a + (b - a) * t for a, b in zip(bottom, top, strict=True))
            for bottom, top in zip(*rings, strict=True)
        ]
        return 0 < t < 1 and all(a < q < b for a, q, b in zip(slices[0], (x,z), slices[1], strict=True))

    # Every corner of this 0.5 m cube belongs to both convex stock supports.
    joint = product((Fraction(-1,4), Fraction(1,4)),
                    (Fraction(181,20), Fraction(191,20)),
                    (Fraction(19), Fraction(39,2)))
    for point in joint:
        require(plinth_inside(point) and all(a < p < b for a,p,b in zip(d_low,point,d_high,strict=True)),
                'plinth lost the interior deck intersection')
    d_face, n_face = spec['detailedCabinFace'], spec['nativeCabinFace']
    require(detail['faceMaterials'][d_face]==2 and native['faceMaterials'][n_face]==2,
            'lower cabin witness is not opaque')
    require(detail['materials'][2]['colour'] & 255 == 255 and native['materials'][2]['colour'] & 255 == 255,
            'opaque cabin material lost alpha')
    placement = tuple(sum(positions[i][axis] for i in detail['faces'][d_face])/3 for axis in range(3))
    require(spec['nativeVertex'] in native['faces'][n_face], 'native anchor missed the known opaque triangle')
    n_anchor=positions[2056+spec['nativeVertex']]
    for point in (placement,n_anchor):
        require(plinth_inside(point), 'lower cabin missed strict tapered plinth volume')
    return {'heightMetres':float(p_high[1]-p_low[1]),
            'certifiedInteriorDeckJointCubeSideMetres':0.5,
            'detailedOpaqueCabinPointMetres':list(map(float,placement)),
            'nativeOpaqueCabinPointMetres':list(map(float,n_anchor))}


def verify():
    data=json.loads((ROOT/'integration/sh014-buffalo-mk2/FIXTURES.json').read_bytes())
    require(data['paddingMetres']==.021, 'wrong certificate guard')
    frozen={}
    for key in (SOURCE_VERTICES,MEMBERSHIP,RESOURCE,SH028):
        raw=(ROOT/key).read_bytes()
        require(hashlib.sha256(raw).hexdigest()==data['sourcePins'][key], 'wrong source: '+key)
        frozen[key]=raw
    points=vertices(frozen[SOURCE_VERTICES][16:])
    require(len(points)==len(frozen[MEMBERSHIP])==2336, 'wrong source point count')
    pieces=[[] for _ in range(57)]
    for slot,point in zip(frozen[MEMBERSHIP],points,strict=True):
        pieces[slot].append(point)
    mm=[]
    for row in data['corridors']:
        a=[tuple(map(Fraction,p)) for p in row['triangle']]
        d=Fraction(row['translationZ'])
        mm.append(planes_mm(pieces,a+[(x,y,z-d) for x,y,z in a],row['axes']))
    require(len(mm)==2 and min(mm)>=2200, 'an interdrum corridor lost its certified clearance')
    require(list(map(float,points[data['rim']['sourceVertex']]))==data['rim']['metres'],
            'source drum contact changed')

    a=frozen[SH028]
    magic,vc,fc=struct.unpack_from('<8sII',a)
    require(magic==b'MPHYS001' and fc==488, 'wrong SH028 station')
    finish=16+24*vc
    source=vertices(a[16:finish])
    faces=[[source[i] for i in face] for face in struct.iter_unpack('<3I',a[finish:])]
    boxes=list(map(bounds,faces))
    dock=data['station']
    start=station_clearance(pieces,boxes,dock['actorX'],[dock['lipCentreZ'][0]]*2)
    aligned=station_clearance(pieces,boxes,0,dock['alignedZ'])
    require(start>=978 and aligned>=11_800, 'SH028 clear controls no longer certified')
    lip=lip_intersection(faces[dock['face']],points[dock['nativeGlobalVertex']],
                         dock['actorX'],dock['lipCentreZ'])
    plinth=opaque_plinth(tomllib.loads(frozen[RESOURCE].decode()),points,data['plinth'])
    return {'corridorClearanceMillimetres':mm,'lipStartClearanceMillimetres':start,
            'alignedClearanceMillimetres':aligned,'sourceLipWitness':lip,'plinth':plinth}


if __name__ == '__main__':
    print(json.dumps(verify(),indent=2))
