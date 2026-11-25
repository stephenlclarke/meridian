"""Encode both frozen Caracal Scout artworks and the separately measured native candidate."""

import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import tomllib

from atomic_asset_output import checked_directory, checked_file, replace_output
from caracal_scout_art_mesh import SCALE, assemble
from prepare_eagle_cruiser_geometry import encode_visual

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / 'assets/fleet/sh053'
INTEGRATION = ROOT / 'integration/sh053-caracal-scout'
STUDIO_NAME = 'CARACAL-SCOUT-STUDIO.json'
OWNERSHIP_SHA = '0d497c935c76f3e62da05c4d20160e8149feee5cfbee784dae217d15c9808739'
FROZEN = {
    'resource.toml': 'ad45273f7deed268127d8cea7d7b79835cbe19e04aeee12dcc36882d820e96b9',
    'native-export.json': '5d22d1367b52d402b965d1177a3ff93909be5f31760f36c2a0d05c005d66bcf5',
    'contract.toml': 'd460619fc51986f13b5c54d08654755341da670df9ece4b755ff2cb35753aba0',
    'engineering.json': '65c91cb82e45783f557ddaf344df8f76f9af94102edbb7e94c4d30cc240b716c',
    'finite-dry-source.json': '94683a1c819ee71f45d19dcbbc62452f9055cbb5dc5ce939432c3bb9fb5fcab1',
    'LEGACY-FINITE-DRY.swift.txt': 'c2808690344231a9aedd89d1fa100cf11b904de8c9cda17d629379799540f514',
    'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
    'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
    'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
    'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d',
}
CANDIDATE_SHA = "31124dc9f0e1c69bf653dd315ecbc5420307dcb857bcdfc31dad7f513377f76f"


def encode(value):
    return (json.dumps(value, allow_nan=False, separators=(",", ":")) + "\n").encode()


def pin(data):
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def stream_visual(relative):
    metre_model = dict(relative, vertices=[[coordinate * float(SCALE) for coordinate in p] for p in relative["vertices"]])
    return b"CACSVIZ1" + encode_visual(metre_model)[8:]


def load_frozen():
    originals = {}
    for name, expected in FROZEN.items():
        folder = INTEGRATION if name.startswith("LEGACY-") else ORIGINAL
        if name.endswith(".rgba"):
            folder = ROOT / "assets/fleet/sh001"
        contents = (folder / name).read_bytes()
        if hashlib.sha256(contents).hexdigest() != expected:
            raise ValueError(f"Changed frozen Caracal Scout source {name}")
        originals[name] = contents
    return originals


