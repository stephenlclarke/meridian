"""Rebuild the Lynx Escort inspection mesh from its pinned source selections."""
import json

import numpy as np
from scipy.spatial import ConvexHull
import trimesh

from measure_banteng_geometry import solid
from prepare_lynx_escort import ORIGINAL, SCALE, load_source, require
from verify_lynx_escort import section


# Indexed in the archived pod's local vertex space. The original authoring
# run projected these two simplified facets onto opposite sides of one UV seam.
POD_SEAM = {
    (48, 51, 57): (48, 53, 54),
    (51, 54, 57): (48, 54, 55),
}


def bind_pod_seam(original, ids, patch, centers, nearest):
    source_faces = original.faces.tolist()
    sources = {face: source_faces.index(list(face)) for face in POD_SEAM.values()}
    candidates = [tuple(ids[i] for i in face) for face in patch]
    triangles = original.triangles[list(sources.values())]
    precision = np.spacing(np.max(np.abs(triangles)))
    for candidate, source in POD_SEAM.items():
        index = candidates.index(candidate)
        require(nearest[index] in sources.values(), "unexpected source face at Lynx pod seam")
        points = trimesh.triangles.closest_point(triangles, np.repeat(centers[index:index+1], 2, axis=0))
        require(np.max(np.abs(points[0] - points[1])) <= precision, "Lynx pod projections left the seam")
        nearest[index] = sources[source]


def rebuilt_section(row, ids, *, bind_seam=False):
    vertices, faces, materials, mappings = row
    src = np.asarray(vertices)
    chosen = src[ids]
    center = np.mean(chosen, axis=0)
    original = trimesh.Trimesh(src,faces,process=False)
    patch = []
    for a,b,c in ConvexHull(chosen).simplices.tolist():
        if np.dot(np.cross(chosen[b]-chosen[a],chosen[c]-chosen[a]), chosen[a]-center) < 0:
            b,c=c,b
        face = [a,b,c]
        shift = face.index(min(face))
        patch.append(face[shift:] + face[:shift])
    patch.sort()
    mesh = trimesh.Trimesh(chosen,patch,process=False)
    _, _, nearest = original.nearest.on_surface(mesh.triangles_center)
    if bind_seam:
        bind_pod_seam(original, ids, patch, mesh.triangles_center, nearest)
    # A simplified well can collapse several source texture faces into one
    # facet. Assign each missing material to the facet with the nearest
    # original surface carrying that material. Leave the original artwork intact.
    counts = {material: sum(materials[index] == material for index in nearest) for material in set(materials)}
    for material, count in counts.items():
        if count:
            continue
        source_indices = [i for i,value in enumerate(materials) if value == material]
        targets = [i for i,index in enumerate(nearest) if counts[materials[index]] > 1]
        assignment = min(
            ((np.linalg.norm(mesh.triangles_center[i]-original.triangles_center[j]), i, j)
             for i in targets for j in source_indices),
        )
        _, chosen_face, source_index = assignment
        counts[materials[nearest[chosen_face]]] -= 1
        nearest[chosen_face] = source_index
        counts[material] += 1
    selected_materials = [materials[index] for index in nearest]
    selected_mappings = []
    for face, index in zip(patch,nearest,strict=True):
        mapping = mappings[index]
        original_triangle = np.array(faces[index])
        closest = [np.linalg.norm(src[original_triangle]-chosen[i],axis=1).argmin() for i in face]
        selected_mappings.append({"texture": mapping["texture"], "uv": [mapping["uv"][i] for i in closest]})
    solid(mesh)
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


def main():
    _, _, _, (_, original, _), (_, parts, _) = load_source()
    stored = (ORIGINAL / "GENERATED-COLLISION.json").read_bytes()
    candidate = json.loads(stored)
    wells = candidate["recessedWellSelections"]
    lofts = candidate["selectedSourceVertexIndices"]
    model = original | {name: [] for name in ("vertices", "faces", "faceMaterials", "faceTextures")}
    bindings = []
    for part in parts:
        index = str(part["index"])
        source = section(original, part)
        if index in wells:
            source = rebuild_well(source, wells[index])
        elif index in lofts:
            source = rebuilt_section(source, lofts[index], bind_seam=part["name"] == "pods explicit member 0")
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
    require(output == stored, "Lynx Escort candidate differs after source reconstruction")


if __name__ == "__main__":
    main()
