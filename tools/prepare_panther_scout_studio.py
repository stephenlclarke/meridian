"""Reproduce SH033's game-only Hull Studio definition from pinned Scout inputs."""

import argparse
from bisect import bisect_right
import hashlib
import json
import math
from pathlib import Path
import struct
import subprocess
import tempfile
import tomllib

from import_goshawk_envelope_source import components


ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'assets/fleet/sh033'
OUTPUT = ROOT / 'integration/sh033-panther-scout/PANTHER-SCOUT-STUDIO.json'
FIXTURES = OUTPUT.with_name('CLEARANCE-FIXTURES.json')
REVISION = 'a11fe16924d87d06021816f9037952afcbac08db'
TREE = '68fec574df12ddf74acb14747e0dd6839f478642'
PINS = {
    'resource.toml': '1988405e8771bf509d9a22811cf36dea4433d4a1dca451e3bf64fd55b9e385b5',
    'native-export.json': 'c55369c81023703ca030f39b6934c69e844e21a4a7c2b400ff95cc0d7c90d405',
    'contract.toml': 'f2e0376401c62989cac6f1faad27d484e53627008818dabe0e49ae2e3f194c76',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
}
SOURCE_PATH = 'fleet-completion-20261019/batch-01-native-lod/SH033-design.toml'
SOURCE_SHA = '2bdbe5b0f70a0d0316d8288d73457f96364fdad930fd7b746ce311437dacf435'
ROUNDING = .02
DETAIL_NAMES = (
    'keel', 'cabin', 'left-arch', 'left-sled', 'left-cross-brace',
    'left-drive', 'left-drive', 'left-drive', 'right-arch', 'right-sled',
    'right-cross-brace', 'right-drive', 'right-drive', 'right-drive',
    'glazing', 'glazing-crest', 'scout-survey-spine', 'left-scout-sail', 'right-scout-sail',
    'left-pipe', 'left-pipe-mount', *(f'left-pipe-clamp-{i}' for i in range(5)),
    'right-pipe', 'right-pipe-mount', *(f'right-pipe-clamp-{i}' for i in range(5)),
)
NATIVE_DETAIL = (0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 14, 16, 17, 18)
TEXTURES = ('l337-patrol-diffuse', 'l337-strike-diffuse', 'l337-service-details')
TEXTURE_HASHES = dict(zip(TEXTURES, (
    '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
    'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
    '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d',
), strict=True))
OPEN_ROUTES = [
    {'name': f'{side} arch opening', 'start': [x,.5,-8], 'end': [x,.5,7], 'radiusMetres': .1}
    for side, x in [('Port', -5.7), ('Starboard', 5.7)]
] + [
    {'name': f'{side} Scout upper opening', 'start': [x,6,-10], 'end': [x,6,-2], 'radiusMetres': .1}
    for side, x in [('Port', -3.8), ('Starboard', 3.8)]
]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def authenticated(folder):
    data = {name: (folder / name).read_bytes() for name in PINS}
    for name, raw in data.items():
        require(hashlib.sha256(raw).hexdigest() == PINS[name], f'Changed Scout source {name}')
    resource = tomllib.loads(data['resource.toml'].decode())
    contract = tomllib.loads(data['contract.toml'].decode())
    archived = json.loads(data['native-export.json'])
    require(resource['id'] == contract['id'] == archived['id'] == 'SH033', 'Wrong Scout identity')
    require(resource['name'] == contract['name'] == 'Panther Scout', 'Wrong Scout variant')
    require(archived['sourcePath'] == contract['source']['geometryFile'] == SOURCE_PATH
            and archived['sourceSHA256'] == contract['source']['geometrySHA256'] == SOURCE_SHA
            and archived['coordinateUnits'] == 'metres', 'Wrong native source or units')
    require(archived['nativeDesign'] == resource['nativeDesign'], 'Native geometry differs from archive')
    require(contract['geometry']['nativeFaceLimit'] == 512, 'Changed native limit')
    for name, expected in TEXTURE_HASHES.items():
        with (ROOT / 'assets/fleet/sh001' / f'{name}.rgba').open('rb') as stream:
            require(hashlib.file_digest(stream, 'sha256').hexdigest() == expected, f'Changed shared texture {name}')
    for key, expected in [('design', (2672, 5212, 33)), ('nativeDesign', (188, 320, 14))]:
        model = resource[key]
        require((len(model['vertices']), len(model['faces']), len(components(model))) == expected,
                f'Changed Scout {key} topology')
        require(model['paletteName'] == 'forest-ivory' and model['license'] == model['appearanceLicense'] == 'MIT',
                f'Wrong Scout {key} paint provenance')
        require(tuple(t['asset'] for t in model['textures']) == TEXTURES and all(t['license'] == 'MIT' for t in model['textures']),
                'Changed shared texture inputs')
    return resource, data['LICENSE'].decode()


