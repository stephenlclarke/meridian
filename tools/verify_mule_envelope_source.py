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
    # Verify the current gameplay baseline after reversing only the two module declarations.
    # The sealed author packet separately preserves its old core and all 519 geometry files.
    binding = json.loads((ROOT / 'integration/sh001-mule/CURRENT-GAMEPLAY.json').read_text())
    for path, pin in binding['coreFiles'].items():
        before = subprocess.check_output(['git', '-C', str(ROOT), 'show', binding['gameplayAuthority'] + ':' + path])
        after = (ROOT / path).read_bytes()
        if path == 'crates/meridian-core/src/lib.rs':
            assert after.count(b'pub mod mule;\n') == 1
            after = after.replace(b'pub mod mule;\n', b'', 1)
        if path == 'crates/meridian-core/src/station.rs':
            assert after.count(b'mod mule;\n') == 1
            after = after.replace(b'mod mule;\n', b'', 1)
        assert len(before) == pin['bytes'] and sha(before) == pin['sha256'], path
        assert after == before, path
    assert len(binding['coreFiles']) == 131
    patch = (ROOT / 'integration/sh001-mule/CURRENT-GAMEPLAY.patch').read_bytes()
    assert sha(patch) == binding['moduleDeclarationPatchSHA256']
    plus = [line for line in patch.decode().splitlines() if line.startswith('+') and not line.startswith('+++')]
    assert plus == ['+pub mod mule;', '+mod mule;']
    criteria = json.loads((ROOT / 'integration/sh001-mule/CRITERIA.json').read_text())
    names = re.findall(r'#\[test\]\s*fn\s+(\w+)\(', (ROOT / 'crates/meridian-core/src/mule/tests.rs').read_text())
    assert len(names) == 10 and names == [test['name'] for test in criteria['newCoreTests']]
    assert all(test['status'] == 'UNRUN' for test in criteria['newCoreTests'])
    print(json.dumps({'status': 'PASS_SOURCE_ONLY', 'existingGameplayCoreFilesUnchanged': len(binding['coreFiles']), 'currentGameplayCorePins': 131, 'newCoreTests': 10, 'vertexEncoding': 5344, 'triangleEncoding': 9008, 'constructorsOrContactOrContainmentOrCompilerExecuted': False}))


if __name__ == '__main__':
    verify()
