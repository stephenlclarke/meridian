"""Illustrative, inscribed meshes of the game's local rounded support groups.

Each local group is sampled separately. Chords between support samples are not
the analytic rounded surface; the returned mesh must never certify clearance
or be used as collision geometry. Overlapping groups are not boolean-unioned.
"""

from math import hypot, isfinite


_RESOLUTION = 6
_EPSILON = 1e-10


def _subtract(a, b):
    return tuple(x - y for x, y in zip(a, b, strict=True))


def _dot(a, b):
    return sum(x * y for x, y in zip(a, b, strict=True))


def _cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0])


def _directions():
    """Uniform integer octahedron shell, normalized onto the unit sphere."""
    n = _RESOLUTION
    return tuple(tuple(value / hypot(x, y, z) for value in (x, y, z))
                 for x in range(-n, n + 1) for y in range(-n, n + 1)
                 for z in range(-n, n + 1) if abs(x) + abs(y) + abs(z) == n)


_DIRECTIONS = _directions()


def _face(points, a, b, c, inside):
    normal = _cross(_subtract(points[b], points[a]), _subtract(points[c], points[a]))
    length = hypot(*normal)
    if length <= _EPSILON ** 2:
        raise ValueError("Degenerate sampled envelope face")
    normal = tuple(value / length for value in normal)
    if _dot(normal, _subtract(inside, points[a])) > 0:
        b, c = c, b
        normal = tuple(-value for value in normal)
    return ((a, b, c), normal)


def _initial_simplex(points):
    a = 0
    b = max(range(len(points)), key=lambda i: hypot(*_subtract(points[i], points[a])))
    line = _subtract(points[b], points[a])
    line_length = hypot(*line)
    if line_length <= _EPSILON:
        raise ValueError("Sampled envelope has no three-dimensional extent")
    c = max(range(len(points)), key=lambda i: hypot(*_cross(line, _subtract(points[i], points[a]))))
    normal = _cross(line, _subtract(points[c], points[a]))
    if hypot(*normal) <= _EPSILON * line_length:
        raise ValueError("Sampled envelope has no three-dimensional extent")
    normal = tuple(value / hypot(*normal) for value in normal)
    d = max(range(len(points)), key=lambda i: abs(_dot(normal, _subtract(points[i], points[a]))))
    if abs(_dot(normal, _subtract(points[d], points[a]))) <= _EPSILON:
        raise ValueError("Sampled envelope has no three-dimensional extent")
    return a, b, c, d


def _convex_hull(points):
    """Incremental outward triangulation of a small normalized sample cloud."""
    a, b, c, d = _initial_simplex(points)
    seed = (a, b, c, d)
    inside = tuple(sum(points[i][axis] for i in seed) / 4 for axis in range(3))
    faces = [_face(points, *indices, inside) for indices in
             ((a, b, c), (a, b, d), (a, c, d), (b, c, d))]
    for i, point in enumerate(points):
        if i in seed:
            continue
        retained = []
        horizon = {}
        for triangle, normal in faces:
            a, b, c = triangle
            if _dot(normal, _subtract(point, points[a])) <= _EPSILON:
                retained.append((triangle, normal))
                continue
            for u, v in ((a, b), (b, c), (c, a)):
                key = (min(u, v), max(u, v))
                if key in horizon:
                    del horizon[key]
                else:
                    horizon[key] = (u, v)
        faces = retained + [_face(points, u, v, i, inside) for u, v in horizon.values()]
    return [triangle for triangle, _ in faces]


def _piece(points, radius):
    """Sample h_C(d) + r for a convex point group C, in local coordinates."""
    points = sorted(set(points))
    origin = points[0]
    offsets = [_subtract(point, origin) for point in points]
    size = max(radius, max(hypot(*point) for point in offsets))
    if not isfinite(size) or size == 0:
        raise ValueError("Rounded group has invalid extent")
    local = [tuple(value / size for value in point) for point in offsets]
    r = radius / size
    samples = set()
    for direction in _DIRECTIONS:
        support = max(local, key=lambda point: _dot(point, direction))
        samples.add(tuple(value + r * component for value, component in zip(support, direction, strict=True)))
    samples = sorted(samples)
    faces = _convex_hull(samples)
    used = sorted({index for face in faces for index in face})
    remap = {index: i for i, index in enumerate(used)}
    xyz = [[origin[axis] + size * samples[i][axis] for axis in range(3)] for i in used]
    if not all(isfinite(value) for point in xyz for value in point) or len({tuple(p) for p in xyz}) != len(xyz):
        raise ValueError("Rounded group cannot be represented in metre-space mesh")
    return xyz, [[remap[index] for index in face] for face in faces]


def _number(value, name):
    if type(value) not in (int, float):
        raise ValueError(f"Invalid {name}")
    try:
        value = float(value)
    except OverflowError as error:
        raise ValueError(f"Invalid {name}") from error
    if not isfinite(value):
        raise ValueError(f"Invalid {name}")
    return value


def envelope_view(package):
    """Return ``{"vertices": [[x,y,z]], "triangles": [[i,j,k]]}`` in metres.

    The detail and native group IDs refer to the same local supports.
    Call after authoritative package validation; this overlay validates only its
    own inputs and deliberately does not reproduce runtime semantic checks.
    A non-volumetric or unrepresentable sampled group raises ValueError.
    """
    if not isinstance(package, dict) or type(package.get("schema")) is not int or package["schema"] not in (1, 2):
        raise ValueError("Expected a schema-1 or archived inspection hull package")
    scale = _number(package.get("metresPerUnit"), "metresPerUnit")
    radius = _number(package.get("roundingMetres"), "roundingMetres")
    if scale <= 0 or radius < 0:
        raise ValueError("Expected positive metresPerUnit and nonnegative roundingMetres")
    groups = {}
    for domain in ("detail", "native"):
        mesh = package.get(domain)
        if not isinstance(mesh, dict):
            raise ValueError(f"Missing {domain} mesh")
        vertices, membership = mesh.get("vertices"), mesh.get("groups")
        if not isinstance(vertices, list) or not isinstance(membership, list) or len(vertices) != len(membership):
            raise ValueError(f"Invalid {domain} group membership")
        for vertex, group in zip(vertices, membership, strict=True):
            if type(group) is not int or group < 0:
                raise ValueError(f"Invalid {domain} group ID")
            if not isinstance(vertex, list) or len(vertex) != 3:
                raise ValueError(f"Invalid {domain} vertex")
            point = tuple(_number(value, f"{domain} vertex") * scale for value in vertex)
            if not all(isfinite(value) for value in point):
                raise ValueError(f"Invalid {domain} metre-space vertex")
            groups.setdefault(group, []).append(point)
    if not groups:
        raise ValueError("No local support groups")
    vertices, triangles = [], []
    for group in sorted(groups):
        points, faces = _piece(groups[group], radius)
        triangles.extend([[index + len(vertices) for index in face] for face in faces])
        vertices.extend(points)
    return {"vertices": vertices, "triangles": triangles}
