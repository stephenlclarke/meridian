"""Measure SH087 local supports against the three calibrated meshes."""
import json
import sys

import numpy as np
import trimesh

from measure_banteng_geometry import adaptive_upper
from prepare_elk import INTEGRATION, RADIUS, collision_inputs, require
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
        require(np.all(local < len(i)) and np.array_equal(i[local],faces), "Elk support face spans groups")
        bound_fan(vertices[i],local)
        per_group[name] = {lod:adaptive_upper(index,vertices[faces],targets[lod]) for lod,index in indices.items()}
    return {"nominalPaddingMetres":RADIUS,"minimumRefinementTargetsMillimetres":targets,
            "fullUpperMillimetres":{lod:max(record[lod] for record in per_group.values()) for lod in meshes},
            "pieces":per_group}


if __name__ == "__main__":
    sys.stdout.buffer.write((json.dumps(fit(*load()),indent=2)+"\n").encode())
