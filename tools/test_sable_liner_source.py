"""Independent verification for both immutable SH012 geometry and prepared visual data."""
import contextlib
from io import StringIO
from pathlib import Path
import json
import runpy
import shutil
import struct
import sys
import tempfile
import tomllib
import unittest
from unittest import mock

import prepare_sable_liner_geometry as source
import verify_sable_liner_geometry as proof


class SableLinerSourceTests(unittest.TestCase):
    def test_bound_inputs_regenerate_every_committed_encoding(self):
        for name, data in source.prepare(source.ASSETS).items():
            with self.subTest(name=name):
                self.assertEqual((source.ASSETS / name).read_bytes(), data)

    def test_visual_preserves_every_original_face_texture_and_uv(self):
        staged = tomllib.loads((source.ASSETS / 'resource.toml').read_text())['design']
        visual = (source.ASSETS / 'visual.bin').read_bytes()
        self.assertEqual(struct.unpack_from('<8sII', visual), (b'SBLRVIZ1', 13488, 56))
        records = iter(struct.iter_unpack('<12fII', visual[16:]))
        def f32(values):
            format_string = f'<{len(values)}f'
            return struct.unpack(format_string, struct.pack(format_string, *values))

        untextured = 0
        for face, mapping, material_index in zip(
                staged['faces'], staged['faceTextures'], staged['faceMaterials'], strict=True):
            material = staged['materials'][material_index]
            color = tuple((material['colour'] >> shift & 255) / 255 for shift in (24, 16, 8, 0))
            for vertex, uv in zip(face, mapping['uv'], strict=True):
                row = next(records)
                self.assertEqual(row[:3], f32(staged['vertices'][vertex]))
                self.assertEqual(row[6:12], f32((*color, *uv)))
                self.assertEqual(row[12:], (mapping['texture'] & 0xffff_ffff, int(material['emissive'])))
            untextured += int(mapping['texture'] == -1)
        self.assertEqual(untextured, 1240)
        self.assertIsNone(next(records, None))

    def test_cli_refuses_rewritten_outputs_and_originals(self):
        with tempfile.TemporaryDirectory() as d, contextlib.redirect_stdout(StringIO()):
            assets = Path(d) / 'sh012'
            shutil.copytree(source.ASSETS, assets)
            args = [source.__file__, '--assets', str(assets)]
            with mock.patch.object(sys, 'argv', args):
                runpy.run_path(source.__file__, run_name='__main__')
            with mock.patch.object(sys, 'argv', args + ['--check']):
                runpy.run_path(source.__file__, run_name='__main__')
                (assets / 'visual.bin').write_bytes(b'modified')
                with self.assertRaisesRegex(ValueError, 'changed SH012 encoding'):
                    runpy.run_path(source.__file__, run_name='__main__')
            (assets / 'native-export.json').write_bytes(b'{}')
            with self.assertRaisesRegex(ValueError, 'changed SH012 origin'):
                source.prepare(assets)

    def test_original_station_and_opening_witnesses_replay_exactly(self):
        path = proof.ROOT / 'integration/sh012-sable-liner/CONTACT-RESULT.json'
        expected = json.loads(path.read_bytes())
        self.assertEqual(proof.verify(), expected)
        self.assertEqual(expected['openingMillimetres'], 829)
        self.assertEqual(expected['lipIntersection']['fraction'], 0.5)
        with contextlib.redirect_stdout(StringIO()) as stdout:
            runpy.run_path(proof.__file__, run_name='__main__')
        self.assertEqual(json.loads(stdout.getvalue()), expected)

    def test_independent_lip_math_rejects_outside_and_endpoint_contacts(self):
        triangle = [(0, 0, 0), (4, 0, 0), (0, 4, 0)]
        with self.assertRaisesRegex(ValueError, 'outside strict lip interior'):
            proof.lip_witness(triangle, [0, 1, 2], (0, 1, 0), 3, 1, -1)
        with self.assertRaisesRegex(ValueError, 'no interior sweep-plane crossing'):
            proof.lip_witness(triangle, [0, 1, 2], (0, 1, 0), 1, 1, 0)
