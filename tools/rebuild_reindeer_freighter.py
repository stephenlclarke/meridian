"""Rebuild the Reindeer Freighter inspection mesh from its pinned source selections."""
import hashlib
import json

import numpy as np
import trimesh

from measure_banteng_geometry import solid
from prepare_reindeer_freighter import INTEGRATION, ORIGINAL, SCALE, load_source, require
from verify_reindeer_freighter import section
from vulture_mk_i_exact_fit import bound_fan


REBUILD_SELECTIONS_SHA = "7da045fdef32ff18c9ffc24d9c75c8e3a6967df6bee3ce9f84c64723ad01c75c"


def rebuilt_section(row, ids, record):
    vertices, faces, materials, mappings = row
    src = np.asarray(vertices)
    chosen = src[ids]
    patch = record["faces"]
    require(all(len(face) == 3 and all(type(i) is int and 0 <= i < len(ids) for i in face) for face in patch),
            "Invalid Reindeer loft triangle")
    bound_fan(chosen * float(SCALE), patch)
    mesh = trimesh.Trimesh(chosen, patch, process=False)
    solid(mesh)
    original = trimesh.Trimesh(src, faces, process=False)
    nearest = record["sourceFaceIndices"]
    require(len(nearest) == len(patch) and all(type(i) is int and 0 <= i < len(faces) for i in nearest), "Invalid source face selection")
    centers = mesh.triangles_center
    _, distances, _ = original.nearest.on_surface(centers)
    reference_points = trimesh.triangles.closest_point(original.triangles[nearest], centers)
    delta = reference_points - centers
    squared = np.einsum("ij,ij->i", delta, delta)
    # Trimesh compares squared distances using tol.merge when it resolves adjacent surface faces.
    require(np.all(np.abs(squared - distances**2) < trimesh.constants.tol.merge),
            "Authored face does not lie on the nearest source surface")
    selected_materials = [materials[index] for index in nearest]
    require(set(selected_materials) == set(materials), "Rebuilt loft lost a source material")
    selected_mappings = []
    for face, index in zip(patch,nearest,strict=True):
        mapping = mappings[index]
        original_triangle = np.array(faces[index])
        closest = [np.linalg.norm(src[original_triangle]-chosen[i],axis=1).argmin() for i in face]
        selected_mappings.append({"texture": mapping["texture"], "uv": [mapping["uv"][i] for i in closest]})
    return [vertices[i] for i in ids],patch,selected_materials,selected_mappings


def rebuild_well(source, ids):
    vertices, original_faces, materials, mappings = source
    chosen = [vertices[index] for index in ids]
    faces = [[0, 1, 4], [1, 2, 4], [2, 0, 4], [0, 3, 1], [1, 3, 2], [2, 3, 0]]
    points = np.asarray(chosen) * float(SCALE)
    mesh = trimesh.Trimesh(points, faces, process=False)
    if mesh.volume < 0:
        faces = [list(reversed(face)) for face in faces]
        mesh = trimesh.Trimesh(points, faces, process=False)
    solid(mesh)
    uv = {}
    for face, mapping in zip(original_faces, mappings, strict=True):
        for source_vertex, source_uv in zip(face, mapping["uv"], strict=True):
            uv.setdefault(source_vertex, source_uv)
    indexed = [
        {"texture": mappings[16 if face_index < 3 else 0]["texture"], "uv": [uv[ids[vertex]] for vertex in face]}
        for face_index, face in enumerate(faces)
    ]
    return chosen, faces, [materials[16]]*3+[materials[0]]*3, indexed


def rebuild_four_corner_well(source, ids, record):
    vertices, faces, materials, mappings = source
    local_uv = {}
    for face, mapping in zip(faces, mappings, strict=True):
        for corner, coord in zip(face, mapping["uv"], strict=True):
            local_uv.setdefault(corner, coord)
    points = np.asarray(vertices) * float(SCALE)
    outer = points[ids[:-1]]
    rebuilt = record["faces"]
    rim = rebuilt[:4]
    tip = len(ids) - 1
    require(all(len(face) == 3 and face[2] == tip for face in rim), "Invalid recessed-well tip")
    following = {a: b for a, b, _ in rim}
    require(set(following) == set(following.values()) == set(range(4)), "Invalid recessed-well rim")
    loop = [0]
    for _ in range(3):
        loop.append(following[loop[-1]])
    require(len(set(loop)) == 4 and following[loop[-1]] == 0, "Disconnected recessed-well rim")
    back = rebuilt[4:]
    require(all(len(face) == 3 and all(type(i) is int and 0 <= i < tip for i in face) for face in back),
            "Invalid recessed-well outer triangle")
    cap = [[loop[0], loop[1], loop[2]], [loop[0], loop[2], loop[3]]]
    bound_fan(outer, back + cap)
    solid(trimesh.Trimesh(points[ids], rebuilt, process=False))
    material = [materials[16]] * 4 + [materials[0]] * len(back)
    mapped = [{"texture": mappings[16 if i < 4 else 0]["texture"],
               "uv": [local_uv[ids[j]] for j in triangle]} for i, triangle in enumerate(rebuilt)]
    return [vertices[idx] for idx in ids], rebuilt, material, mapped


def main():
    _, _, _, (_, original, _), (_, parts, _) = load_source()
    stored = (ORIGINAL / "GENERATED-COLLISION.json").read_bytes()
    candidate = json.loads(stored)
    raw = (INTEGRATION / "REBUILD-SELECTIONS.json").read_bytes()
    require(hashlib.sha256(raw).hexdigest() == REBUILD_SELECTIONS_SHA, "Changed Reindeer reconstruction selections")
    records = json.loads(raw)["parts"]
    wells = candidate["recessedWellSelections"]
    lofts = candidate["selectedSourceVertexIndices"]
    require(set(records) == set(lofts) | {k for k, ids in wells.items() if len(ids) != 5},
            "Missing or extra Reindeer reconstruction parts")
    model = original | {name: [] for name in ("vertices", "faces", "faceMaterials", "faceTextures")}
    bindings = []
    for part in parts:
        index = str(part["index"])
        source = section(original, part)
        if index in records:
            record = records[index]
            require(record["part"] == part["name"] and record["indices"] == (wells.get(index) or lofts[index]),
                    "Reindeer reconstruction record does not match source selection")
        if index in wells:
            ids = wells[index]
            source = rebuild_well(source, ids) if len(ids) == 5 else rebuild_four_corner_well(source, ids, records[index])
        elif index in lofts:
            source = rebuilt_section(source, lofts[index], records[index])
        vertices, faces, materials, mappings = source
        start = len(model["vertices"])
        bindings.append({key: part[key] for key in ("index", "name", "creatorMethod")} | {
            "vertexStart": start, "vertexCount": len(vertices),
            "faceStart": len(model["faces"]), "faceCount": len(faces),
        })
        model["vertices"].extend(vertices)
        model["faces"].extend([[corner + start for corner in face] for face in faces])
        model["faceMaterials"].extend(materials)
        model["faceTextures"].extend(mappings)
    rebuilt = candidate | {"nativeCandidate": model, "parts": bindings}
    output = (json.dumps(rebuilt, separators=(",", ":")) + "\n").encode()
    require(output == stored, "Reindeer Freighter candidate differs after source reconstruction")


if __name__ == "__main__":
    main()
