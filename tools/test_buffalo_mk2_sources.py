"""Geometry and appearance tests against the authenticated corrected SH014 originals."""
import contextlib
from io import StringIO
from pathlib import Path
import json
from math import sqrt
import runpy
import shutil
import struct
import sys
import tempfile
import tomllib
import unittest
from unittest import mock

import prepare_buffalo_mk2_geometry as buffalo
import verify_buffalo_mk2_geometry as proof


class BuffaloMk2Sources(unittest.TestCase):
    def test_geometry_and_visual_reproduce_the_originals(self):
        for name, encoded in buffalo.prepare(buffalo.FOLDER).items():
            with self.subTest(output=name):
                self.assertEqual((buffalo.FOLDER / name).read_bytes(), encoded)
        geometry = tomllib.loads((buffalo.FOLDER / 'resource.toml').read_text())
        archived = json.loads((buffalo.FOLDER / 'pre-correction-native-export.json').read_text())
        self.assertEqual(len(geometry['nativeDesign']['faces']), 500)
        self.assertEqual(len(archived['nativeDesign']['faces']), 928)

    def test_visual_copies_original_materials_order_and_repeating_uvs(self):
        original = tomllib.loads((buffalo.FOLDER / 'resource.toml').read_text())
        for mesh, name, corner_count, map_counts in [
            ('design', 'visual.bin', 10_380, [1356, 0, 2104]),
            ('nativeDesign', 'native-visual.bin', 1_500, [264, 0, 236]),
        ]:
            with self.subTest(mesh=mesh):
                self.check_visual(original[mesh], name, corner_count, map_counts)

    def check_visual(self, original, name, corner_count, map_counts):
        visual = (buffalo.FOLDER / name).read_bytes()
        self.assertEqual(struct.unpack_from('<8sII', visual), (b'BM2LVIZ1', corner_count, 56))
        records = iter(struct.iter_unpack('<12fII', visual[16:]))

        def f32(values):
            fmt = f'<{len(values)}f'
            return struct.unpack(fmt, struct.pack(fmt, *values))

        counts = [0, 0, 0]
        for face, material_index, mapping in zip(original['faces'], original['faceMaterials'],
                                                  original['faceTextures'], strict=True):
            mat = original['materials'][material_index]
            color = tuple((mat['colour'] >> shift & 255) / 255 for shift in (24, 16, 8, 0))
            a, b, c = [original['vertices'][i] for i in face]
            u = [q - p for p, q in zip(a, b, strict=True)]
            v = [q - p for p, q in zip(a, c, strict=True)]
            cross = [u[j] * v[k] - u[k] * v[j] for j, k in [(1, 2), (2, 0), (0, 1)]]
            length = sqrt(sum(n * n for n in cross))
            normal = [n / length for n in cross]
            for i, uv in zip(face, mapping['uv'], strict=True):
                row = next(records)
                self.assertEqual(row[:6], f32((*original['vertices'][i], *normal)))
                self.assertEqual(row[6:12], f32((*color, *uv)))
                self.assertEqual(row[12:], (mapping['texture'], int(mat['emissive'])))
            counts[mapping['texture']] += 1
        self.assertIsNone(next(records, None))
        self.assertEqual(counts, map_counts)

    def test_cli_generates_but_refuses_rewritten_active_sources_and_outputs(self):
        with tempfile.TemporaryDirectory() as temp, contextlib.redirect_stdout(StringIO()):
            folder = Path(temp) / 'sh014'
            shutil.copytree(buffalo.FOLDER, folder)
            args = [buffalo.__file__, '--assets', str(folder)]
            with mock.patch.object(sys, 'argv', args):
                runpy.run_path(buffalo.__file__, run_name='__main__')
            with mock.patch.object(sys, 'argv', args + ['--check']):
                runpy.run_path(buffalo.__file__, run_name='__main__')
                (folder / 'source-piece-membership.bin').write_bytes(b'changed')
                with self.assertRaisesRegex(ValueError, 'changed SH014 encoding'):
                    runpy.run_path(buffalo.__file__, run_name='__main__')
            (folder / 'resource.toml').write_bytes((folder / 'pre-correction-native-export.json').read_bytes())
            with self.assertRaisesRegex(ValueError, 'changed SH014 original'):
                buffalo.prepare(folder)

    def test_lip_certificate_requires_interior_and_opening_requires_full_sweep(self):
        triangle = [(0, 0, 0), (4, 0, 0), (0, 4, 0)]
        with self.assertRaisesRegex(ValueError, 'misses strict source face interior'):
            proof.lip_intersection(triangle, (0, 1, 0), 3, (1, -1))
        piece = [[(0, 0, 0), (1, 1, 1)]]
        self.assertEqual(proof.planes_mm(piece, [(2, 0, 0), (4, 0, 0)], [(1, 0, 0)]), 979)
        with self.assertRaisesRegex(ValueError, 'no separating plane'):
            proof.planes_mm(piece, [(2, 0, 0), (1, 0, 0)], [(1, 0, 0)])

    def test_opening_station_and_opaque_joint_reproduce_air_evidence(self):
        path = proof.ROOT / 'integration/sh014-buffalo-mk2/CONTACT-RESULT.json'
        expected = json.loads(path.read_bytes())
        self.assertEqual(proof.verify(), expected)
        self.assertEqual(expected['corridorClearanceMillimetres'], [2211, 2205])
        self.assertEqual(expected['alignedClearanceMillimetres'], 11847)
        self.assertAlmostEqual(expected['sourceLipWitness']['fraction'], .5, places=14)
        self.assertEqual(expected['plinth']['certifiedInteriorDeckJointCubeSideMetres'], .5)
        with contextlib.redirect_stdout(StringIO()) as stdout:
            runpy.run_path(proof.__file__, run_name='__main__')
        self.assertEqual(json.loads(stdout.getvalue()), expected)