def build():
    originals = load_frozen()
    resource_raw = originals["resource.toml"]
    archive_raw = originals["native-export.json"]
    finite_raw = originals["finite-dry-source.json"]
    candidate_raw = (ORIGINAL / "SH053-project-candidate.json").read_bytes()
    if pin(candidate_raw)["sha256"] != CANDIDATE_SHA:
        raise ValueError("Changed project-authored Caracal Scout candidate")
    source = tomllib.loads(resource_raw.decode())
    archive = json.loads(archive_raw)
    finite = json.loads(finite_raw)
    approved = json.loads(candidate_raw)
    if source["nativeDesign"] != archive["nativeDesign"] or archive["sourceSHA256"] != approved["originalSourceSHA256"]:
        raise ValueError("The original Caracal Scout native archive does not match the candidate provenance")
    if float(SCALE) != finite["metresPerSourceUnit"]:
        raise ValueError("The frozen Caracal Scout scale changed")
    detail, archived_native, candidate = source["design"], source["nativeDesign"], approved["nativeCandidate"]
    face_counts = [len(mesh["faces"]) for mesh in (detail, archived_native, candidate)]
    if face_counts != [5372, 1380, 492]:
        raise ValueError("Changed Caracal Scout archival or candidate topology")
    ownership_raw = (INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes()
    if pin(ownership_raw)["sha256"] != OWNERSHIP_SHA:
        raise ValueError("Changed frozen Caracal Scout part ownership")
    owners = json.loads(ownership_raw)["models"]
    slots, meshes = assemble((detail, owners["design"]["parts"]), (candidate, approved["parts"]))
    if len(slots) != 89 or [len(mesh["vertices"]) for mesh in meshes] != [3224, 296]:
        raise ValueError("Unexpected grouped Caracal Scout shape")
    specimen = json.loads((INTEGRATION / "candidate-support.json").read_bytes())
    probes = []
    for route in specimen["routes"]:
        if route.get("clearanceMillimetres", 0) <= 100:
            raise ValueError(f"Insufficient Caracal Scout shape clearance for {route['name']}")
        centroid = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(centroid, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": centroid, "end": end, "radiusMetres": .1})
    if len(probes) != 7:
        raise ValueError("Unexpected Caracal Scout shape routes")
    points = meshes[0]["vertices"] + meshes[1]["vertices"]
    split = len(meshes[0]["vertices"])
    triangles = meshes[0]["triangles"] + [[index + split for index in face] for face in meshes[1]["triangles"]]
    outputs = {
        "source.vertices": struct.pack("<8sII", b"MSH053V1", split, len(points) - split) + b"".join(
            struct.pack("<3d", *(value * float(SCALE) for value in point)) for point in points),
        "source-triangles.bin": b"".join(struct.pack("<3I", *triangle) for triangle in triangles),
        "source-piece-membership.bin": bytes(meshes[0]["groups"] + meshes[1]["groups"]),
        "PIECES.json": encode({"pieces": [{"name": name, "occurrence": count} for name, count in slots],
                               "roundingMetres": .02}),
        "visual.bin": stream_visual(detail),
        "archive-native-visual.bin": stream_visual(archived_native),
        "candidate-native-visual.bin": stream_visual(candidate),
    }
    engineering = finite["emptyDryReference"]
    package = {
        "schema": 1, "id": "SH053", "name": "Caracal Scout", "metresPerUnit": float(SCALE),
        "roundingMetres": .02, "detail": meshes[0], "native": meshes[1],
        "materials": [
            {"name": material["name"], "rgb": [material["colour"] >> shift & 255 for shift in (24, 16, 8)],
             "alpha": material["colour"] & 255, "emissive": material["emissive"]}
            for material in detail["materials"]
        ],
        "faceMaterials": detail["faceMaterials"],
        "faceTextures": [
            {"uv": mapping["uv"], "texture": detail["textures"][mapping["texture"]]["asset"]
             if mapping["texture"] >= 0 else None}
            for mapping in detail["faceTextures"]
        ],
        "engineering": {
            "massKg": engineering["massKg"], "centerOfMass": engineering["centreMetres"],
            "inertia": [engineering["inertiaAboutCentreOfMassKgM2"][i:i + 3] for i in (0, 3, 6)],
        },
        "flight": {"maxSpeed": source["game"]["speed"]},
        "clearances": probes,
        "provenance": {
            "sourceId": "SH053", "revision": "a11fe16924d87d06021816f9037952afcbac08db",
            "frozenResource": pin(resource_raw),
            "frozenNativeArchive": {"artifact": "native-export.json", "faces": 1380,
                                    **pin(archive_raw)},
            "separateNativeCandidate": {"artifact": "SH053-project-candidate.json", "faces": 492,
                                        "faceLimit": 512, "recipeOrigin": "project-authored", **pin(candidate_raw)},
            "derivedHashes": {name: pin(data) for name, data in outputs.items()},
            "originalScaleRatio": [SCALE.numerator, SCALE.denominator],
            "engineeringReference": engineering["id"], "scope": "offline catalogue art inspection",
        },
    }
    outputs[STUDIO_NAME] = encode(package)
    outputs["SOURCE-GEOMETRY.json"] = encode({
        "id": "SH053", "metresPerUnit": float(SCALE), "sourceScaleRatio": [SCALE.numerator, SCALE.denominator],
        "archivedDetailFaces": 5372, "archivedNativeFaces": 1380, "candidateNativeFaces": 492,
        "candidateNativeFaceLimit": 512, "supportGroups": 89,
        "outputs": {name: pin(data) for name, data in outputs.items()},
    })
    return outputs


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    checked_directory(ORIGINAL)
    checked_directory(INTEGRATION)
    outputs = build()
    paths = {name: checked_file((INTEGRATION if name == STUDIO_NAME else ORIGINAL) / name) for name in outputs}
    for name, content in outputs.items():
        path = paths[name]
        if args.check:
            if path.read_bytes() != content:
                raise ValueError(f"Changed Caracal Scout output {name}")
        else:
            replace_output(path, content)


if __name__ == "__main__":
    main()
