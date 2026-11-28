"""Certify the one-piece Newt envelope against its two original surface domains."""
import hashlib
from itertools import combinations
import json
from math import gcd
import sys

import numpy as np
import trimesh

import measure_goshawk_surface_fit as fit
from prepare_newt_escape_pod_studio import OUTPUT, definition, require
from vulture_mk_i_exact_fit import bound_fan, full_upper, grid_index


FAN_SHA256 = "ceef2a633361b262ad7feff2c6a4658244ebb4b928aa620ac4e614a347d61e6b"


def supporting_planes(shell):
    """Enumerate exact supporting planes including alternative coplanar diagonals."""
    planes = set()
    for a, b, c in combinations(shell, 3):
        normal = fit.cross(fit.sub(b, a), fit.sub(c, a))
        divisor = gcd(*normal)
        if not divisor:
            continue
        normal = tuple(k // divisor for k in normal)
        limit = fit.dot(normal, a)
        dots = [fit.dot(normal, p) for p in shell]
        if max(dots) == limit:
            planes.add((*normal, limit))
        elif min(dots) == limit:
            planes.add((*(-k for k in normal), -limit))
    return planes


def certificate():
    package = definition()
    require(package == json.loads(OUTPUT.read_bytes()), "Changed Newt Studio source")
    detail, native = (package[key] for key in ("detail", "native"))
    require(package["roundingMetres"] == fit.RADIUS
            and set(detail["groups"] + native["groups"]) == {0}, "Changed one-piece Newt envelope")
    scale = package["metresPerUnit"]
    shell = np.asarray(native["vertices"]) * scale
    detailed = np.asarray(detail["vertices"]) * scale
    points, _ = fit.dyadic(np.vstack((shell, detailed)))
    planes = supporting_planes(points[:len(shell)])
    require(len(planes) >= 4, "Missing full-dimensional Newt shell")
    for x, y, z, limit in planes:
        require(all(fit.dot((x, y, z), p) <= limit for p in points[len(shell):]),
                "Detailed point outside the closed Newt shell")
    raw = OUTPUT.with_name("SUPPORT-FAN.json").read_bytes()
    require(hashlib.sha256(raw).hexdigest() == FAN_SHA256, "Changed Newt support fan")
    faces = np.asarray(json.loads(raw), dtype=np.int64)
    require(faces.shape == (92, 3) and faces.min() >= 0 and faces.max() < len(shell),
            "Wrong Newt support-fan topology")
    bound_fan(shell, faces)
    # The outermost native vertex in +Z belongs to both meshes. Its offset
    # along +Z lies on the rounded hull exactly 20 mm from both source meshes.
    vertex = int(np.argmax(shell[:, 2]))
    require(np.array_equal(shell[vertex], detailed[vertex]) and
            np.all(detailed[:, 2] <= shell[vertex, 2]), "Lost the shared forward support vertex")
    # Two midpoint splits precede the standard integer-grid certificate. All
    # corners lie within two metres. One extra millimetre bounds the binary64
    # midpoint roundings before the existing grid and 20 mm radius allowances.
    require(np.max(np.abs(shell)) < 2, "Newt coordinates exceed the subdivision bound")
    patches = fit.split(fit.split(shell[faces]))
    bounds = {}
    for label, vertices, mesh in (("detailed", detailed, detail), ("native", shell, native)):
        reference = trimesh.Trimesh(vertices, mesh["triangles"], process=False)
        bounds[label] = full_upper(grid_index(reference), patches, 28) + 1
    require(max(bounds.values()) <= 29, "Newt fit exceeds the authored upper bound")
    return {"nativeVertices": len(shell), "detailedVertices": len(detailed),
            "exactSupportPlanes": len(planes), "supportFanTriangles": len(faces),
            "sharedForwardVertex": vertex, "exposedLowerMillimetres": 20,
            "completeUpperMillimetres": bounds}


if __name__ == "__main__":
    sys.stdout.buffer.write((json.dumps(certificate(), indent=2) + "\n").encode("utf-8"))
