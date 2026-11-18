"""Bind the selected Mule source; copy fields and encode vertices without solving geometry."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import struct
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SOURCE_ROOT_ENV = 'MERIDIAN_MULE_SOURCE_ROOT'
RESOURCE = 'resource.toml'
ORIGINAL_DESIGN = 'original-design.toml'
OWNERSHIP = 'ownership.json'
CONTRACT = 'contract.toml'
NOTICE = 'LICENSE'
SNAPSHOT_SOURCE = 'snapshot'
PROPOSAL_SOURCE = 'proposal'
ARCHIVE_MEMBER = 'fleet-design-v2/ships/SH001.toml'
ARCHIVE_SHA256 = '4392dd6eb5a289145c88e5cef3ec3f83552d7920d9dc3bbb3331b8539c11ec9a'
MODELS = ('design', 'nativeDesign')
SOURCE_COUNTS = ((4360, 7236), (984, 1772))
SOURCE_FILES = (
    (RESOURCE, SNAPSHOT_SOURCE, 'Sources/L337Core/Resources/Ships/SH001.toml'),
    (ORIGINAL_DESIGN, PROPOSAL_SOURCE, 'replacement-hulls/SH001-design.toml'),
    (OWNERSHIP, SNAPSHOT_SOURCE, 'docs/milestones/fleet-physical/creator-ownership-records/SH001.json'),
    ('physical.json', SNAPSHOT_SOURCE, 'Sources/L337Core/Resources/OriginalFleetPhysical/SH001.json'),
    (NOTICE, SNAPSHOT_SOURCE, NOTICE),
)
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


def source_root():
    configured = os.environ.get(SOURCE_ROOT_ENV)
    if not configured:
        raise ValueError(f'Set {SOURCE_ROOT_ENV} to the approved Mule source directory')
    root = Path(configured).resolve(strict=True)
    if not root.is_dir():
        raise ValueError('Approved Mule source root must be a directory')
    return root


def checked_path(root, selected, *, directory=False):
    """Accept existing regular paths inside a trusted root, without redirects."""
    candidate = selected if selected.is_absolute() else root / selected
    if not candidate.is_relative_to(root) or '..' in candidate.parts:
        raise ValueError('Mule path is outside the approved directory')
    resolved = candidate.resolve(strict=True)
    if resolved != candidate or not resolved.is_relative_to(root):
        raise ValueError('Mule path must not redirect through a symbolic link or junction')
    valid = resolved.is_dir() if directory else resolved.is_file()
    if not valid:
        raise ValueError('Mule path has the wrong file type')
    return resolved


def read_contract(archive_bytes):
    digest = hashlib.sha256(archive_bytes).hexdigest()
    if digest != ARCHIVE_SHA256:
        raise ValueError('Changed current R3 archive')
    with zipfile.ZipFile(io.BytesIO(archive_bytes)) as archive:
        return authenticated(archive.read(ARCHIVE_MEMBER), CONTRACT), digest


def read_inputs(snapshot, proposal, archive):
    root = source_root()
    snapshot = checked_path(root, snapshot, directory=True)
    proposal = checked_path(root, proposal, directory=True)
    archive = checked_path(root, archive)
    directories = {SNAPSHOT_SOURCE: snapshot, PROPOSAL_SOURCE: proposal}
    inputs = {}
    for name, location, relative in SOURCE_FILES:
        directory = directories[location]
        path = checked_path(directory, directory / relative)
        inputs[name] = authenticated(path.read_bytes(), name)
    inputs[CONTRACT], archive_sha = read_contract(archive.read_bytes())
    return inputs, archive_sha


def validate_component_ranges(parts, expected):
    face_end = vertex_end = 0
    for part in parts:
        if part['faceStart'] != face_end or part['vertexStart'] != vertex_end:
            raise ValueError('Original component ranges are not complete and ordered')
        face_end += part['faceCount']
        vertex_end += part['vertexCount']
    if (vertex_end, face_end) != expected:
        raise ValueError('Lost original component membership')


def validate_model(values, original, model, expected):
    if values != original:
        raise ValueError('Selected resource differs from original ' + model)
    if (len(values['vertices']), len(values['faces'])) != expected:
        raise ValueError('Changed Mule source cardinality')
    if values['license'] != 'MIT' or values['appearanceLicense'] != 'MIT':
        raise ValueError('Changed original source notices')


def encode_vertices(source, design, ownership):
    """Preserve each detailed/native coordinate token's f64 value and source order."""
    output = bytearray(b'MSH001V1')
    output.extend(struct.pack('<II', 4360, 984))
    for model, expected in zip(MODELS, SOURCE_COUNTS, strict=True):
        values = source[model]
        validate_model(values, design[model], model, expected)
        validate_component_ranges(ownership['models'][model]['parts'], expected)
        for point in values['vertices']:
            if len(point) != 3:
                raise ValueError('Invalid original coordinate arity')
            output.extend(struct.pack('<ddd', *point))
    return bytes(output)


