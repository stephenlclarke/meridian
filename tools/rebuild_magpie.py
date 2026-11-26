"""Rebuild the one-vertex Magpie canopy repair from pinned original vertices."""
import json

import numpy as np
from scipy.spatial import ConvexHull
import trimesh

from measure_banteng_geometry import solid
from prepare_magpie import ARCHIVED_COLLISION_SHA, INTEGRATION, ORIGINAL, load_source, pin, require
from verify_magpie import section

GLAZING = 19
SOURCE_VERTEX_OUT = 411
SOURCE_VERTEX_IN = 377


def rebuilt(row, indices):
    vertices, faces, materials, mappings = row
    all_points = np.asarray(vertices)
    chosen = all_points[indices]
    hull = ConvexHull(chosen)
    source = trimesh.Trimesh(all_points, faces, process=False)
    center = chosen.mean(axis=0)
    faces = []
    for a, b, c in hull.simplices.tolist():
        if np.dot(np.cross(chosen[b] - chosen[a], chosen[c] - chosen[a]), chosen[a] - center) < 0:
            b, c = c, b
        face = [a, b, c]
        start = face.index(min(face))
        faces.append(face[start:] + face[:start])
    faces.sort()
    mesh = trimesh.Trimesh(chosen, faces, process=False)
    solid(mesh)
    _, _, nearest = source.nearest.on_surface(mesh.triangles_center)
    textures = []
    for face, match in zip(faces, nearest, strict=True):
        mapping = mappings[match]
        original = source.triangles[match]
        corners = [int(np.linalg.norm(original - chosen[i], axis=1).argmin()) for i in face]
        textures.append({"texture": mapping["texture"], "uv": [mapping["uv"][i] for i in corners]})
    return [vertices[i] for i in indices], faces, [materials[i] for i in nearest], textures

def main():
    _, _, _, (_, original, _), (_, owner, _) = load_source()
    own = json.loads((INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes())
    archive = (INTEGRATION / "ARCHIVED-COLLISION.json").read_bytes()
    require(pin(archive)["sha256"] == ARCHIVED_COLLISION_SHA, "changed Magpie archived candidate")
    archived = json.loads(archive)
    baseline = archived["nativeDesign"]
    glazing = owner[GLAZING]
    source_id = glazing["vertexStart"]
    indices = set(archived["parts"][GLAZING]["originalVertexIDs"])
    require(SOURCE_VERTEX_OUT in indices and SOURCE_VERTEX_IN not in indices, "changed Magpie swap")
    chosen = sorted(i - source_id for i in (indices - {SOURCE_VERTEX_OUT}) | {SOURCE_VERTEX_IN})
    raw_glazing = section(original, glazing)

    output = baseline | {key: [] for key in ("vertices", "faces", "faceMaterials", "faceTextures")}
    bindings = []
    for pick, src in zip(archived["parts"], owner, strict=True):
        i = pick["index"]
        points, faces, materials, mappings = section(baseline, pick)
        indices = pick["originalVertexIDs"]
        if i == GLAZING:
            points, faces, materials, mappings = rebuilt(raw_glazing, chosen)
            indices = [source_id + vertex for vertex in chosen]
        points = [original["vertices"][index] for index in indices]
        start = len(output["vertices"])
        bindings.append({"index": i, "name": src["name"], "creatorMethod": src["creatorMethod"],
                         "vertexStart": start, "vertexCount": len(points),
                         "faceStart": len(output["faces"]), "faceCount": len(faces), "originalVertexIDs": indices})
        output["vertices"].extend(points)
        output["faces"].extend([[vertex + start for vertex in face] for face in faces])
        output["faceMaterials"].extend(materials)
        output["faceTextures"].extend(mappings)
    assert len(output["faces"]) == 512
    fixed = {"id": "SH107", "role": "derived native inspection candidate",
             "originalSourceSHA256": own["geometrySHA256"], "r3SHA256": own["contractSHA256"],
             "baselineCandidateSource": "ARCHIVED-COLLISION.json",
             "contactRepair": {"part": GLAZING, "sourceVertexOut": SOURCE_VERTEX_OUT,
                               "sourceVertexIn": SOURCE_VERTEX_IN},
             "nativeCandidate": output, "parts": bindings}
    candidate = (json.dumps(fixed, separators=(",", ":")) + "\n").encode()
    require(candidate == (ORIGINAL / "GENERATED-COLLISION.json").read_bytes(),
            "Magpie candidate bytes differ after rebuilding the glazing repair")


if __name__ == "__main__":
    main()
