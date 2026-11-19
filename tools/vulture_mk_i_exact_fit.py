"""Integer certificates for the SH037 fit and original-mesh comparison."""
from collections import Counter
from fractions import Fraction
from itertools import pairwise
from math import isqrt

import numpy as np

import measure_goshawk_surface_fit as fit


def squared_distance(point, triangle):
    """Return the exact point-to-triangle squared distance as numerator and denominator."""
    a, b, c = triangle
    ab, ac, ap = fit.sub(b, a), fit.sub(c, a), fit.sub(point, a)
    aa, bb, cc = fit.dot(ab, ab), fit.dot(ab, ac), fit.dot(ac, ac)
    pa, pc = fit.dot(ap, ab), fit.dot(ap, ac)
    denom = aa * cc - bb * bb
    u, v = cc * pa - bb * pc, aa * pc - bb * pa
    if denom > 0 and u >= 0 and v >= 0 and u + v <= denom:
        height = fit.dot(ap, fit.cross(ab, ac))
        return height * height, denom
    best = None
    for start, end in pairwise((a, b, c, a)):
        ray, offset = fit.sub(end, start), fit.sub(point, start)
        length, advance = fit.dot(ray, ray), fit.dot(ray, offset)
        if advance <= 0:
            candidate = fit.dot(offset, offset), 1
        elif advance >= length:
            delta = fit.sub(point, end)
            candidate = fit.dot(delta, delta), 1
        else:
            candidate = fit.dot(offset, offset) * length - advance * advance, length
        if best is None or candidate[0] * best[1] < best[0] * candidate[1]:
            best = candidate
    return best


class TriangleIndex:
    """Bounding boxes prune exact queries without dropping equally near faces."""

    def __init__(self, triangles):
        self.triangles = triangles
        boxes = [tuple((min(axis), max(axis)) for axis in zip(*triangle)) for triangle in triangles]

        def partition(faces):
            bounds = tuple((min(boxes[i][k][0] for i in faces),
                            max(boxes[i][k][1] for i in faces)) for k in range(3))
            if len(faces) <= 8:
                return bounds, tuple(faces), ()
            axis = max(range(3), key=lambda k: bounds[k][1] - bounds[k][0])
            faces.sort(key=lambda i: (sum(boxes[i][axis]), i))
            mid = len(faces) // 2
            return bounds, (), (partition(faces[:mid]), partition(faces[mid:]))

        self.root = partition(list(range(len(triangles))))

    def nearest(self, point):
        def box_squared(box):
            return sum(max(lo - value, 0, value - hi)**2 for (lo, hi), value in zip(box, point))

        best = None
        ties = []

        def search(node, lower):
            nonlocal best
            if best is not None and lower * best[1] > best[0]:
                return
            _, faces, children = node
            for index in faces:
                num, den = squared_distance(point, self.triangles[index])
                order = -1 if best is None else num * best[1] - best[0] * den
                if order < 0:
                    best = num, den
                    ties[:] = [index]
                elif order == 0:
                    ties.append(index)
            for distance, child in sorted(((box_squared(c[0]), c) for c in children),
                                          key=lambda row: row[0]):
                search(child, distance)

        search(self.root, box_squared(self.root[0]))
        return best, sorted(ties)


def bound_fan(points, faces):
    """Certify the exact authored proxy faces for the rounded-hull patch bound."""
    cloud, scale = fit.dyadic(points)
    center_sum = tuple(map(sum, zip(*cloud)))
    radial_error = Fraction(0)
    edges = Counter()
    for face in faces:
        a, b, c = (cloud[int(i)] for i in face)
        normal = fit.cross(fit.sub(b, a), fit.sub(c, a))
        h = fit.dot(normal, a)
        from_center = h * len(cloud) - fit.dot(normal, center_sum)
        assert from_center > 0
        excess = max(fit.dot(normal, p) for p in cloud) - h
        radial_error = max(radial_error, Fraction(excess * len(cloud), from_center))
        edges.update(pairwise((*map(int, face), int(face[0]))))
    assert all(v == edges[(b, a)] == 1 for (a, b), v in edges.items())
    diameter = sum(max(col) - min(col) for col in zip(*cloud))
    # Outward radial error between the closed authored fan and its true convex hull.
    assert radial_error * diameter / scale < Fraction(1, 1_000_000)


def grid_index(mesh):
    cloud = np.rint(mesh.vertices * fit.GRID).astype(np.int64).tolist()
    return TriangleIndex([tuple(cloud[int(i)] for i in face) for face in mesh.faces])


def full_upper(index, fan, target_mm):
    """Bound every patch by distances to a single fixed reference triangle."""
    patches = np.rint(fan * fit.GRID).astype(np.int64)
    upper = 0
    for depth in range(fit.DEPTH + 1):
        if not len(patches):
            break
        margin = 2 * (depth + 4) + 2 + (fit.GRID + 49) // 50
        cutoff = target_mm * fit.GRID // 1000 - margin
        split_mask = np.zeros(len(patches), dtype=bool)
        for i, patch in enumerate(patches.tolist()):
            center = tuple(sum(axis) // 3 for axis in zip(*patch))
            _, ties = index.nearest(center)
            reference = index.triangles[ties[0]]
            # Distance to one convex triangle is convex, so corner distances
            # bound the entire patch. Ceiling the rational keeps it outward.
            distances = [squared_distance(vertex, reference) for vertex in patch]
            squared = max((num + den - 1) // den for num, den in distances)
            if depth < fit.DEPTH and (cutoff < 0 or squared > cutoff**2):
                split_mask[i] = True
            else:
                upper = max(upper, isqrt(squared) + 1 + margin)
        patches = fit.split(patches[split_mask], integer=True)
    return (upper * 1000 + fit.GRID - 1) // fit.GRID
