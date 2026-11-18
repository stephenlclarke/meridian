"""Verify source bytes, old-source preservation and current-gameplay adaptation only."""
import ast
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess
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
    for model, offset, counts in [('design', 0, (4360, 7236, 371)), ('nativeDesign', 4360, (984, 1772, 49))]:
        assert resource[model] == original[model]
        assert (len(resource[model]['vertices']), len(resource[model]['faces']), len(owner['models'][model]['parts'])) == counts
        assert len(groups[model]) == counts[2]
        for part, assigned in zip(owner['models'][model]['parts'], groups[model], strict=True):
            for key in ['vertexStart', 'vertexCount', 'faceStart', 'faceCount']:
                assert part[key] == assigned[key]
            assert part['index'] == assigned['component'] and part['name'] == assigned['name']
            membership.extend(bytes([assigned['piece']]) * assigned['vertexCount'])
        for point in resource[model]['vertices']:
            vertices.extend(struct.pack('<ddd', *point))
        for face in resource[model]['faces']:
            triangles.extend(struct.pack('<III', *(index + offset for index in face)))
    assert bytes(vertices) == (folder / 'all-source.vertices').read_bytes()
    assert bytes(triangles) == (folder / 'source-triangles.bin').read_bytes()
    assert bytes(membership) == (folder / 'source-piece-membership.bin').read_bytes()
    assert set(membership) == set(range(16))
    # Bind unchanged current gameplay and the exact minimal World adaptation.
    # The sealed author packet separately preserves its old core and all 519 geometry files.
    binding = json.loads((ROOT / 'integration/sh001-mule/CURRENT-GAMEPLAY.json').read_text())
    world_binding = json.loads((ROOT / 'integration/sh001-mule/WORLD-BINDING.json').read_text())
    for path, pin in binding['coreFiles'].items():
        before = subprocess.check_output(['git', '-C', str(ROOT), 'show', binding['gameplayAuthority'] + ':' + path])
        after = (ROOT / path).read_bytes()
        if path == 'crates/meridian-core/src/lib.rs':
            assert after.count(b'pub mod mule;\n') == 1
            after = after.replace(b'pub mod mule;\n', b'', 1)
        if path == 'crates/meridian-core/src/station.rs':
            assert after.count(b'mod mule;\n') == 1
            after = after.replace(b'mod mule;\n', b'', 1)
        if path == 'crates/meridian-core/src/flight/emp/tests.rs':
            # The existing ordinary-World control has no internal Mule scenario.
            added = b'        mule_validation: None,\n'
            assert after.count(added) == 1
            after = after.replace(added, b'', 1)
        assert len(before) == pin['bytes'] and sha(before) == pin['sha256'], path
        if path in world_binding['modifiedCurrentFiles']:
            bound = world_binding['modifiedCurrentFiles'][path]
            assert len(before) == bound['before']['bytes'] and sha(before) == bound['before']['sha256'], path
            assert len(after) == bound['after']['bytes'] and sha(after) == bound['after']['sha256'], path
        else:
            assert after == before, path
    assert len(binding['coreFiles']) == 131
    patch = (ROOT / 'integration/sh001-mule/CURRENT-GAMEPLAY.patch').read_bytes()
    assert sha(patch) == binding['moduleDeclarationPatchSHA256']
    plus = [line for line in patch.decode().splitlines() if line.startswith('+') and not line.startswith('+++')]
    assert plus == ['+pub mod mule;', '+mod mule;']
    criteria = json.loads((ROOT / 'integration/sh001-mule/CRITERIA.json').read_text())
    names = re.findall(r'#\[test\]\s*fn\s+(\w+)\(', (ROOT / 'crates/meridian-core/src/mule/tests.rs').read_text())
    assert len(names) == 11 and names == [test['name'] for test in criteria['newCoreTests']]
    assert all(test['status'] == 'UNRUN' for test in criteria['newCoreTests'])
    world_names = re.findall(r'#\[test\]\s*fn\s+(\w+)\(', (ROOT / 'crates/meridian-core/src/flight/mule_validation/tests.rs').read_text())
    assert len(world_names) == 4 and world_names == [test['name'] for test in criteria['worldCoreTests']]
    assert all(test['status'] == 'UNRUN' for test in criteria['worldCoreTests'])
    tightness = (ROOT / 'crates/meridian-core/src/mule/tightness_tests.rs').read_text()
    tightness_names = re.findall(r'#\[test\]\s*fn\s+(\w+)\(', tightness)
    assert len(tightness_names) == 3 and tightness_names == [test['name'] for test in criteria['tightnessCoreTests']]
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
    print(json.dumps({'status': 'PASS_SOURCE_ONLY', 'existingGameplayCoreFilesUnchanged': 127, 'ordinaryWorldTestInitializerAdapted': 1, 'minimalWorldAdaptedCoreFiles': 3, 'currentGameplayCorePins': 131, 'newCoreTests': 11, 'worldCoreTests': 4, 'tightnessCoreTests': 3, 'totalMuleCoreTests': 18, 'vertexEncoding': 5344, 'triangleEncoding': 9008, 'constructorsOrContactOrContainmentOrCompilerExecuted': False}))


if __name__ == '__main__':
    verify()
