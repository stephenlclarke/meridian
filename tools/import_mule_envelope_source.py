"""Bind the selected Mule source; copy fields and encode vertices without solving geometry."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
PINS = {
    'resource.toml': '1c8cce7fb8130c12780fb522592249805cb06e6b22fcffb169ae651556f47c82',
    'original-design.toml': '5a5c99c1ab2c481e0891a90a18f9376b054709f14604c2fa5a45771d4d2817fe',
    'ownership.json': 'bdb09d57db6e8c5e110ec103cf097a48c3efb33960da8e64e6d450c9dd26503e',
    'physical.json': 'f7beb62451cb86ab289e25f88f642924df183532c2a944030ac4f7fd2825491d',
    'contract.toml': '27c960f32df859e67b969e2d9a25d9fb15f8a887a253cd20f7cd213a728bf14c',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
}


def authenticated(data, name):
    if hashlib.sha256(data).hexdigest() != PINS[name]:
        raise ValueError('Changed Mule input: ' + name)
    return data


def encode_vertices(resource, original, ownership):
    """Preserve each detailed/native coordinate token's f64 value and source order."""
    source = tomllib.loads(resource.decode())
    design = tomllib.loads(original.decode())
    parts = json.loads(ownership)
    output = bytearray(b'MSH001V1')
    output.extend(struct.pack('<II', 4360, 984))
    for model, expected in [('design', (4360, 7236)), ('nativeDesign', (984, 1772))]:
        values = source[model]
        if values != design[model]:
            raise ValueError('Selected resource differs from original ' + model)
        if (len(values['vertices']), len(values['faces'])) != expected:
            raise ValueError('Changed Mule source cardinality')
        if values['license'] != 'MIT' or values['appearanceLicense'] != 'MIT':
            raise ValueError('Changed original source notices')
        face_end = vertex_end = 0
        for part in parts['models'][model]['parts']:
            if part['faceStart'] != face_end or part['vertexStart'] != vertex_end:
                raise ValueError('Original component ranges are not complete and ordered')
            face_end += part['faceCount']
            vertex_end += part['vertexCount']
        if (vertex_end, face_end) != expected:
            raise ValueError('Lost original component membership')
        for point in values['vertices']:
            if len(point) != 3:
                raise ValueError('Invalid original coordinate arity')
            output.extend(struct.pack('<ddd', *point))
    return bytes(output)


def component_groups(ownership):
    """Assign original named structural operations to source-bound smooth pieces."""
    result = {}
    point_groups = bytearray()
    # Keep differently positioned detailed and native body wells separate.
    body_wells = {}
    for model in ['design', 'nativeDesign']:
        current = 0
        pannier = 0
        rows = []
        for part in ownership['models'][model]['parts']:
            name = part['name']
            if name == 'pannier connected structural support':
                current = 1 + 2 * pannier
            elif name == 'pannier pressure envelope':
                current = 2 + 2 * pannier
                pannier += 1
            elif name == 'cockpit glazing envelope':
                current = 13
            elif name.startswith('engine_pair/negative side/'):
                current = 14
            elif name.startswith('engine_pair/positive side/'):
                current = 15
            group = current
            if name.startswith('body primary pressure envelope/service-well-'):
                root = name.split('/', 2)[1]
                group = body_wells.setdefault((model, root), 16 + len(body_wells))
            elif name == 'body primary pressure envelope/longitudinal-service-strut-0':
                group = 32
            elif name == 'body primary pressure envelope/longitudinal-service-strut-1':
                group = 33
            rows.append({'component': part['index'], 'name': name, 'piece': group,
                         'vertexStart': part['vertexStart'], 'vertexCount': part['vertexCount'],
                         'faceStart': part['faceStart'], 'faceCount': part['faceCount']})
            point_groups.extend(bytes([group]) * part['vertexCount'])
        if pannier != 6:
            raise ValueError('Changed original pannier operation count')
        result[model] = rows
    if len(body_wells) != 16:
        raise ValueError('Changed original body well assembly count')
    return result, bytes(point_groups)