def structural_group(name, current, pannier):
    if name == 'pannier connected structural support':
        return 1 + 2 * pannier, pannier
    if name == 'pannier pressure envelope':
        return 2 + 2 * pannier, pannier + 1
    if name == 'cockpit glazing envelope':
        return 13, pannier
    if name.startswith('engine_pair/negative side/'):
        return 14, pannier
    if name.startswith('engine_pair/positive side/'):
        return 15, pannier
    return current, pannier


def component_group(model, name, current, body_wells):
    if name.startswith('body primary pressure envelope/service-well-'):
        root = name.split('/', 2)[1]
        return body_wells.setdefault((model, root), 16 + len(body_wells))
    if name == 'body primary pressure envelope/longitudinal-service-strut-0':
        return 32
    if name == 'body primary pressure envelope/longitudinal-service-strut-1':
        return 33
    return current


def component_groups(ownership):
    """Assign original named structural operations to source-bound smooth pieces."""
    result = {}
    point_groups = bytearray()
    # Keep differently positioned detailed and native body wells separate.
    body_wells = {}
    for model in MODELS:
        current = pannier = 0
        rows = []
        for part in ownership['models'][model]['parts']:
            name = part['name']
            current, pannier = structural_group(name, current, pannier)
            group = component_group(model, name, current, body_wells)
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


def validate_component_triangles(faces, part):
    first = part['vertexStart']
    end = first + part['vertexCount']
    face_start = part['faceStart']
    face_end = face_start + part['faceCount']
    for triangle in faces[face_start:face_end]:
        if len(triangle) != 3 or any(index < first or index >= end for index in triangle):
            raise ValueError('Original triangle crosses its creator component range')


def encode_triangles(source, membership):
    triangles = bytearray()
    for model, offset in zip(MODELS, (0, SOURCE_COUNTS[0][0]), strict=True):
        faces = source[model]['faces']
        for part in membership[model]:
            validate_component_triangles(faces, part)
        for triangle in faces:
            triangles.extend(struct.pack('<III', *(index + offset for index in triangle)))
    return bytes(triangles)


def encode_outputs(inputs):
    source = tomllib.loads(inputs[RESOURCE].decode())
    design = tomllib.loads(inputs[ORIGINAL_DESIGN].decode())
    ownership = json.loads(inputs[OWNERSHIP])
    vertices = encode_vertices(source, design, ownership)
    membership, group_bytes = component_groups(ownership)
    return {
        **inputs,
        'all-source.vertices': vertices,
        'source-triangles.bin': encode_triangles(source, membership),
        'source-piece-membership.bin': group_bytes,
        'COMPONENT-PIECES.json': (json.dumps(membership, indent=2) + '\n').encode(),
    }


def build_manifest(outputs, archive_sha):
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
    return manifest


def publish_outputs(outputs, check):
    destination = ROOT / 'assets/fleet/sh001'
    if check:
        for name, data in outputs.items():
            path = checked_path(ROOT, destination / name)
            if path.read_bytes() != data:
                raise ValueError('Changed packaged Mule output: ' + name)
        return

    checked_path(ROOT, destination, directory=True)
    for name in outputs:
        path = destination / name
        if path.exists() or path.is_symlink():
            checked_path(ROOT, path)
    for name, data in outputs.items():
        (destination / name).write_bytes(data)


def import_source(snapshot, proposal, archive, check=False):
    """Authenticate selected later per-ID records before writing any packaged byte."""
    inputs, archive_sha = read_inputs(snapshot, proposal, archive)
    outputs = encode_outputs(inputs)
    manifest = build_manifest(outputs, archive_sha)
    outputs['MANIFEST.json'] = (json.dumps(manifest, indent=2) + '\n').encode()
    publish_outputs(outputs, check)
    return manifest


def main():
    parser = argparse.ArgumentParser(
        description=__doc__,
        epilog=f'Set {SOURCE_ROOT_ENV} to an approved directory containing the three inputs. '
        'Use paths relative to that directory or absolute canonical paths within it.',
    )
    parser.add_argument('--snapshot', type=Path, required=True)
    parser.add_argument('--proposal', type=Path, required=True)
    parser.add_argument('--archive', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    result = import_source(args.snapshot, args.proposal, args.archive, args.check)
    print(json.dumps({'status': 'SOURCE_BYTES_AND_VERTEX_ENCODING_ONLY', 'vertices': result['vertices'], 'envelopeOrConstructorExecuted': False}))


if __name__ == '__main__':
    main()