def arch_faces(mesh, starts, rings, stride, cuts):
    """Keep four source-local longitudinal bands per arch; duplicate seam points only."""
    result = {}
    count = 2 * stride * (rings - 1) + 2 * (stride - 2)
    for side, (first_vertex, first_face) in zip(('left', 'right'), starts, strict=True):
        for face_index in range(first_face, first_face + count):
            rows = sorted({(i - first_vertex) // stride for i in mesh['faces'][face_index]})
            require(0 <= rows[0] <= rows[-1] < rings and
                    (len(rows) == 1 and rows[0] in (0, rings-1) or len(rows) == 2 and rows[1] == rows[0]+1),
                    'Unexpected Scout arch topology')
            result[face_index] = f'{side}-arch-{bisect_right(cuts, rows[0])}'
    return result


def support_mesh(mesh, names, arch, slots):
    labels = {}
    for group, name in zip(components(mesh), names, strict=True):
        for vertex in group:
            labels[vertex] = name
    points, membership, faces, remap = [], [], [], {}
    for number, face in enumerate(mesh['faces']):
        require(len(face) == len(set(face)) == 3 and len({labels[i] for i in face}) == 1,
                'Invalid source triangle or group')
        group = slots[arch.get(number, labels[face[0]])]
        new_face = []
        for index in face:
            key = (index, group)
            if key not in remap:
                remap[key] = len(points)
                points.append(mesh['vertices'][index])
                membership.append(group)
            new_face.append(remap[key])
        faces.append(new_face)
    require({i for i, _ in remap} == set(range(len(mesh['vertices']))), 'Unreferenced Scout source point')
    require([[points[i] for i in f] for f in faces] == [[mesh['vertices'][i] for i in f] for f in mesh['faces']],
            'Source face positions or ordering changed')
    return {'vertices': points, 'triangles': faces, 'groups': membership}


def geometry(resource):
    slot_names = sorted((set(DETAIL_NAMES) - {'left-arch', 'right-arch'}) |
                        {f'{side}-arch-{band}' for side in ('left', 'right') for band in range(4)})
    slots = {name: i for i, name in enumerate(slot_names)}
    detail = resource['design']
    native = resource['nativeDesign']
    detailed_mesh = support_mesh(detail, DETAIL_NAMES,
                                 arch_faces(detail, ((388, 768), (1124, 2216)), 28, 12, (5, 14, 22)), slots)
    native_mesh = support_mesh(native, [DETAIL_NAMES[i] for i in NATIVE_DETAIL],
                               arch_faces(native, ((36, 64), (92, 160)), 5, 4, (1, 2, 3)), slots)
    return detailed_mesh, native_mesh, slot_names


def definition(folder=ASSETS):
    resource, license_text = authenticated(folder)
    detail, native, names = geometry(resource)
    visible = resource['design']
    return {
        'schema': 1, 'id': 'SH033', 'name': 'Panther Scout', 'metresPerUnit': 1,
        'roundingMetres': ROUNDING, 'detail': detail, 'native': native,
        'materials': [{'name': m['name'], 'rgb': [(m['colour'] >> shift) & 255 for shift in (24,16,8)],
                       'alpha': m['colour'] & 255, 'emissive': m['emissive']} for m in visible['materials']],
        'faceMaterials': visible['faceMaterials'],
        'faceTextures': [{'uv': f['uv'], 'texture': visible['textures'][f['texture']]['asset'] if f['texture'] >= 0 else None}
                         for f in visible['faceTextures']],
        'engineering': None, 'flight': {'maxSpeed': resource['game']['speed']},
        'clearances': OPEN_ROUTES,
        'provenance': {'sourceId': 'SH033', 'revision': REVISION, 'tree': TREE,
                       'sourceHashes': PINS, 'archiveModelSHA256': SOURCE_SHA,
                       'familyActivationSHA256': '82e845ba6b66c3340a766079136be3a0136e58f6287e9d9a380017f3414e4ff0',
                       'physicalContractSHA256': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
                       'license': license_text, 'originalMetresPerUnit': 1, 'groupNames': names,
                       'textureHashes': TEXTURE_HASHES},
    }


def clearance_fixtures(package):
    """Separate analytic witnesses, not a substitute for the real Rust queries."""
    names = package['provenance']['groupNames']
    grouped = [[] for _ in names]
    for mesh in (package['detail'], package['native']):
        for vertex, group in zip(mesh['vertices'], mesh['groups'], strict=True):
            grouped[group].append(vertex)
    boxes = [(tuple(map(min, zip(*points))), tuple(map(max, zip(*points)))) for points in grouped]
    open_rows = []
    for route in package['clearances']:
        lo = tuple(map(min, zip(route['start'], route['end'])))
        hi = tuple(map(max, zip(route['start'], route['end'])))
        require(sum(a != b for a, b in zip(lo, hi)) == 1, 'Expected axial fixture')
        gaps = [math.hypot(*(max(a-d, c-b, 0) for a,b,c,d in zip(lo,hi,box_lo,box_hi)))
                for box_lo, box_hi in boxes]
        gap = min(gaps) - package['roundingMetres'] - route['radiusMetres']
        require(gap > 0, f'No AABB-separated clearance for {route["name"]}')
        open_rows.append({**route, 'provedLowerGapMetres': gap, 'expectedPassed': True})
    solid_rows = []
    for name in ('keel', 'scout-survey-spine', 'left-scout-sail', 'right-scout-sail'):
        points = grouped[names.index(name)]
        # Mean is a convex combination of actual source points, hence inside the
        # source convex support. The straight test route includes that point.
        centre = [math.fsum(axis)/len(points) for axis in zip(*points)]
        solid_rows.append({'name': name, 'start': [centre[0]-1, *centre[1:]],
                           'end': [centre[0]+1, *centre[1:]], 'radiusMetres': .1,
                           'supportInteriorWitness': centre, 'expectedPassed': False})
    port = (ROOT / 'assets/legacy/sh028.port').read_bytes()
    require(port[:8] == b'MPORT001' and struct.unpack_from('<I', port, 88)[0] == 4, 'Changed station port')
    aperture = list(struct.iter_unpack('<3d', port[92:]))
    require(aperture == [(-62.32,-50.8,0),(62.32,-50.8,0),(62.32,50.8,0),(-62.32,50.8,0)],
            'Changed original station opening coordinates')
    raw = [point for group in grouped for point in group]
    extent = [max(abs(p[axis]) for p in raw) + package['roundingMetres'] for axis in (0,1)]
    aligned_margin = [62.32-extent[0],50.8-extent[1]]
    offset_excess = 60 - min(p[0] for p in raw) + package['roundingMetres'] - 62.32
    require(min(aligned_margin) > 0 and offset_excess > 0, 'Unexpected aperture fixture')
    return {'hull': 'SH033', 'roundingMetres': ROUNDING, 'openRoutes': open_rows, 'solidRoutes': solid_rows,
            'stationAperture': {'sourceSHA256': hashlib.sha256(port).hexdigest(), 'widthMetres': 124.64,
                                'heightMetres': 101.6, 'alignedProjectionMarginMetres': aligned_margin,
                                'offsetXMetres': 60, 'offsetProjectionExcessMetres': offset_excess,
                                'scope': 'Projection only; physical SH028 station sweep not yet run'},
            'scope': 'Conservative local AABB gap and actual convex support interior witnesses; Rust clearance validation pending'}


def validate_with(validator, package, evidence):
    """Run an already-built actual Hull Studio validator; never builds or launches the game."""
    validator = validator.resolve()
    require(validator.is_file(), 'Select the real meridian-hull-tool binary')
    with tempfile.TemporaryDirectory(prefix='meridian-sh033-') as temp:
        folder = Path(temp)

        def validate(label, doc, expected):
            path = folder / 'package.json'
            report = folder / 'report.json'
            report.unlink(missing_ok=True)
            path.write_text(json.dumps(doc, allow_nan=False, separators=(',', ':')))
            result = subprocess.run([str(validator), 'validate', str(path), '--report', str(report)],
                                    capture_output=True, check=False, timeout=180)
            require(report.is_file(), f'Missing validator report for {label}: {result.stderr!r}')
            data = json.loads(report.read_bytes())
            require(data['valid'] is expected and (result.returncode == 0) is expected,
                    f'Unexpected validator result for {label}: {data.get("diagnostics")}')
            require(data['clearances'] == [{'name': r['name'], 'passed': expected} for r in doc['clearances']],
                    f'Unexpected per-route witness for {label}')
            require(len(data['supportGroups']) == len(package['provenance']['groupNames']),
                    f'Wrong support-group count for {label}')
            return {'label': label, 'exitCode': result.returncode, 'clearances': data['clearances'],
                    'diagnostics': data['diagnostics']}

        positive = validate('original Scout open paths', package, True)
        refusals = []
        for route in evidence['solidRoutes']:
            probe = {key: route[key] for key in ('name', 'start', 'end', 'radiusMetres')}
            refusals.append(validate(route['name'], {**package, 'clearances': [probe]}, False))
    with validator.open('rb') as stream:
        executable_sha = hashlib.file_digest(stream, 'sha256').hexdigest()
    return {'validatorSHA256': executable_sha, 'positive': positive, 'negative': refusals}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=OUTPUT)
    parser.add_argument('--fixtures', type=Path, default=FIXTURES)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--validator', type=Path, help='Run the already-built actual validator on open and solid route fixtures')
    args = parser.parse_args()
    package = definition()
    encoded = (json.dumps(package, allow_nan=False, separators=(',', ':')) + '\n').encode()
    witnesses = clearance_fixtures(package)
    evidence = (json.dumps(witnesses, allow_nan=False, indent=2) + '\n').encode()
    if args.check:
        require(args.output.read_bytes() == encoded, 'Changed generated SH033 definition')
        require(args.fixtures.read_bytes() == evidence, 'Changed generated SH033 clearance evidence')
    else:
        args.output.write_bytes(encoded)
        args.fixtures.write_bytes(evidence)
    if args.validator:
        result = validate_with(args.validator, package, witnesses)
        result['sourcePackageSHA256'] = hashlib.sha256(encoded).hexdigest()
        result['fixtureSHA256'] = hashlib.sha256(evidence).hexdigest()
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
