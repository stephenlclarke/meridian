"""Headless geometric checks of the illustrative inspector overlay."""

from collections import Counter
from itertools import product
from math import hypot
import random
import unittest

from hull_studio.surface import envelope_view


def package(detail, native, scale=1.0, radius=0.5):
    def mesh(entries):
        return {"vertices": [list(point) for point, _ in entries],
                "groups": [group for _, group in entries], "triangles": []}
    return {"schema": 1, "metresPerUnit": scale, "roundingMetres": radius,
            "detail": mesh(detail), "native": mesh(native)}


class EnvelopeViewTests(unittest.TestCase):
    def check_closed(self, view):
        vertices, triangles = view['vertices'], view['triangles']
        edges = Counter()
        directed = Counter()
        used = set()
        for a, b, c in triangles:
            self.assertEqual(len({a, b, c}), 3)
            self.assertTrue(all(0 <= index < len(vertices) for index in (a, b, c)))
            used.update((a, b, c))
            for start, end in ((a, b), (b, c), (c, a)):
                edges[min(start, end), max(start, end)] += 1
                directed[start, end] += 1
        self.assertTrue(triangles)
        self.assertEqual(used, set(range(len(vertices))))
        self.assertTrue(all(count == 2 for count in edges.values()))
        self.assertTrue(all(directed[a, b] == directed[b, a] for a, b in edges))

    def test_one_point_rounding_in_metres_is_a_closed_inscribed_sphere(self):
        view = envelope_view(package([((10, -2, 4), 0)], [], scale=2, radius=1.3))
        self.check_closed(view)
        centre = (20, -4, 8)
        for point in view['vertices']:
            self.assertAlmostEqual(hypot(*(a-b for a, b in zip(point, centre))), 1.3, places=10)
        for a, b, c in view['triangles']:
            u, v, w = [tuple(x-y for x, y in zip(view['vertices'][i], centre)) for i in (a, b, c)]
            # Signed tetrahedral volume from the centre: every face points out.
            cross = (v[1]*w[2]-v[2]*w[1], v[2]*w[0]-v[0]*w[2], v[0]*w[1]-v[1]*w[0])
            self.assertGreater(sum(x*y for x, y in zip(u, cross)), 0)
        for axis in range(3):
            axis_values = [point[axis] for point in view['vertices']]
            self.assertAlmostEqual(min(axis_values), centre[axis] - 1.3)
            self.assertAlmostEqual(max(axis_values), centre[axis] + 1.3)

    def test_shared_detail_native_group_forms_capsule_not_independent_spheres(self):
        view = envelope_view(package([((-2, 0, 0), 5)], [((2, 0, 0), 5)], scale=3))
        self.check_closed(view)
        vertices = view['vertices']
        self.assertEqual((min(p[0] for p in vertices), max(p[0] for p in vertices)), (-6.5, 6.5))
        # Every support point is on the analytic capsule.
        for x, y, z in vertices:
            self.assertAlmostEqual(hypot(x - min(6, max(-6, x)), y, z), 0.5, places=10)
        self.assertEqual(len(vertices) - len(view['triangles']) // 2, 2)

    def test_disjoint_groups_do_not_generate_bridges_and_order_is_stable(self):
        detail = [((-10, 0, 0), 2), ((10, 0, 0), 8)]
        native = [((-10, 1, 0), 2), ((10, 1, 0), 8)]
        output = envelope_view(package(detail, native))
        self.check_closed(output)
        self.assertEqual(output, envelope_view(package(detail[::-1], native[::-1])))
        for tri in output['triangles']:
            self.assertEqual(len({output['vertices'][index][0] < 0 for index in tri}), 1)
        self.assertEqual(len(output['vertices']) - len(output['triangles']) // 2, 4)

    def test_rounded_cube_contains_only_points_in_analytic_offset(self):
        corners = list(product((-2, 2), repeat=3))
        view = envelope_view(package([(p, 0) for p in corners], [], radius=0.25))
        self.check_closed(view)
        for point in view['vertices']:
            self.assertAlmostEqual(hypot(*(max(0, abs(value)-2) for value in point)), 0.25)
        for axis in range(3):
            self.assertAlmostEqual(min(p[axis] for p in view['vertices']), -2.25)
            self.assertAlmostEqual(max(p[axis] for p in view['vertices']), 2.25)

    def test_zero_radius_tetrahedron_and_planar_group(self):
        points = [(0, 0, 0), (1, 0, 0), (0, 2, 0), (0, 0, 3)]
        view = envelope_view(package([(p, 0) for p in points], [], radius=0))
        self.check_closed(view)
        self.assertEqual({tuple(p) for p in view['vertices']}, set(points))
        with self.assertRaisesRegex(ValueError, 'three-dimensional extent'):
            envelope_view(package([(p, 0) for p in points[:3]], [], radius=0))
        self.check_closed(envelope_view(package([(p, 0) for p in points[:3]], [], radius=0.1)))

    def test_invalid_package_input_is_not_silently_repaired(self):
        for data, message in [
            (package([], []), 'No local support'),
            (package([((0, 0, 0), -1)], []), 'group ID'),
            (package([((0, 0, 0), True)], []), 'group ID'),
            (package([((0, 0, float('nan')), 0)], []), 'vertex'),
            (package([((0, 0, 0), 0)], [], scale=0), 'positive metresPerUnit'),
            (package([((0, 0, 0), 0)], [], radius=-1), 'nonnegative roundingMetres'),
            (package([((1e308, 0, 0), 0)], [], scale=2), 'metre-space vertex'),
        ]:
            with self.subTest(message=message), self.assertRaisesRegex(ValueError, message):
                envelope_view(data)

    def test_varied_seeded_groups_remain_closed(self):
        rng = random.Random(1024)
        for count in (1, 2, 3, 4, 8, 20, 100):
            for radius in (0.02, 0.2, 2.0):
                with self.subTest(count=count, radius=radius):
                    entries = [(tuple(rng.uniform(-50, 50) for _ in range(3)), 0) for _ in range(count)]
                    view = envelope_view(package(entries, [], radius=radius))
                    self.check_closed(view)


if __name__ == '__main__':
    unittest.main()
