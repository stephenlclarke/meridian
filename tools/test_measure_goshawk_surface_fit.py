"""Independent analytic fixtures and a replay of the calibrated SH008 adapter."""
import contextlib
import hashlib
from io import StringIO
from itertools import product
import json
from pathlib import Path
import runpy
import shutil
import struct
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

import numpy as np
import trimesh

import measure_goshawk_surface_fit as fit


TRIANGLE = ((0, 0, 0), (6, 0, 0), (0, 6, 0))
TETRAHEDRON = np.array([[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]])
TETRAHEDRON_FACES = np.array([[0, 1, 2], [0, 1, 3], [0, 2, 3], [1, 2, 3]])
REPORT = fit.ASSETS.parents[2] / 'integration/sh008-goshawk/SURFACE-FIT.json'


def plane_at(height):
    # All projections of the unit tetrahedron lie inside this triangle.
    return trimesh.Trimesh([[-2, -2, height], [6, -2, height], [-2, 6, height]],
                           [[0, 1, 2]], process=False)


def box(x, y, z):
    return fit.audit_fan(np.array(list(product(x, y, z)), dtype=float))


class TestDistanceCertificates(unittest.TestCase):
    def test_exact_distance_distinguishes_plane_edge_vertex_and_degenerate_face(self):
        cases = [
            ('interior', (1, 1, 3), TRIANGLE, 9),
            ('edge', (3, -4, 0), TRIANGLE, 16),
            ('past edge start', (-3, -4, 0), TRIANGLE, 25),
            ('past edge end', (9, -4, 0), TRIANGLE, 25),
            ('vertex at reversed edge', (9, -4, 0), (TRIANGLE[1], TRIANGLE[2], TRIANGLE[0]), 25),
            ('collinear face', (6, 3, 4), ((0, 0, 0), (4, 0, 0), (8, 0, 0)), 25),
            ('point face', (0, 0, 3), ((0, 0, 0),) * 3, 9),
        ]
        for name, point, triangle, squared in cases:
            with self.subTest(name=name):
                # Equality has zero strict clearance from the threshold.
                self.assertTrue(fit.farther_than(point, *triangle, squared - 1))
                self.assertFalse(fit.farther_than(point, *triangle, squared))
                self.assertFalse(fit.farther_than(point, *triangle, squared + 1))

    def test_lower_certificate_checks_the_entire_mesh_and_refuses_touch(self):
        point = (1, 1, 3)
        distant = tuple((x, y, -7) for x, y, _ in TRIANGLE)
        fit.verify_lower(point, [distant, TRIANGLE], 1000, 2)
        with self.assertRaises(AssertionError):
            fit.verify_lower(point, [distant, TRIANGLE], 1000, 3)

    def test_upper_covers_exact_tetrahedron_tip_even_if_cutoff_cannot_be_met(self):
        # The rounded vertex at (0, 0, 1.02) is exactly 1,020 mm from the plane.
        # An upper bound below that would confuse rounding with numerical slack,
        # or take the requested target as the result when refinement is exhausted.
        piece = fit.audit_fan(TETRAHEDRON)
        mesh = plane_at(0)
        for target_mm in (1200, 0):
            with self.subTest(target_mm=target_mm), mock.patch.object(fit, 'DEPTH', 2):
                bound = fit.certified_upper(mesh, piece, target_mm)
                self.assertGreaterEqual(bound, 1020)
                self.assertLessEqual(bound, 1021)


