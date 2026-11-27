"""CLI measurements for fleet hulls with a detailed, archived and candidate native mesh."""

import argparse
from itertools import combinations
import json
import sys

import numpy as np
import trimesh

from directed_hull_fit import bind_directed_fit
from measure_banteng_geometry import adaptive_upper, upper
from vulture_mk_i_exact_fit import bound_fan, grid_index


def fidelity(meshes):
    indices = {name: grid_index(mesh) for name, mesh in meshes.items()}
    bounds = {}
    for first, second in combinations(meshes, 2):
        bounds[first + "To" + second[0].upper() + second[1:]] = upper(meshes[first], indices[second])
        bounds[second + "To" + first[0].upper() + first[1:]] = upper(meshes[second], indices[first])
    return {"continuousUpperMillimetres": bounds}


class HullMeasurement:
    def __init__(self, source, targets, group_error):
        self.source = source
        self.targets = targets
        self.group_error = group_error
        self.directed, self.directions = bind_directed_fit(source.load_source, source.SCALE)

    def load(self):
        _, _, _, _, names, raw = self.source.collision_inputs()
        vertices, groups, meshes = [], {name: [] for name in names}, {}
        for lod, (model, membership) in zip(("detailed", "archivedNative", "candidateNative"), raw, strict=True):
            offset = len(vertices)
            vertices.extend(model["vertices"])
            meshes[lod] = trimesh.Trimesh(model["vertices"], model["faces"], process=False)
            for vertex_index, slots in enumerate(membership, start=offset):
                for slot in slots:
                    groups[names[slot]].append(vertex_index)
        return np.array(vertices), meshes, {name: np.array(ids) for name, ids in groups.items()}

    def fit(self, vertices, meshes, groups):
        fans = json.loads((self.source.INTEGRATION / "SUPPORT-FAN.json").read_bytes())
        indices = {name: grid_index(mesh) for name, mesh in meshes.items()}
        per_group = {}
        for name, faces in fans.items():
            ids = groups[name]
            local = np.searchsorted(ids, faces)
            self.source.require(np.all(local < len(ids)) and np.array_equal(ids[local], faces), self.group_error)
            bound_fan(vertices[ids], local)
            per_group[name] = {lod: adaptive_upper(index, vertices[faces], self.targets[lod])
                               for lod, index in indices.items()}
        return {
            "nominalPaddingMetres": self.source.RADIUS,
            "minimumRefinementTargetsMillimetres": dict(self.targets),
            "fullUpperMillimetres": {lod: max(row[lod] for row in per_group.values()) for lod in meshes},
            "pieces": per_group,
        }

    def main(self, description, fidelity_measure=None):
        parser = argparse.ArgumentParser(description=description)
        mode = parser.add_mutually_exclusive_group()
        if fidelity_measure is not None:
            mode.add_argument("--fidelity", action="store_true")
        mode.add_argument("--directions", action="store_true")
        mode.add_argument("--reference-directions", action="store_true")
        args = parser.parse_args()
        if args.directions or args.reference_directions:
            result = self.directions(args.reference_directions)
        else:
            data = self.load()
            if fidelity_measure is not None and args.fidelity:
                result = fidelity_measure(data[1])
            else:
                result = self.fit(*data)
        sys.stdout.buffer.write((json.dumps(result, indent=2) + "\n").encode())
