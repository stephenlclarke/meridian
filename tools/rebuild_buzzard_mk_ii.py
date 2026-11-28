"""Reconstruct the pinned Buzzard Mk II inspection mesh from original and archived recipe sections."""
import json

from prepare_buzzard_mk_ii import INTEGRATION, ORIGINAL, load_source, require
from verify_buzzard_mk_ii import section


def main():
    _, _, _, (_, original, _), (_, parts, _) = load_source()
    output = (ORIGINAL / "GENERATED-COLLISION.json").read_bytes()
    candidate = json.loads(output)
    recipe = json.loads((INTEGRATION / "ARCHIVED-RECIPE-PROVENANCE.json").read_bytes())
    fixture = json.loads((INTEGRATION / "ARCHIVED-CANDIDATE-SECTIONS.json").read_bytes())
    indices = set(recipe["sampling"]["changedOriginalPartIndices"])
    require(fixture["id"] == "SH142" and fixture["calibrated"] is False and
            fixture["archivedGeneratorSHA256"] == recipe["recipe"]["sha256"] and
            {int(key) for key in fixture["sections"]} == indices,
            "changed Buzzard Mk II archived recipe sections")
    model = original | {key: [] for key in ("vertices", "faces", "faceMaterials", "faceTextures")}
    owners = []
    for index, part in enumerate(parts):
        vertices, faces, materials, mappings = (
            fixture["sections"][str(index)] if index in indices else section(original, part))
        offset = len(model["vertices"])
        owners.append({"index": index, "name": part["name"], "vertexStart": offset,
            "vertexCount": len(vertices), "faceStart": len(model["faces"]),
            "faceCount": len(faces), "originalMaterialIndices": sorted(set(materials))})
        model["vertices"].extend(vertices)
        model["faces"].extend([[corner + offset for corner in face] for face in faces])
        model["faceMaterials"].extend(materials)
        model["faceTextures"].extend(mappings)
    replay = candidate | {"nativeCandidate": model, "parts": owners}
    require((json.dumps(replay, separators=(",", ":")) + "\n").encode() == output,
            "Buzzard Mk II mesh differs from reconstructed archived and fixed sections")


if __name__ == "__main__":
    main()
