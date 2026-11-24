"""Replay exact SH035 hull openings, source joints and original SH028 station controls."""
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import struct
import tomllib

from prepare_stag_liner_geometry import DETAIL, NATIVE, PIECE_COUNT, require
from hull_contact_geometry import (
    GUARD_MM, bounds, inside_y, lip_intersection, points, separating_planes, station_clearance,
)


ROOT = Path(__file__).resolve().parents[1]
INTEGRATION = ROOT / 'integration/sh035-stag-liner'
FOLDER = 'assets/fleet/sh035/'
POINTS = FOLDER + 'source.vertices'
MEMBERSHIP = FOLDER + 'source-piece-membership.bin'
FACES = FOLDER + 'source-triangles.bin'
RESOURCE = FOLDER + 'resource.toml'
STATION = 'assets/legacy/sh028.physics'


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
    require(len(clearances) == 7 and min(clearances.values()) >= 250, 'a SH035 swept channel closes')

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
    faces = [[vertices[i] for i in f] for f in struct.iter_unpack('<3I', frozen[FACES])]
    require(all(model['materials'][m]['colour'] & 255 == 255 for model in (detail, native)
                for m in (0, 1, 2)), 'structural joint must use original opaque materials')
    joints = []
    for joint in data['originalJoints']:
        point = tuple(map(Fraction, joint['pointMetres']))
        counts = []
        for limits, material in zip(joint['faceRanges'], joint['materials'], strict=True):
            span = slice(*limits)
            require(set(face_materials[span]) == set(material), 'changed source joint material')
            counts.append(inside_y(point, faces[span]))
        joints.append({'name': joint['name'], 'pointMetres': joint['pointMetres'],
                       'sourceFaceHitCountsAboveBelow': counts})
    require(len(joints) == 8, 'missing original gallery or cockpit overlap')
    solid = data['sourceSolidContacts']
    require(len(solid) == 3 and len({frozen[MEMBERSHIP][i] for i in solid}) == 3,
            'missing a solid original hull control')
    return {'clearanceMillimetres': clearances, 'alignedMillimetres': aligned,
            'offsetStartMillimetres': before, 'sourceLipWitness': crossing,
            'sourceSolidContactsMetres': [list(map(float, vertices[i])) for i in solid],
            'originalJoints': joints}


if __name__ == '__main__':
    print(json.dumps(verify(), indent=2))