class TestFanAndExposure(unittest.TestCase):
    def test_closed_fan_exposes_the_tetrahedron_and_skips_a_nested_member(self):
        piece = fit.audit_fan(TETRAHEDRON)
        self.assertEqual(len(piece['supportFaces']), 4)
        for ids, axis in piece['supportFaces']:
            face_projection = TETRAHEDRON[list(ids)] @ axis
            np.testing.assert_allclose(face_projection, np.repeat(face_projection[0], 3), atol=1e-12)
            self.assertLessEqual(float((TETRAHEDRON @ axis).max() - face_projection[0]), 1e-12)
        outer = box((-1, 2), (-1, 2), (-1, 2))
        positions, _, _ = fit.exposed_samples(piece, [piece])
        self.assertEqual(len(positions), 76)
        positions, _, _ = fit.exposed_samples(piece, [piece, outer])
        self.assertEqual(fit.worst_sample(plane_at(-1), positions), (None, None))

    def test_fan_refuses_an_open_surface_and_a_missed_extreme_vertex(self):
        # Fault injection replaces only Qhull's facet nominations. Exact
        # orientation, edge pairing and the radial error audit stay real.
        missed_tip = np.vstack((TETRAHEDRON, [0., 0., 3.]))
        cases = [
            ('open', TETRAHEDRON, TETRAHEDRON_FACES[:-1]),
            ('missed source extreme', missed_tip, TETRAHEDRON_FACES),
            ('collapsed triangle', TETRAHEDRON, np.array([[0, 0, 1]])),
        ]
        for name, points, faces in cases:
            with self.subTest(name=name), mock.patch.object(
                    fit, 'ConvexHull', return_value=SimpleNamespace(simplices=faces)):
                with self.assertRaises(AssertionError):
                    fit.audit_fan(points)

    def test_exact_exposed_witness_refuses_position_occlusion_and_inflated_distance(self):
        piece = fit.audit_fan(TETRAHEDRON)
        piece['sourceIds'] = np.arange(4)
        positions, supports, weights = fit.exposed_samples(piece, [piece])
        i = next(i for i, p in enumerate(positions) if p[2] < 0 and np.all(weights[i] > 0))
        plane = plane_at(-1)
        exact, scale = fit.dyadic(plane.vertices)
        faces = ([tuple(exact)], scale)
        report, _ = fit.worst_sample(plane, positions[i:i + 1])
        self.assertAlmostEqual(report['metres'], 0.98)
        samples = positions, supports, weights
        far = box((3, 4), (3, 4), (3, 4))
        fit.certify_exposed_sample(report, i, samples, piece, [piece, far], faces)
        self.assertEqual(report['certifiedLowerMillimetres'], 979)
        self.assertEqual(sum(report['supportFacetWeightNumeratorsOver12']), 12)
        self.assertEqual(set(report['supportFacetGlobalVertices']), {0, 1, 2})

        moved = positions.copy()
        moved[i, 2] -= 0.001
        cover = box((-1, 2), (-1, 2), (-1, 2))
        cases = [
            ('off its rounded support', report.copy(), (moved, supports, weights), [piece]),
            ('inside a second piece', report.copy(), samples, [piece, cover]),
            ('overstated threshold', dict(report, metres=1.1), samples, [piece]),
        ]
        for name, witness, sample_data, pieces in cases:
            with self.subTest(name=name), self.assertRaises(AssertionError):
                fit.certify_exposed_sample(witness, i, sample_data, piece, pieces, faces)

    def test_channel_requires_a_separator_for_the_swept_obstacle(self):
        # The first box is 1.85 m from the obstacle's maximum X, before rounding.
        clear = box((2, 3), (-2, -1), (8, 9))
        self.assertIn(fit.channel_control([clear])['minimumCertifiedClearanceMillimetres'],
                      range(1828, 1830))
        crossing = box((-.25, .25), (-2, -1), (8, 9))
        with self.assertRaises(AssertionError):
            fit.channel_control([clear, crossing])


