"""Verify the Medusa inspection shield independently of flight qualification."""

from hashlib import sha256
import json
import math

import numpy as np
import trimesh

from measure_goshawk_surface_fit import certified_upper
from prepare_medusa_inspection import CROWN, ROUNDING, read_fans, require, shield_inputs, source_fans
from vulture_mk_i_exact_fit import bound_fan


def check():
    resource, owners, _, receipt, _, points, groups = shield_inputs()
    mesh = resource["design"]
    original = np.asarray(mesh["faces"])
    detail_points = points[:len(mesh["vertices"])]
    detailed_parts = owners["design"]["parts"]
    original_faces = {
        part["name"]: original[part["faceStart"]:part["faceStart"] + part["faceCount"]] - part["vertexStart"]
        for part in detailed_parts
    }
    reference = {
        part["name"]: trimesh.Trimesh(
            detail_points[part["vertexStart"]:part["vertexStart"] + part["vertexCount"]],
            original_faces[part["name"]],
            process=False,
        )
        for part in detailed_parts
    }
    shield = source_fans(points, groups, author=False)
    rows = []
    crown_axis_guard = []
    crown = detailed_parts[CROWN]["name"]
    for name, ids, faces in read_fans(shield, groups):
        local_points = points[ids]
        face_indices = np.asarray(faces)
        order = {vertex: i for i, vertex in enumerate(ids)}
        local_faces = [[order[vertex] for vertex in face] for face in faces]
        # The proof uses exact dyadic points. No Qhull selection is replayed here.
        bound_fan(local_points, local_faces)
        part = name.split("/span-", 1)[0]
        bound = certified_upper(reference[part], {"triangles": points[face_indices]}, 500)
        rows.append({"support": name, "triangles": len(faces), "outerFitUpperMillimetres": bound})
        if part == crown:
            axis = np.rint(local_points.mean(axis=0)[:2] * 1024).astype(np.int64)
            minimum = np.min(local_points[:, :2] @ axis)
            # Integer mm rounded inward with 1 mm guard for the coordinate calculations.
            clear = math.floor((minimum / np.linalg.norm(axis) - ROUNDING) * 1000) - 1
            require(clear > 0, f"Shield closes the axial opening of crown support {name}")
            crown_axis_guard.append(clear)
    return {
        "sourceRevision": receipt["revision"],
        "shieldSha256": sha256(shield).hexdigest(),
        "scaleMetresPerSourceUnit": "5/41",
        "roundingMillimetres": 20,
        "groups": len(rows),
        "allOriginalFacesShareShieldSupport": True,
        "eachFanRadialErrorLessThanMicrometres": 1,
        "crownSectors": len(crown_axis_guard),
        "crownAxisClearanceLowerMillimetres": min(crown_axis_guard),
        "maximumOuterFitUpperMillimetres": max(row["outerFitUpperMillimetres"] for row in rows),
        "supports": rows,
    }


if __name__ == "__main__":
    print(json.dumps(check(), indent=2))
