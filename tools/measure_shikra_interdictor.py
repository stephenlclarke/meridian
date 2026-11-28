"""Measure SH095 local supports against the three calibrated meshes."""
import argparse
from itertools import combinations
import json
import sys

import numpy as np
import trimesh

from measure_banteng_geometry import adaptive_upper, upper
from prepare_shikra_interdictor import INTEGRATION, RADIUS, collision_inputs, require
from vulture_mk_i_exact_fit import bound_fan, grid_index


def load():
    _, _, _, _, names, raw = collision_inputs()
    vertices, groups, meshes = [], {name:[] for name in names}, {}
    for lod,(model,membership) in zip(("detailed","archivedNative","candidateNative"),raw,strict=True):
        offset = len(vertices)
        vertices.extend(model["vertices"])
        meshes[lod] = trimesh.Trimesh(model["vertices"],model["faces"],process=False)
        for index,slots in enumerate(membership):
            for slot in slots:
                groups[names[slot]].append(offset+index)
    return np.array(vertices), meshes, {name:np.array(ids) for name,ids in groups.items()}


def fit(vertices, meshes, groups):
    fans = json.loads((INTEGRATION/"SUPPORT-FAN.json").read_bytes())
    targets = {"detailed":2600,"archivedNative":2600,"candidateNative":3000}
    indices = {name:grid_index(mesh) for name,mesh in meshes.items()}
    per_group = {}
    for name,faces in fans.items():
        i = groups[name]
        local = np.searchsorted(i,faces)
        require(np.all(local < len(i)) and np.array_equal(i[local],faces), "Shikra Interdictor support face spans groups")
        bound_fan(vertices[i],local)
        per_group[name] = {lod:adaptive_upper(index,vertices[faces],targets[lod]) for lod,index in indices.items()}
    return {"nominalPaddingMetres":RADIUS,"minimumRefinementTargetsMillimetres":targets,
            "fullUpperMillimetres":{lod:max(record[lod] for record in per_group.values()) for lod in meshes},
            "pieces":per_group}


def fidelity(meshes):
    indices = {name: grid_index(mesh) for name, mesh in meshes.items()}
    bounds = {}
    for first, second in combinations(meshes, 2):
        bounds[first + "To" + second[0].upper() + second[1:]] = upper(meshes[first], indices[second])
        bounds[second + "To" + first[0].upper() + first[1:]] = upper(meshes[second], indices[first])
    return {"continuousUpperMillimetres": bounds}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fidelity", action="store_true")
    args = parser.parse_args()
    data = load()
    result = fidelity(data[1]) if args.fidelity else fit(*data)
    sys.stdout.buffer.write((json.dumps(result, indent=2) + "\n").encode())