class TestPinnedAssets(unittest.TestCase):
    def test_source_loader_accepts_calibrated_shapes_and_piece_assignment(self):
        manifest, names, vertices, faces, membership = fit.load(fit.ASSETS)
        self.assertEqual(manifest['sourceRevision'], 'a11fe16924d87d06021816f9037952afcbac08db')
        self.assertEqual((len(names), len(vertices), len(faces), len(membership)),
                         (69, 2608, 4448, 2608))
        self.assertFalse(np.any(faces[:3484] >= 2072))
        self.assertFalse(np.any(faces[3484:] < 2072))

    def test_equal_length_byte_edit_refuses_the_original_digest(self):
        with tempfile.TemporaryDirectory() as root:
            folder = Path(root) / 'assets'
            shutil.copytree(fit.ASSETS, folder)
            path = folder / 'source-triangles.bin'
            data = bytearray(path.read_bytes())
            data[-1] ^= 1
            path.write_bytes(data)
            with self.assertRaisesRegex(AssertionError, 'source-triangles[.]bin'):
                fit.load(folder)

    def test_repinning_corrupt_geometry_cannot_bypass_shape_and_assignment_checks(self):
        cases = [
            (fit.SOURCE_VERTICES, 0, b'BADMAGIC'),
            (fit.SOURCE_VERTICES, 16, struct.pack('<d', float('nan'))),
            ('source-triangles.bin', 0, struct.pack('<I', 2072)),
            ('source-piece-membership.bin', 0, b'\x01'),
        ]
        for filename, offset, replacement in cases:
            with self.subTest(filename=filename, offset=offset), tempfile.TemporaryDirectory() as root:
                folder = Path(root) / 'assets'
                shutil.copytree(fit.ASSETS, folder)
                path = folder / filename
                data = bytearray(path.read_bytes())
                data[offset:offset + len(replacement)] = replacement
                path.write_bytes(data)
                original_manifest = folder / 'MANIFEST.json'
                manifest = json.loads(original_manifest.read_bytes())
                manifest['files'][filename]['sha256'] = hashlib.sha256(data).hexdigest()
                original_manifest.write_text(json.dumps(manifest))
                with self.assertRaises(AssertionError):
                    fit.load(folder)

    def test_cli_refuses_python_optimization_that_disables_assertions(self):
        result = subprocess.run([sys.executable, '-O', '-B', fit.__file__, '--help'],
                                capture_output=True, text=True, check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('The geometric certificate checks require Python assertions', result.stderr)

    def test_real_cli_replays_certificate_against_both_originals(self):
        saved_bytes = REPORT.read_bytes()
        self.assertEqual(hashlib.sha256(saved_bytes).hexdigest(),
                         '7f936de1e79b7275fd72d99ac277604b8027bae25797199fce1f42bc158e6043')
        accepted = json.loads(saved_bytes)
        stdout = StringIO()
        with contextlib.redirect_stdout(stdout), mock.patch.object(
                sys, 'argv', [fit.__file__, '--assets', str(fit.ASSETS)]):
            runpy.run_path(fit.__file__, run_name='__main__')
        replay = json.loads(stdout.getvalue())
        self.assertEqual(replay['sourcePins'], accepted['sourcePins'])
        self.assertEqual(replay['channelControl'], accepted['channelControl'])
        self.assertEqual(replay['wellDecompositionPrototype'], accepted['wellDecompositionPrototype'])
        self.assertEqual([row['piece'] for row in replay['pieces']],
                         [row['piece'] for row in accepted['pieces']])

        # Nearest-facet ties can select different samples on different platforms.
        # The exact proofs must still reproduce the accepted bounds to millimetres.
        limits = {'detailed': (558, 699), 'nativeCandidate': (1098, 1105)}
        for source, (lower, upper) in limits.items():
            with self.subTest(source=source):
                results = [p['sourceComparisons'][source] for p in replay['pieces']]
                witnesses = [row['largestExposedSample'] or {} for row in results]
                self.assertGreaterEqual(max(w.get('certifiedLowerMillimetres', 0) for w in witnesses),
                                        lower)
                self.assertLessEqual(max(row['fullPieceSurfaceUpperMillimetres'] for row in results),
                                     upper)
        self.assertGreaterEqual(replay['mandatoryCrossSourceVertices']
                                ['detailedVerticesToNativeCandidate'][0]['certifiedLowerMillimetres'], 1082)


if __name__ == '__main__':
    unittest.main()
