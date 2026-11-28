"""Bound the surface of every local rounded Cheetah support against each original mesh."""
from collections import defaultdict
import json
import sys

import numpy as np
from scipy.spatial import ConvexHull
import trimesh

from measure_banteng_geometry import upper
from prepare_cheetah_shield import definition, inputs
from vulture_mk_i_exact_fit import grid_index


def measure():
    package = definition()
    source, _, _, _ = inputs()
    scale = package["metresPerUnit"]
    references = {
        name: grid_index(trimesh.Trimesh(np.array(source[name]["vertices"])*scale,
                                        source[name]["faces"], process=False))
        for name in ("design", "nativeDesign")
    }
    by_group = defaultdict(list)
    for mesh in (package["detail"], package["native"]):
        for p, group in zip(mesh["vertices"], mesh["groups"], strict=True):
            by_group[group].append(p)
    rows = []
    for group, coordinates in sorted(by_group.items()):
        points = np.array(coordinates)*scale
        triangles = ConvexHull(points).simplices
        hull = trimesh.Trimesh(points, triangles, process=False)
        # The rounding ball moves a support point by at most 20 mm.
        rows.append({"group": group, **{name: upper(hull, index) + 20 for name, index in references.items()}})
    return {"units": "mm", "includesBuriedFaces": True, "groups": len(rows),
            "roundedSurfaceUpper": {name: max(row[name] for row in rows) for name in references},
            "perGroupUpper": rows}


if __name__ == "__main__":
    sys.stdout.buffer.write((json.dumps(measure(), indent=2) + "\n").encode())
