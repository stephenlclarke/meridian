"""Measure directed mesh distances on the exact dyadic lattice used by fleet receipts."""

from fractions import Fraction
from functools import lru_cache
from heapq import heappop, heappush
from math import isqrt

from measure_goshawk_surface_fit import dyadic
from vulture_mk_i_exact_fit import TriangleIndex, squared_distance


DEPTH = 12
TOLERANCE_MM = 25


def _bounds(vertices):
    return [[min(axis), max(axis)] for axis in zip(*vertices, strict=True)]


def _owner(parts, face):
    return next((part["index"], part["name"]) for part in parts
                if part["faceStart"] <= face < part["faceStart"] + part["faceCount"])


def _subdivide(triangle):
    a, b, c = triangle
    ab, bc, ac = (tuple((v + w) // 2 for v, w in zip(p, q, strict=True))
                  for p, q in ((a, b), (b, c), (a, c)))
    return (a, ab, ac), (ab, b, bc), (ac, bc, c), (ab, bc, ac)


class _PatchSearch:
    def __init__(self, target_triangles, first_corner):
        index = TriangleIndex(target_triangles)
        self.nearest = lru_cache(maxsize=100000)(index.nearest)
        self.target_triangles = target_triangles
        self.pending = []
        self.serial = 0
        distance, ties = self.nearest(first_corner)
        self.best = Fraction(*distance)
        self.witness = (0, first_corner, ties)

    def examine(self, triangle, face, depth):
        for point in triangle:
            raw, ties = self.nearest(point)
            value = Fraction(*raw)
            if value > self.best:
                self.best = value
                self.witness = face, point, ties

        # The lattice centroid selects a reference triangle. Only the corners set the bound.
        centre = tuple(sum(axis) // 3 for axis in zip(*triangle, strict=True))
        _, ties = self.nearest(centre)
        reference = self.target_triangles[ties[0]]
        cap = max(Fraction(*squared_distance(point, reference)) for point in triangle)
        self.serial += 1
        heappush(self.pending, (-cap, self.serial, face, depth, triangle))

    def refine(self, tolerance):
        explored = 0
        while self.pending:
            cap = -self.pending[0][0]
            # Flooring the lattice square root keeps the stopping inequality conservative.
            lower = isqrt(self.best.numerator // self.best.denominator)
            if cap <= (lower + tolerance) ** 2:
                break
            _, _, face, depth, triangle = heappop(self.pending)
            assert depth < DEPTH, (face, depth)
            for patch in _subdivide(triangle):
                self.examine(patch, face, depth + 1)
            explored += 1
        return explored


def _directed(first, second, metres_per_unit):
    source, source_parts = first
    target, target_parts = second
    if len(source["faces"]) == 0:
        raise ValueError("Directed fit requires a measured source face")

    units = [tuple(x * float(metres_per_unit) for x in v)
             for v in source["vertices"] + target["vertices"]]
    lattice, base = dyadic(units)
    scale = base * 2**DEPTH
    points = [tuple(x * 2**DEPTH for x in point) for point in lattice]
    offset = len(source["vertices"])
    target_triangles = [tuple(points[offset + i] for i in face) for face in target["faces"]]

    search = _PatchSearch(target_triangles, points[source["faces"][0][0]])
    for face, corners in enumerate(source["faces"]):
        search.examine(tuple(points[i] for i in corners), face, 0)
    explored = search.refine(Fraction(TOLERANCE_MM * scale, 1000))

    # Each refinement replaces one patch with four, so a measured source retains a bound.
    upper = -search.pending[0][0]
    face, point, ties = search.witness
    lower_mm = isqrt((search.best * 1000000 / (scale * scale)) // 1)
    scaled_upper = upper * 1000000 / (scale * scale)
    upper_mm = isqrt(scaled_upper // 1)
    upper_mm += upper_mm * upper_mm < scaled_upper
    target_ref = [[float(Fraction(x, scale)) for x in v] for v in target_triangles[ties[0]]]
    return {
        "directedMaximumMillimetres": {"lowerFloor": lower_mm, "upperCeiling": upper_mm},
        "sourceWitness": {
            "face": face, "part": _owner(source_parts, face),
            "pointMetres": [float(Fraction(v, scale)) for v in point],
            "exactPointMetres": [str(Fraction(v, scale)) for v in point],
            "squaredDistanceMetres": str(search.best / (scale * scale)),
        },
        "nearestTarget": {
            "faces": ties, "parts": [_owner(target_parts, i) for i in ties],
            "oneTriangleBoxMetres": _bounds(target_ref),
        },
        "refinedPatches": explored,
    }


def bind_directed_fit(load_source, metres_per_unit):
    """Bind the public directed and directions entry points to one hull's source and scale."""
    def directed(first, second):
        return _directed(first, second, metres_per_unit)

    def directions(reference_only):
        _, _, _, raw, parts = load_source()
        detail, archive, candidate = zip(raw, parts, strict=True)
        if reference_only:
            return {"detailedToArchive": directed(detail, archive),
                    "archiveToDetailed": directed(archive, detail)}
        return {
            "archiveToCandidate": directed(archive, candidate),
            "candidateToArchive": directed(candidate, archive),
            "detailedToCandidate": directed(detail, candidate),
            "candidateToDetailed": directed(candidate, detail),
            "axisExtremaMetres": {
                label: _bounds([[x * float(metres_per_unit) for x in p] for p in mesh["vertices"]])
                for label, (mesh, _) in (("detailed", detail), ("archive", archive), ("candidate", candidate))
            },
        }

    return directed, directions
