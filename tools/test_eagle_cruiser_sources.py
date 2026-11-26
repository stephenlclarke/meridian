"""Authenticate the original Eagle sources and check the separate inspection candidate."""
import hashlib
import json
from math import hypot
from pathlib import Path
import struct
import tempfile
import tomllib
import unittest
from unittest.mock import patch

import prepare_eagle_cruiser_geometry as geometry
import prepare_eagle_cruiser_studio as studio
import verify_eagle_cruiser_geometry as proof


class EagleCruiserSources(unittest.TestCase):
    def test_frozen_original_blobs_and_shared_atlases(self):
        sources = json.loads((proof.INTEGRATION / 'FROZEN-SOURCES.json').read_bytes())
        self.assertEqual(sources['revision'], geometry.LEGACY)
        self.assertEqual(sources['tree'], '68fec574df12ddf74acb14747e0dd6839f478642')
        self.assertEqual(len(sources['paths']), 11)
        for name, expected in sources['paths'].items():
            with self.subTest(path=name):
                folder = proof.INTEGRATION if name.startswith('LEGACY-') else geometry.ASSETS
                if name.endswith('.rgba'):
                    folder = geometry.ROOT / 'assets/fleet/sh001'
                data = (folder / name).read_bytes()
                self.assertEqual(len(data), expected['bytes'])
                self.assertEqual(hashlib.sha256(data).hexdigest(), expected['sha256'])
                self.assertEqual(hashlib.sha1(f'blob {len(data)}\0'.encode() + data).hexdigest(), expected['blob'])

    def test_source_encodings_preserve_material_uv_normals_and_winding_in_both_meshes(self):
        generated = geometry.prepare(geometry.ASSETS)
        for name, data in generated.items():
            with self.subTest(path=name):
                self.assertEqual((geometry.ASSETS / name).read_bytes(), data)
        original = tomllib.loads((geometry.ASSETS / 'resource.toml').read_text())
        for designation, filename, expected_count in (
            ('design', 'visual.bin', [936, 864, 860, 1216]),
            ('nativeDesign', 'native-visual.bin', [120, 88, 24, 16]),
        ):
            mesh = original[designation]
            visual = generated[filename]
            self.assertEqual(struct.unpack_from('<8sII', visual), (b'EAGLVIZ1', len(mesh['faces']) * 3, 56))
            corners = iter(struct.iter_unpack('<12fII', visual[16:]))
            counts = [0, 0, 0, 0]
            for face, material_id, map_row in zip(mesh['faces'], mesh['faceMaterials'], mesh['faceTextures'], strict=True):
                a, b, c = [mesh['vertices'][index] for index in face]
                ab = [end - start for start, end in zip(a, b, strict=True)]
                ac = [end - start for start, end in zip(a, c, strict=True)]
                cross = [ab[i] * ac[j] - ab[j] * ac[i] for i, j in ((1, 2), (2, 0), (0, 1))]
                normal = [x / hypot(*cross) for x in cross]
                material = mesh['materials'][material_id]
                rgba = [((material['colour'] >> shift) & 255) / 255 for shift in (24, 16, 8, 0)]
                slot = map_row['texture']
                counts[slot] += 1
                for coords, uv in zip((a, b, c), map_row['uv'], strict=True):
                    actual = next(corners)
                    packed = struct.pack('<12f', *coords, *normal, *rgba, *uv)
                    self.assertEqual(actual[:12], struct.unpack('<12f', packed))
                    self.assertEqual(actual[12:], (slot & 0xffff_ffff, int(material['emissive'])))
            self.assertIsNone(next(corners, None))
            self.assertEqual(counts, expected_count)

    def test_geometry_and_studio_checks_do_not_write_and_reject_redirects(self):
        with patch.object(geometry, 'replace_output', side_effect=AssertionError('geometry check wrote output')),              patch('sys.argv', ['prepare_eagle_cruiser_geometry', '--check']):
            geometry.main()
        with patch.object(studio, 'replace_output', side_effect=AssertionError('Studio check wrote output')),              patch('sys.argv', ['prepare_eagle_cruiser_studio', '--check']):
            studio.main()
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / 'keep'
            target.write_text('unchanged')
            link = target.with_name('redirect.json')
            try:
                link.symlink_to(target)
            except OSError as error:
                self.skipTest(f'Symbolic links unavailable: {error}')
            with patch.object(studio, 'OUTPUT', link.resolve().parent / link.name),                  patch('sys.argv', ['prepare_eagle_cruiser_studio', '--check']):
                with self.assertRaisesRegex(ValueError, 'file without redirects'):
                    studio.main()
            self.assertEqual(target.read_text(), 'unchanged')

    def test_trench_joints_and_numerical_receipts_replay(self):
        directory = proof.INTEGRATION
        result = proof.verify()
        self.assertEqual(geometry.document(result), (directory / 'CONTACT-RESULT.json').read_bytes())
        self.assertEqual(sorted(result['clearanceMillimetres'].values()), [984, 984, 1923, 1923, 3138, 3138, 3349])
        self.assertEqual(len(result['originalJoints']), 5)
        criteria = json.loads((directory / 'CRITERIA.json').read_bytes())
        self.assertIn('tools/hull_contact_geometry.py', criteria['replayPins'])
        for name, digest in criteria['replayPins'].items():
            with self.subTest(path=name):
                self.assertEqual(hashlib.sha256((geometry.ROOT / name).read_bytes()).hexdigest(), digest)
        audit = json.loads((directory / 'SURFACE-FIT.json').read_bytes())
        self.assertEqual(audit['fullUpperMillimetres'], {'detailed': 1772, 'native': 5377})
        limiting = max(audit['pieces'], key=lambda item: item['upperMillimetres']['native'])
        self.assertEqual(limiting['piece'], 'left-louvre-4')
        delta = json.loads((directory / 'ORIGINAL-DIFFERENCE.json').read_bytes())['native']
        self.assertEqual(delta['mandatoryOppositeSourceVertex'], 1943)
        self.assertEqual(delta['exactNearestFaces'], [109, 116])
        self.assertEqual(delta['strictLowerMillimetres'], 5088)

    def test_shared_parity_preserves_eagle_refusal_diagnostics(self):
        top = [[(0, 1, 0), (4, 1, 0), (0, 1, 4)]]
        for point, message in (
            ((0, 0, 0), 'joint parity ray hit a source edge'),
            ((1, 1, 1), 'joint lies on a source face'),
            ((1, 0, 1), 'joint left the source volume'),
        ):
            with self.subTest(point=point), self.assertRaisesRegex(ValueError, '^' + message + '$'):
                proof.inside_y(point, top)

    def test_studio_binds_original_meshes_and_distinct_finite_engineering(self):
        detail, native = geometry.authenticate(geometry.ASSETS)
        result = studio.definition()
        self.assertEqual(result, json.loads(studio.OUTPUT.read_bytes()))
        self.assertEqual(result['provenance']['engineeringReference'], 'finite-intrinsic-empty-eagle')
        source = json.loads((geometry.ASSETS / 'finite-dry-source.json').read_bytes())['emptyDryReference']
        engineering = result['engineering']
        self.assertEqual(engineering['massKg'], 295038)
        self.assertEqual(engineering['centerOfMass'], source['centreMetres'])
        self.assertEqual(sum(engineering['inertia'], []), source['inertiaAboutCentreOfMassKgM2'])
        self.assertEqual(len(result['clearances']), 7)
        for key, original in (('detail', detail), ('native', native)):
            with self.subTest(mesh=key):
                self.assertEqual(result[key]['vertices'], original['vertices'])
                self.assertEqual(result[key]['triangles'], original['faces'])
        self.assertEqual(set(result['detail']['groups'] + result['native']['groups']), set(range(27)))
