"""Verify source bytes, old-source preservation and current-gameplay adaptation only."""
import ast
import hashlib
import json
from pathlib import Path
import re
import struct
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def verify():
    folder = ROOT / 'assets/fleet/sh001'
    manifest = json.loads((folder / 'MANIFEST.json').read_text())
    for name, pin in manifest['files'].items():
        data = (folder / name).read_bytes()
        assert len(data) == pin['bytes'] and sha(data) == pin['sha256'], name
    tree = ast.parse((ROOT / 'tools/import_mule_envelope_source.py').read_text())
    pins = next(ast.literal_eval(node.value) for node in tree.body if isinstance(node, ast.Assign) and any(isinstance(target, ast.Name) and target.id == 'PINS' for target in node.targets))
    for name, expected in pins.items():
        assert sha((folder / name).read_bytes()) == expected, name
    resource = tomllib.loads((folder / 'resource.toml').read_text())
    original = tomllib.loads((folder / 'original-design.toml').read_text())
    owner = json.loads((folder / 'ownership.json').read_text())
    groups = json.loads((folder / 'COMPONENT-PIECES.json').read_text())
    vertices = bytearray(b'MSH001V1') + struct.pack('<II', 4360, 984)
    triangles = bytearray()
    membership = bytearray()
    well_pieces = {}
    for model, offset, counts in [('design', 0, (4360, 7236, 371)), ('nativeDesign', 4360, (984, 1772, 49))]:
        assert resource[model] == original[model]
        assert (len(resource[model]['vertices']), len(resource[model]['faces']), len(owner['models'][model]['parts'])) == counts
        assert len(groups[model]) == counts[2]
        for part, assigned in zip(owner['models'][model]['parts'], groups[model], strict=True):
            for key in ['vertexStart', 'vertexCount', 'faceStart', 'faceCount']:
                assert part[key] == assigned[key]
            assert part['index'] == assigned['component'] and part['name'] == assigned['name']
            name = part['name']
            piece = assigned['piece']
            if name.startswith('body primary pressure envelope/service-well-'):
                key = (model, name.split('/', 2)[1])
                assert piece == well_pieces.setdefault(key, 16 + len(well_pieces))
            elif name == 'body primary pressure envelope/longitudinal-service-strut-0':
                assert piece == 32
            elif name == 'body primary pressure envelope/longitudinal-service-strut-1':
                assert piece == 33
            elif name == 'body primary pressure envelope':
                assert piece == 0
            else:
                assert 1 <= piece < 16
            first_vertex = part['vertexStart']
            end_vertex = first_vertex + part['vertexCount']
            first_face = part['faceStart']
            end_face = first_face + part['faceCount']
            for face in resource[model]['faces'][first_face:end_face]:
                assert len(face) == 3 and all(first_vertex <= i < end_vertex for i in face)
            membership.extend(bytes([assigned['piece']]) * assigned['vertexCount'])
        for point in resource[model]['vertices']:
            vertices.extend(struct.pack('<ddd', *point))
        for face in resource[model]['faces']:
            triangles.extend(struct.pack('<III', *(index + offset for index in face)))
    assert bytes(vertices) == (folder / 'all-source.vertices').read_bytes()
    assert bytes(triangles) == (folder / 'source-triangles.bin').read_bytes()
    assert bytes(membership) == (folder / 'source-piece-membership.bin').read_bytes()
    assert len(well_pieces) == 16
    assert set(membership) == set(range(34))
    # Replay only the reviewed additive Mule hooks against the protected base.
    # Checking every pre-existing core blob catches unrelated gameplay edits.
    base = '792f4485372ae37d0c1d583ae11229e85932ce2f'
    additions = {
        'flight.rs': [
            b'mod mule_validation;\n',
            b'    mule_validation: Option<crate::traffic::mule_validation::State>,\n',
            b'            mule_validation: None,\n',
            b'        if self.mule_validation.is_some() {\n            return self.step_mule_validation(input);\n        }\n',
        ],
        'flight/emp/tests.rs': [b'        mule_validation: None,\n'],
        'flight/persistence.rs': [b'            mule_validation: None,\n'],
        'lib.rs': [b'pub mod mule;\n'],
        'station.rs': [b'mod mule;\n'],
        'traffic.rs': [
            b'pub(crate) mod mule_validation;\n',
            b'pub use mule_validation::{MuleActorSnapshot, MuleValidationRefusal};\n',
        ],
    }
    pin_bytes = (ROOT / 'integration/sh001-mule/PROTECTED-CORE.json').read_bytes()
    assert sha(pin_bytes) == 'a3cc243f3e8993157e98fc0b876375c06ede55dda93a5a4178247108cc0a5f93'
    baseline = json.loads(pin_bytes)
    assert baseline['base'] == base
    checked = set()
    for relative, blob in baseline['gitBlobs'].items():
        data = (ROOT / 'crates/meridian-core/src' / relative).read_bytes()
        for addition in additions.get(relative, []):
            assert data.count(addition) == 1, relative
            data = data.replace(addition, b'', 1)
        header = b'blob ' + str(len(data)).encode() + b'\0'
        assert hashlib.sha1(header + data).hexdigest() == blob, relative
        checked.add(relative)
    assert set(additions) <= checked and len(checked) == 115
    # Preserve the historical component and World patch identities separately.
    binding = json.loads((ROOT / 'integration/sh001-mule/CURRENT-GAMEPLAY.json').read_text())
    patch = (ROOT / 'integration/sh001-mule/CURRENT-GAMEPLAY.patch').read_bytes()
    assert sha(patch) == binding['moduleDeclarationPatchSHA256']
    plus = [line for line in patch.decode().splitlines() if line.startswith('+') and not line.startswith('+++')]
    assert plus == ['+pub mod mule;', '+mod mule;']
    criteria = json.loads((ROOT / 'integration/sh001-mule/CRITERIA.json').read_text())
    speed_contract = tomllib.loads((folder / 'contract.toml').read_text())['performance']['maxSpeed']
    speed_source = (ROOT / 'crates/meridian-core/src/mule.rs').read_text()
    speed_constant = re.search(r'const MULE_MAX_SPEED_UNITS: f32 = ([0-9.]+);', speed_source)
    assert speed_constant and float(speed_constant[1]) == speed_contract
    names = re.findall(r'#\[test\]\s*fn\s+(\w+)\(', (ROOT / 'crates/meridian-core/src/mule/tests.rs').read_text())
    assert len(names) == 11 and names == [test['name'] for test in criteria['newCoreTests']]
    assert all(test['status'] == 'UNRUN' for test in criteria['newCoreTests'])
    world_names = re.findall(r'#\[test\]\s*fn\s+(\w+)\(', (ROOT / 'crates/meridian-core/src/flight/mule_validation/tests.rs').read_text())
    assert len(world_names) == 6 and world_names == [test['name'] for test in criteria['worldCoreTests']]
    assert all(test['status'] == 'UNRUN' for test in criteria['worldCoreTests'])
    tightness = (ROOT / 'crates/meridian-core/src/mule/tightness_tests.rs').read_text()
    tightness_names = re.findall(r'#\[test\]\s*fn\s+(\w+)\(', tightness)
    assert len(tightness_names) == 5 and tightness_names == [test['name'] for test in criteria['tightnessCoreTests']]
    assert all(test['status'] == 'UNRUN' for test in criteria['tightnessCoreTests'])
    rim_text = tightness.split('const WELL_RIMS:', 1)[1].split('];', 1)[0]
    rim_indices = [(int(piece), int(first)) for piece, first in re.findall(r'\((\d+), (\d+)\)', rim_text)]
    assert rim_indices == [(part['piece'], part['vertexStart']) for part in groups['design'] if '/service-well-' in part['name'] and '/recess-' not in part['name']]
    assert len(rim_indices) == 84
    world_binding = json.loads((ROOT / 'integration/sh001-mule/WORLD-BINDING.json').read_text())
    world_patch = (ROOT / world_binding['canonicalPatch']['path']).read_bytes()
    assert len(world_patch) == world_binding['canonicalPatch']['bytes']
    assert sha(world_patch) == world_binding['canonicalPatch']['sha256']
    assert len(world_binding['currentRootCoreFiles']) == 135
    assert len(world_binding['modifiedCurrentFiles']) == 3
    print(json.dumps({'status': 'PASS_SOURCE_ONLY', 'protectedGameplayCoreFilesUnchanged': len(checked) - len(additions), 'protectedCoreFilesWithMuleHooks': len(additions), 'protectedCoreFilesChecked': len(checked), 'newCoreTests': 11, 'worldCoreTests': 6, 'tightnessCoreTests': 5, 'totalMuleCoreTests': 22, 'vertexEncoding': 5344, 'triangleEncoding': 9008, 'constructorsOrContactOrContainmentOrCompilerExecuted': False}))


if __name__ == '__main__':
    verify()
