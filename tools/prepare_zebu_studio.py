"""Build SH036 Studio input and exact dual-LOD source streams, without editing geometry."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import tomllib

from encode_goshawk_visual import TEXTURES
from import_goshawk_envelope_source import components, pin

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'assets/fleet/sh036'
INTEGRATION = ROOT / 'integration/sh036-zebu'
LEGACY = '0d8cc9ebe0018bf74b0d9fa10ef23b70339dd6b0'
NATIVE_PATH = 'fleet-completion-20261019/batch-02-native-lod/SH036-design.toml'
NATIVE_SHA = '3dd03d7688ee39b0009b257bb5cb0b65f20f5c4ac4580549396b07a48fceb1de'
ROUNDING = .02
COUNTS = ((1790, 3468, 28), (114, 192, 9))
SOURCE_PATHS = {
    'resource.toml': 'Sources/L337Core/Resources/Ships/SH036.toml',
    'native-export.json': 'Sources/L337Core/Resources/OriginalFleetShowroomModels/SH036.json',
    'contract.toml': 'docs/fleet-design-v2/ships/SH036.toml',
    'engineering-budget.json': 'Sources/L337Core/Resources/OriginalFleetPhysical/SH036.json',
    'finite-dry-reference.json': 'Sources/L337Core/Resources/ZebuPhysical/SH036-finite-dry-source.json',
    'LICENSE': 'LICENSE',
}
PINS = {
    'resource.toml': '106f9390f2fe373f50a207303fd1aa4e33d1d6b237964d3241d6981ea8d3035f',
    'native-export.json': '0d7eebae5806902b1fc8924a6dbb35f51c3e6bab27c6c6fcde5b18cdc8ea3225',
    'contract.toml': '4fc25d2778c8e0ab927313a279ca31fcb68df46557223e2cb268a99e7e97dcc4',
    'engineering-budget.json': 'c32a65aaa57bf82dc3a47e3e54820fe6c0d6fe9fd3da7c772da889d5ecf372b1',
    'finite-dry-reference.json': 'da4db5c7ce1b55e15440f5a5dc5807eed39e3eb782ce719deeb0f0bc813b5f44',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
}
TEXTURE_PINS = (
    '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
    'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
    '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d',
)
DETAIL_NAMES = (
    'lower-fuselage', 'hump-pressure-hull', 'port-reserve', 'port-strut',
    'port-drive', 'port-drive', 'port-drive', 'starboard-reserve', 'starboard-strut',
    'starboard-drive', 'starboard-drive', 'starboard-drive', 'glazing', 'canopy-crest',
    'port-pipe', 'port-pipe-mount', *(f'port-clamp-{i}' for i in range(5)),
    'starboard-pipe', 'starboard-pipe-mount', *(f'starboard-clamp-{i}' for i in range(5)),
)
NATIVE_DETAIL = (0, 1, 2, 3, 4, 7, 8, 9, 12)
NAMES = tuple(dict.fromkeys(DETAIL_NAMES))

# The two reserve assemblies flank the hump. The struts bridge lower down;
# keep their supports separate to preserve both upper longitudinal passages.
CLEARANCES = (
    {'name': 'port-upper-reserve-channel', 'start': [-7.1, 1.4, 6], 'end': [-7.1, 1.4, 18], 'radiusMetres': .05},
    {'name': 'starboard-upper-reserve-channel', 'start': [7.1, 1.4, 6], 'end': [7.1, 1.4, 18], 'radiusMetres': .05},
    {'name': 'stern-between-drives', 'start': [0, 0, -25], 'end': [0, 0, -19.3], 'radiusMetres': .10},
)
CONTACTS = (
    {'name': 'lower-fuselage-contact', 'start': [.45, -9, -10], 'end': [.45, 1, -10], 'radiusMetres': .05},
    {'name': 'port-reserve-contact', 'start': [-16, -.7, 8], 'end': [-8, -.7, 8], 'radiusMetres': .05},
    {'name': 'starboard-reserve-contact', 'start': [16, -.7, 8], 'end': [8, -.7, 8], 'radiusMetres': .05},
)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def encode_json(value):
    return (json.dumps(value, allow_nan=False, separators=(',', ':')) + '\n').encode()


def authenticate(folder):
    inputs = {name: (folder / name).read_bytes() for name in PINS}
    for name, data in inputs.items():
        require(hashlib.sha256(data).hexdigest() == PINS[name], f'Changed SH036 original {name}')
    resource, contract = (tomllib.loads(inputs[name].decode()) for name in ('resource.toml', 'contract.toml'))
    export = json.loads(inputs['native-export.json'])
    dry, budget = (json.loads(inputs[name]) for name in ('finite-dry-reference.json', 'engineering-budget.json'))
    require(resource['name'] == contract['name'] == dry['name'] == 'Zebu' and all(x['id'] == 'SH036' for x in (resource, contract, export, dry, budget)), 'Wrong Zebu identity')
    require(export['coordinateUnits'] == dry['coordinateUnits'] == budget['coordinateUnits'] == 'metres' and dry['metresPerSourceUnit'] == budget['metresPerSourceUnit'] == 1, 'Wrong source units')
    require(export['sourcePath'] == contract['source']['geometryFile'] == NATIVE_PATH and export['sourceSHA256'] == contract['source']['geometrySHA256'] == dry['nativeSourceSHA256'] == NATIVE_SHA, 'Wrong selected native variant')
    require(resource['nativeDesign'] == export['nativeDesign'] and contract['geometry']['nativeFaceLimit'] == 512, 'Native mesh mismatch')
    require(resource['game']['speed'] == contract['performance']['maxSpeed'] == 33, 'Wrong original speed')
    require(dry['emptyStateOnly'] and not dry['operatingAuthorityGranted'] and not dry['nativeEvidenceQualified'] and not dry['peopleOrFuelCargoEquipmentGranted'] and dry['emptyDryReference']['massKg'] == budget['dryMassKilograms'] == 64051, 'Changed physical reference authority')
    require(dry['r3SHA256'] == PINS['contract.toml'] and dry['engineeringSHA256'] == PINS['engineering-budget.json'] and dry['resourceSHA256'] == PINS['resource.toml'], 'Physical source identity mismatch')
    for name, expected in zip(TEXTURES, TEXTURE_PINS, strict=True):
        data = (folder.parent / 'sh001' / f'{name}.rgba').read_bytes()
        require(len(data) == 4194304 and pin(data)['sha256'] == expected, f'Changed shared source map {name}')
    return resource, inputs


def bounds(points):
    return [[fn(axis) for axis in zip(*points, strict=True)] for fn in (min, max)]


def memberships(mesh, names, counts):
    groups = components(mesh)
    require((len(mesh['vertices']), len(mesh['faces']), len(groups)) == counts, 'Changed mesh counts')
    require(mesh['license'] == mesh['appearanceLicense'] == 'MIT' and mesh['paletteName'] == 'sand-maroon', 'Changed source appearance')
    require([t['asset'] for t in mesh['textures']] == TEXTURES and all(t['repeat'] and t['license'] == 'MIT' for t in mesh['textures']), 'Changed texture order')
    require(all(len(point) == 3 and all(math.isfinite(v) for v in point) for point in mesh['vertices']), 'Invalid original coordinates')
    require(all(len(face) == len(set(face)) == 3 and all(type(i) is int and 0 <= i < counts[0] for i in face) for face in mesh['faces']), 'Invalid original triangle')
    result = [None] * counts[0]
    for indices, name in zip(groups, names, strict=True):
        for index in indices:
            result[index] = NAMES.index(name)
    require(all(result[a] == result[b] == result[c] for a, b, c in mesh['faces']), 'Triangle crosses supports')
    return result


def visual(mesh):
    data = bytearray(struct.pack('<8sII', b'ZEBUVIZ1', 3 * len(mesh['faces']), 56))
    for face, material, mapping in zip(mesh['faces'], mesh['faceMaterials'], mesh['faceTextures'], strict=True):
        points = [mesh['vertices'][i] for i in face]
        a, b = ([v - u for u, v in zip(points[0], point, strict=True)] for point in points[1:])
        normal = [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]
        length = math.hypot(*normal)
        slot = mapping['texture']
        require(length > 0 and math.isfinite(length) and -1 <= slot < len(TEXTURES), 'Invalid visual face')
        material = mesh['materials'][material]
        rgba = [(material['colour'] >> shift & 255) / 255 for shift in (24, 16, 8, 0)]
        for point, uv in zip(points, mapping['uv'], strict=True):
            data.extend(struct.pack('<12fII', *point, *(n / length for n in normal), *rgba, *uv, slot & 0xffffffff, int(material['emissive'])))
    return bytes(data)


def clearance_bound(path, groups):
    """Axis separation lower bound for each convex support, net of rounding and probe radius."""
    lo, hi = bounds((path['start'], path['end']))
    margins = []
    for points in groups:
        minimum, maximum = bounds(points)
        distance = max(max(a - d, c - b, 0) for a, b, c, d in zip(minimum, maximum, lo, hi, strict=True))
        margins.append(distance - ROUNDING - path['radiusMetres'])
    return min(margins)


def first_hit(mesh, route):
    start, end = route['start'], route['end']
    axes = [i for i in range(3) if start[i] != end[i]]
    require(len(axes) == 1, 'Contact witness must be axis aligned')
    axis = axes[0]
    a, b = [i for i in range(3) if i != axis]
    hits = []
    for face_id, face in enumerate(mesh['faces']):
        p, q, s = (mesh['vertices'][i] for i in face)
        det = (q[a]-p[a])*(s[b]-p[b]) - (q[b]-p[b])*(s[a]-p[a])
        if abs(det) < 1e-10:
            continue
        u = ((start[a]-p[a])*(s[b]-p[b]) - (start[b]-p[b])*(s[a]-p[a])) / det
        v = ((q[a]-p[a])*(start[b]-p[b]) - (q[b]-p[b])*(start[a]-p[a])) / det
        if u < 0 or v < 0 or u + v > 1:
            continue
        coordinate = p[axis] + u*(q[axis]-p[axis]) + v*(s[axis]-p[axis])
        t = (coordinate-start[axis]) / (end[axis]-start[axis])
        if 0 <= t <= 1:
            hits.append((t, face_id, u, v))
    require(bool(hits), 'No contact with original triangle')
    t, index, u, v = min(hits)
    require(min(u, v, 1-u-v, t, 1-t) > .01, 'Contact grazing or at endpoint')
    return {'face': index, 'fraction': t, 'pointMetres': [x + t*(y-x) for x, y in zip(start, end, strict=True)], 'barycentric': [1-u-v, u, v]}


def prepare(folder=ASSETS):
    resource, inputs = authenticate(folder)
    detail, native = resource['design'], resource['nativeDesign']
    dgroups = memberships(detail, DETAIL_NAMES, COUNTS[0])
    ngroups = memberships(native, [DETAIL_NAMES[i] for i in NATIVE_DETAIL], COUNTS[1])
    points = detail['vertices'] + native['vertices']
    ids = dgroups + ngroups
    groups = [[p for p, group in zip(points, ids, strict=True) if group == i] for i in range(len(NAMES))]
    require(len(NAMES) == 24 and all(groups), 'Wrong support count')
    faces = detail['faces'] + [[v + len(detail['vertices']) for v in f] for f in native['faces']]
    streams = {
        'source.vertices': struct.pack('<8sII', b'MSH036V1', len(dgroups), len(ngroups)) + b''.join(struct.pack('<3d', *p) for p in points),
        'source-triangles.bin': b''.join(struct.pack('<3I', *f) for f in faces),
        'source-piece-membership.bin': bytes(ids),
        'PIECES.json': encode_json({'pieces': list(NAMES), 'roundingMetres': ROUNDING}),
        'visual.bin': visual(detail), 'native-visual.bin': visual(native),
    }
    provenance = {'sourceId': 'SH036', 'revision': LEGACY, 'sourcePaths': SOURCE_PATHS, 'sourceHashes': PINS, 'derivedHashes': {name: pin(data) for name, data in streams.items()}, 'originalMetresPerUnit': 1, 'approvedNativeSource': NATIVE_PATH, 'approvedNativeSHA256': NATIVE_SHA, 'license': inputs['LICENSE'].decode(), 'physicalStatus': 'authored empty-dry references only; no operational qualification'}
    def studio_mesh(mesh, membership):
        return {'vertices': mesh['vertices'], 'triangles': mesh['faces'], 'groups': membership}
    definition = {'schema': 1, 'id': 'SH036', 'name': 'Zebu', 'metresPerUnit': 1, 'roundingMetres': ROUNDING, 'detail': studio_mesh(detail, dgroups), 'native': studio_mesh(native, ngroups), 'materials': [{'name': m['name'], 'rgb': [m['colour'] >> shift & 255 for shift in (24,16,8)], 'alpha': m['colour'] & 255, 'emissive': m['emissive']} for m in detail['materials']], 'faceMaterials': detail['faceMaterials'], 'faceTextures': [{'uv': m['uv'], 'texture': detail['textures'][m['texture']]['asset'] if m['texture'] >= 0 else None} for m in detail['faceTextures']], 'engineering': None, 'flight': {'maxSpeed': resource['game']['speed']}, 'clearances': list(CLEARANCES), 'provenance': provenance}
    passes = [{'route': path, 'conservativeNetClearanceMetres': round(clearance_bound(path, groups)-.001, 6)} for path in CLEARANCES]
    require(all(row['conservativeNetClearanceMetres'] > 0 for row in passes), 'Opening blocked by conservative support')
    contacts = [{'route': path, 'detailFirstHit': first_hit(detail, path), 'nativeFirstHit': first_hit(native, path)} for path in CONTACTS]
    witnesses = {'schema': 1, 'id': 'SH036', 'roundingMetres': ROUNDING, 'method': 'Axis support separation, including sphere radius, rounding and 1 mm guard; contact centreline crosses interior of original triangles in each LOD.', 'clearances': passes, 'contacts': contacts}
    streams['SOURCE-GEOMETRY.json'] = encode_json({'id': 'SH036', 'name': 'Zebu', 'detail': dict(zip(('vertices','faces','components'), COUNTS[0], strict=True), bounds=bounds(detail['vertices'])), 'native': dict(zip(('vertices','faces','components'), COUNTS[1], strict=True), bounds=bounds(native['vertices']), faceLimit=512), 'pieces': len(NAMES), 'provenance': provenance})
    return streams, {'ZEBU-STUDIO.json': encode_json(definition), 'CONTACT-WITNESSES.json': encode_json(witnesses)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for folder, files in zip((ASSETS, INTEGRATION), prepare(), strict=True):
        for name, data in files.items():
            path = folder / name
            if args.check:
                require(path.read_bytes() == data, f'Changed generated Zebu {name}')
            else:
                folder.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)


if __name__ == '__main__':
    main()
