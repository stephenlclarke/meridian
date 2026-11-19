"""Inspect distances from displayed piece surfaces to the detailed visible mesh."""

from math import sqrt
from statistics import fmean

from definitions import parse, reference_bytes, reference_fit


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def offset(a, b, t):
    return tuple(x + t * y for x, y in zip(a, b))


def closest(point, face):
    """Closest point on a triangle using barycentric Voronoi regions."""
    a, b, c = face
    ab, ac, ap = sub(b, a), sub(c, a), sub(point, a)
    d1, d2 = dot(ab, ap), dot(ac, ap)
    if d1 <= 0 and d2 <= 0:
        return a
    bp = sub(point, b)
    d3, d4 = dot(ab, bp), dot(ac, bp)
    if d3 >= 0 and d4 <= d3:
        return b
    vc = d1 * d4 - d3 * d2
    if vc <= 0 and d1 >= 0 and d3 <= 0:
        return offset(a, ab, d1 / (d1 - d3))
    cp = sub(point, c)
    d5, d6 = dot(ab, cp), dot(ac, cp)
    if d6 >= 0 and d5 <= d6:
        return c
    vb = d5 * d2 - d1 * d6
    if vb <= 0 and d2 >= 0 and d6 <= 0:
        return offset(a, ac, d2 / (d2 - d6))
    va = d3 * d6 - d5 * d4
    if va <= 0 and d4 >= d3 and d5 >= d6:
        return offset(b, sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6)))
    denom = va + vb + vc
    return offset(offset(a, ab, vb / denom), ac, vc / denom)


def tree(faces):
    lo = tuple(min(min(p[i] for p in f) for f in faces) for i in range(3))
    hi = tuple(max(max(p[i] for p in f) for f in faces) for i in range(3))
    if len(faces) <= 8:
        return lo, hi, faces, ()
    axis = max(range(3), key=lambda i: hi[i] - lo[i])
    faces.sort(key=lambda f: sum(p[axis] for p in f))
    mid = len(faces) // 2
    return lo, hi, (), (tree(faces[:mid]), tree(faces[mid:]))


def box_distance_sq(point, node):
    return sum(
        max(lo - p, 0, p - hi) ** 2 for p, lo, hi in zip(point, node[0], node[1])
    )


def nearest(point, node, best=(float("inf"), None)):
    for face in node[2]:
        candidate = closest(point, face)
        delta = sub(point, candidate)
        d2 = dot(delta, delta)
        if d2 < best[0]:
            best = d2, candidate
    for child in sorted(node[3], key=lambda n: box_distance_sq(point, n)):
        if box_distance_sq(point, child) <= best[0]:
            best = nearest(point, child, best)
    return best


def fit_report(package, envelope):
    # Fit evidence binds geometry and grouping, independently of name and palette.
    original = parse(reference_bytes())
    if all(
        package[key] == original[key]
        for key in ("detail", "native", "metresPerUnit", "roundingMetres")
    ):
        summary = reference_fit()
        sample = summary["largestSample"]
        return {
            "maxMetres": sample["metres"],
            "point": sample["point"],
            "nearestPoint": sample["closestPoint"],
            "upperBoundMetres": summary["fullUpperMillimetres"] / 1000,
            "method": "Verified Peregrine exposed-surface sample; detailed mesh",
        }
    scale = package["metresPerUnit"]
    mesh = package["detail"]
    vertices = [[x * scale for x in p] for p in mesh["vertices"]]
    bvh = tree([[vertices[i] for i in f] for f in mesh["triangles"]])
    ev, faces = envelope["vertices"], envelope["triangles"]
    if not faces:
        return None
    # A bounded deterministic sample keeps editing responsive. Interior overlap
    # and unsampled concavities preclude interpreting this as a union bound.
    step = max(1, (len(faces) + 255) // 256)
    samples = [
        tuple(fmean(s) for s in zip(*(ev[i] for i in face))) for face in faces[::step]
    ]
    samples.extend(ev[:: max(1, (len(ev) + 255) // 256)])
    sq, point, other = -1, None, None
    for sample in samples:
        d2, hit = nearest(sample, bvh)
        if d2 > sq:
            sq, point, other = d2, sample, hit
    return {
        "maxMetres": sqrt(sq),
        "point": point,
        "nearestPoint": other,
        "samples": len(samples),
        "method": "Sampled display piece surfaces to detailed mesh, including overlaps; no global bound",
    }
