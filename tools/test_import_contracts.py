"""Conversion and command-line contracts using synthetic, non-packaged test geometry."""
import contextlib
import copy
import hashlib
import importlib.util
import io
from pathlib import Path
import struct
import sys
import tempfile
import unittest
from unittest.mock import patch


def load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(name + '.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


SHIP = load('import_ship')
STATION = load('import_station')
SHIP_SOURCE = b'''[design]
license = "MIT"
appearanceLicense = "MIT"
vertices = [[0,0,0], [1,0,0], [0,1,0]]
faces = [[2,0,1]]
materials = [{colour = 142671871}]
faceMaterials = [0]
'''


def convert_fixture(raw):
    with patch.object(SHIP, 'SOURCE_SHA256', hashlib.sha256(raw).hexdigest()):
        return SHIP.convert(raw)


def mesh():
    return {'license': 'MIT', 'appearanceLicense': 'MIT',
            'vertices': [[0, 0, 0], [1, 0, 0], [0, 1, 0]],
            'faces': [[2, 0, 1]], 'faceMaterials': [0],
            'materials': [{'colour': 142671871}]}


class ImportContractTests(unittest.TestCase):
    def test_ship_conversion_preserves_index_order_and_both_srgb_branches(self):
        output = convert_fixture(SHIP_SOURCE)
        self.assertEqual(len(output), 72)
        vertices = list(struct.iter_unpack('<6f', output))
        self.assertEqual([v[:3] for v in vertices], [(0., 1., 0.), (0., 0., 0.), (1., 0., 0.)])
        for vertex in vertices:
            for actual, expected in zip(vertex[3:], [0.002428215868, 0.215860500114, 1.0], strict=True):
                self.assertAlmostEqual(actual, expected, places=7)

    def test_ship_conversion_rejects_invalid_notice_material_index_or_position(self):
        changes = [
            (b'license = "MIT"', b'license = "unknown"', 'notices'),
            (b'appearanceLicense = "MIT"', b'appearanceLicense = "unknown"', 'notices'),
            (b'faceMaterials = [0]', b'faceMaterials = []', 'material assignment'),
            (b'faceMaterials = [0]', b'faceMaterials = [1]', 'valid source triangles'),
            (b'faces = [[2,0,1]]', b'faces = [[2,0]]', 'valid source triangles'),
            (b'faces = [[2,0,1]]', b'faces = [[3,0,1]]', 'index is out of range'),
            (b'faces = [[2,0,1]]', b'faces = [[-1,0,1]]', 'index is out of range'),
            (b'[0,1,0]', b'[0,1]', 'finite 3D'),
            (b'[0,1,0]', b'[nan,1,0]', 'finite 3D'),
        ]
        for original, replacement, message in changes:
            with self.subTest(replacement=replacement):
                with self.assertRaisesRegex(ValueError, message):
                    convert_fixture(SHIP_SOURCE.replace(original, replacement))

    def test_ship_cli_writes_checks_and_refuses_manifest_tampering(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'synthetic.toml'
            source.write_bytes(SHIP_SOURCE)
            with patch.object(SHIP, 'ROOT', root), patch.object(SHIP, 'SOURCE_SHA256', hashlib.sha256(SHIP_SOURCE).hexdigest()), contextlib.redirect_stdout(io.StringIO()):
                with patch.object(sys, 'argv', ['import_ship', str(source)]):
                    SHIP.main()
                target = root / 'assets/legacy/sh025.mesh'
                original = target.read_bytes()
                self.assertEqual(original, convert_fixture(SHIP_SOURCE))
                with patch.object(sys, 'argv', ['import_ship', str(source), '--check']):
                    SHIP.main()
                    (root / 'assets/legacy/manifest.json').write_text('{}')
                    with self.assertRaisesRegex(ValueError, 'differs'):
                        SHIP.main()
                self.assertEqual(target.read_bytes(), original)

    def test_station_physical_binary_uses_f64_and_keeps_triangle_indices(self):
        output = STATION.physical_bytes({'nativeDesign': mesh()})
        self.assertEqual(output[:16], b'MPHYS001\x03\x00\x00\x00\x01\x00\x00\x00')
        self.assertEqual(struct.unpack_from('<9d', output, 16), (0., 0., 0., 1., 0., 0., 0., 1., 0.))
        self.assertEqual(struct.unpack_from('<3I', output, 88), (2, 0, 1))
        self.assertEqual(len(output), 100)

    def test_station_display_contract_and_material_refusals(self):
        self.assertEqual(STATION.display_bytes({'design': mesh()}), convert_fixture(SHIP_SOURCE))
        for field, value, message in [('faceMaterials', [], 'Missing face material'), ('faceMaterials', [1], 'Invalid material')]:
            data = mesh()
            data[field] = value
            with self.assertRaisesRegex(ValueError, message):
                STATION.display_bytes({'design': data})

    def test_station_design_refuses_invalid_notice_vertices_or_faces(self):
        changes = [('license', 'unknown'), ('appearanceLicense', 'unknown'),
                   ('vertices', []), ('vertices', [[0, 1]]), ('vertices', [[float('inf'), 0, 0]]),
                   ('vertices', [[1e13, 0, 0]]), ('faces', [[0, 1]]),
                   ('faces', [[0, 1, 3]]), ('faces', [[0, 1, -1]]), ('faces', [[0, 1, 1.5]])]
        for field, value in changes:
            data = copy.deepcopy(mesh())
            data[field] = value
            with self.subTest(field=field, value=value):
                with self.assertRaises(ValueError):
                    STATION.design({'nativeDesign': data}, 'nativeDesign')

    def test_station_port_preserves_axes_depth_and_aperture_order(self):
        points = [{'x': -1, 'y': -1, 'z': 0}, {'x': 1, 'y': -1, 'z': 0},
                  {'x': 1, 'y': 1, 'z': 0}, {'x': -1, 'y': 1, 'z': 0}]
        output = STATION.port_bytes({'nativeDocking': {
            'center': {'x': 1, 'y': 2, 'z': 3}, 'outward': {'x': 0, 'y': 0, 'z': 1},
            'up': {'x': 0, 'y': 1, 'z': 0}, 'tunnelDepth': 4, 'aperture': points}})
        self.assertEqual(output[:8], b'MPORT001')
        self.assertEqual(struct.unpack_from('<10dI', output, 8), (1., 2., 3., 0., 0., 1., 0., 1., 0., 4., 4))
        self.assertEqual(struct.unpack_from('<12d', output, 92), (-1., -1., 0., 1., -1., 0., 1., 1., 0., -1., 1., 0.))

    def test_station_cli_checks_without_overwriting_mismatched_asset(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            destination = root / 'assets/legacy'
            destination.mkdir(parents=True)
            output = {'fixture.mesh': b'test-only conversion bytes'}
            with patch.object(STATION, 'ROOT', root), patch.object(STATION, 'read_blob', return_value=b'synthetic'), patch.object(STATION, 'outputs', return_value=output), contextlib.redirect_stdout(io.StringIO()):
                with patch.object(sys, 'argv', ['import_station', directory]):
                    STATION.main()
                with patch.object(sys, 'argv', ['import_station', directory, '--check']):
                    STATION.main()
                    target = destination / 'fixture.mesh'
                    target.write_bytes(b'tampered')
                    with self.assertRaisesRegex(ValueError, 'Committed import mismatch'):
                        STATION.main()
                    self.assertEqual(target.read_bytes(), b'tampered')


if __name__ == '__main__':
    unittest.main()