def import_source(snapshot, proposal, archive, check=False):
    """Authenticate selected later per-ID records before writing any packaged byte."""
    inputs = {
        'resource.toml': (snapshot / 'Sources/L337Core/Resources/Ships/SH001.toml').read_bytes(),
        'original-design.toml': (proposal / 'replacement-hulls/SH001-design.toml').read_bytes(),
        'ownership.json': (snapshot / 'docs/milestones/fleet-physical/creator-ownership-records/SH001.json').read_bytes(),
        'physical.json': (snapshot / 'Sources/L337Core/Resources/OriginalFleetPhysical/SH001.json').read_bytes(),
        'LICENSE': (snapshot / 'LICENSE').read_bytes(),
    }
    archive_sha = hashlib.sha256(archive.read_bytes()).hexdigest()
    if archive_sha != '4392dd6eb5a289145c88e5cef3ec3f83552d7920d9dc3bbb3331b8539c11ec9a':
        raise ValueError('Changed current R3 archive')
    with zipfile.ZipFile(archive) as z:
        inputs['contract.toml'] = z.read('fleet-design-v2/ships/SH001.toml')
    for name, data in inputs.items():
        authenticated(data, name)
    outputs = {**inputs, 'all-source.vertices': encode_vertices(inputs['resource.toml'], inputs['original-design.toml'], inputs['ownership.json'])}
    membership, group_bytes = component_groups(json.loads(inputs['ownership.json']))
    source = tomllib.loads(inputs['resource.toml'].decode())
    for model in ['design', 'nativeDesign']:
        for part in membership[model]:
            first = part['vertexStart']
            end = first + part['vertexCount']
            for triangle in source[model]['faces'][part['faceStart']:part['faceStart'] + part['faceCount']]:
                if len(triangle) != 3 or any(index < first or index >= end for index in triangle):
                    raise ValueError('Original triangle crosses its creator component range')
    triangles = bytearray()
    for model, offset in [('design', 0), ('nativeDesign', 4360)]:
        for triangle in source[model]['faces']:
            triangles.extend(struct.pack('<III', *(index + offset for index in triangle)))
    outputs['source-triangles.bin'] = bytes(triangles)
    outputs['source-piece-membership.bin'] = group_bytes
    outputs['COMPONENT-PIECES.json'] = (json.dumps(membership, indent=2) + '\n').encode()

    manifest = {
        'id': 'SH001', 'name': 'Mule', 'authorityArchiveSHA256': archive_sha, 'sourceRevision': 'a11fe16924d87d06021816f9037952afcbac08db',
        'selectedResource': 'OriginalFleetRelativePendingSourceRegistrations resource1c8cce7; later than the backlog resource pin',
        'scaleNumerator': 40000, 'scaleDenominator': 3411,
        'coordinateEncoding': 'Unscaled original local XYZ f64 values; detailed then native; source origin/winding/fields unchanged',
        'vertices': {'detailed': 4360, 'native': 984}, 'faces': {'detailed': 7236, 'native': 1772},
        'sourceComponents': {'detailed': 371, 'native': 49},
        'files': {name: {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()} for name, data in outputs.items()},
        'transform': 'Verbatim source files and source-order XYZ encoding only; no envelope/hull/contact/containment/union algorithm executed',
        'gameplayEnvelope': 'Thirty-four separate source-convex-support pieces with a2cm spherical offset; distinct conservative approximation, not exact source union or native512-face qualification',
        'pieceGrouping': 'Primary body; twelve detailed and four native body well assemblies; two longitudinal service struts; six supports; six panniers; cockpit; two engines. Each whole creator component stays in one piece.',
        'exactGeometryQualification': 'UNRESOLVED; original source and all prior exact-geometry caps/results retained',
        'admission': 'UNQUALIFIED; source binding grants no actor, shield, crew, equipment, blueprint, purchase or salvage authority',
    }
    outputs['MANIFEST.json'] = (json.dumps(manifest, indent=2) + '\n').encode()
    destination = ROOT / 'assets/fleet/sh001'
    for name, data in outputs.items():
        if check:
            if (destination / name).read_bytes() != data:
                raise ValueError('Changed packaged Mule output: ' + name)
        else:
            (destination / name).write_bytes(data)
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--snapshot', type=Path, required=True)
    parser.add_argument('--proposal', type=Path, required=True)
    parser.add_argument('--archive', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    result = import_source(args.snapshot, args.proposal, args.archive, args.check)
    print(json.dumps({'status': 'SOURCE_BYTES_AND_VERTEX_ENCODING_ONLY', 'vertices': result['vertices'], 'envelopeOrConstructorExecuted': False}))


if __name__ == '__main__':
    main()
