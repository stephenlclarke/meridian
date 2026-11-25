"""Rebuild the Osprey Support inspection mesh from its pinned source selections."""
import json

import numpy as np
from scipy.spatial import ConvexHull
import trimesh

from measure_banteng_geometry import solid
from prepare_osprey_support import ORIGINAL, SCALE, load_source, require
from verify_osprey_support import section


def rebuilt_section(row, ids):
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


def physical(section):
    vertices,faces,_,_=section
    return trimesh.Trimesh(np.asarray(vertices)*SCALE,faces,process=False)


def reduced_arc(source,rows,cross,closed):
    vertices, original_faces, materials, uv = source
    assert len(set(materials))==1
    assert len({row["texture"] for row in uv})==1
    uv_by_vertex={}
    for f, mapping in zip(original_faces,uv,strict=True):
        for a, coord in zip(f,mapping["uv"],strict=True):
            uv_by_vertex.setdefault(a,coord)
    ids=[row*8+side for row in rows for side in cross]
    corners=len(cross)
    faces=[]
    for row in range(len(rows) if closed else len(rows)-1):
        following=(row+1)%len(rows)
        for c in range(corners):
            a,b=row*corners+c,following*corners+c
            nxt=(c+1)%corners
            c2,d=following*corners+nxt,row*corners+nxt
            faces.extend([[a,b,c2],[a,c2,d]])
    if not closed:
        last=(len(rows)-1)*corners
        for k in range(1,corners-1):
            faces.extend([[0,k,k+1],[last,last+k+1,last+k]])
    points=[vertices[i] for i in ids]
    mesh=physical((points,faces,None,None))
    if mesh.volume < 0:
        faces=[list(reversed(f)) for f in faces]
        mesh=physical((points,faces,None,None))
    assert mesh.is_watertight and mesh.is_winding_consistent and mesh.volume>0
    assert mesh.euler_number == (0 if closed else 2)
    mappings=[{"texture":uv[0]["texture"],"uv":[uv_by_vertex[ids[i]] for i in f]} for f in faces]
    return ids,mesh,[points,faces,[materials[0]]*len(faces),mappings]


def main():
    _, _, _, (_, original, _), (_, parts, _) = load_source()
    stored = (ORIGINAL / "GENERATED-COLLISION.json").read_bytes()
    candidate = json.loads(stored)
    wells = candidate["recessedWellSelections"]
    lofts = candidate["selectedSourceVertexIndices"]
    arcs = candidate["arcSourceVertexIndices"]
    model = original | {name: [] for name in ("vertices", "faces", "faceMaterials", "faceTextures")}
    bindings = []
    for part in parts:
        index = str(part["index"])
        source = section(original, part)
        if index in arcs:
            selected = arcs[index]
            rows = sorted({i // 8 for i in selected})
            corners = sorted({i % 8 for i in selected})
            closed = len(source[1]) == 2 * len(source[0])
            recovered, _, source = reduced_arc(source, rows, corners, closed)
            require(recovered == selected, "changed original arc crown or seam")
        elif index in wells:
            source = rebuild_well(source, wells[index])
        elif index in lofts:
            source = rebuilt_section(source, lofts[index])
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
    require(output == stored, "Osprey Support candidate differs after source reconstruction")


if __name__ == "__main__":
    main()
