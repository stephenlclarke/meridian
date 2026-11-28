"""Encode both frozen Yak artworks and the separately measured native candidate."""

import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import tomllib

from atomic_asset_output import checked_directory, checked_file, replace_output
from yak_art_mesh import SCALE, assemble
from prepare_eagle_cruiser_geometry import encode_visual

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / 'assets/fleet/sh050'
INTEGRATION = ROOT / 'integration/sh050-yak'
STUDIO_NAME = 'YAK-STUDIO.json'
OWNERSHIP_SHA = 'd6f912f4ea39a32c2f99ae817eaaa2c592ad49910c6be7f91fffa87a1ee10c97'
FROZEN = {
    'resource.toml': '5b6e9eb756970bb36e2974d574e6dccea0a5d50b9a2a2c1481c5146aec8471c4',
    'native-export.json': '169976c61b33abcafc7660b37a00aa9ed14cfba0d19baba36b461a4e9d30d782',
    'contract.toml': 'bbe9e3acc93bd75e24ee42500c8b836d1a4d7eecf765c9f34bd879a6c38eaa0a',
    'engineering.json': '349091c7e330603cf3b1acefdd242406f93c49956ad709d1e50638b0e66f074a',
    'finite-dry-source.json': 'c9056cedb0c23998dcd0928603e09e9b5bf0d18bdcd4d9c3d7509bb0ea37b272',
    'LEGACY-FINITE-DRY.swift.txt': 'f701e68b9f4daa4c0df614a4459240bf4983cd991efd9f974aba77e8da23c05c',
    'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
    'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
    'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
    'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d',
}
CANDIDATE_SHA = "b2b06d321bbf3fca82b6f7b7a18e7d48b83115a4cfdca093733a3ca9191ef56f"


def encode(value):
    return (json.dumps(value, allow_nan=False, separators=(",", ":")) + "\n").encode()


def pin(data):
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def stream_visual(relative):
    metre_model = dict(relative, vertices=[[coordinate * float(SCALE) for coordinate in p] for p in relative["vertices"]])
    return b"YAKVIZ01" + encode_visual(metre_model)[8:]


def load_frozen():
    originals = {}
    for name, expected in FROZEN.items():
        folder = INTEGRATION if name.startswith("LEGACY-") else ORIGINAL
        if name.endswith(".rgba"):
            folder = ROOT / "assets/fleet/sh001"
        contents = (folder / name).read_bytes()
        if hashlib.sha256(contents).hexdigest() != expected:
            raise ValueError(f"Changed frozen Yak source {name}")
        originals[name] = contents
    return originals


def build():
    originals = load_frozen()
    resource_raw = originals["resource.toml"]
    archive_raw = originals["native-export.json"]
    finite_raw = originals["finite-dry-source.json"]
    candidate_raw = (ORIGINAL / "SH050-candidate.json").read_bytes()
    if pin(candidate_raw)["sha256"] != CANDIDATE_SHA:
        raise ValueError("Changed creator-pinned Yak candidate")
    source = tomllib.loads(resource_raw.decode())
    archive = json.loads(archive_raw)
    finite = json.loads(finite_raw)
    approved = json.loads(candidate_raw)
    if source["nativeDesign"] != archive["nativeDesign"] or archive["sourceSHA256"] != approved["originalSourceSHA256"]:
        raise ValueError("The original Yak native archive does not match the candidate provenance")
    if float(SCALE) != finite["metresPerSourceUnit"]:
        raise ValueError("The frozen Yak scale changed")
    detail, archived_native, candidate = source["design"], source["nativeDesign"], approved["nativeCandidate"]
    face_counts = [len(mesh["faces"]) for mesh in (detail, archived_native, candidate)]
    if face_counts != [3448, 928, 480]:
        raise ValueError("Changed Yak archival or candidate topology")
    ownership_raw = (INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes()
    if pin(ownership_raw)["sha256"] != OWNERSHIP_SHA:
        raise ValueError("Changed frozen Yak part ownership")
    owners = json.loads(ownership_raw)["models"]
    slots, meshes = assemble((detail, owners["design"]["parts"]), (candidate, approved["parts"]))
    if len(slots) != 162 or [len(mesh["vertices"]) for mesh in meshes] != [2048, 288]:
        raise ValueError("Unexpected grouped Yak shape")
    specimen = json.loads((INTEGRATION / "candidate-support.json").read_bytes())
    probes = []
    for route in specimen["routes"]:
        if route.get("clearanceMillimetres", 0) <= 100:
            raise ValueError(f"Insufficient Yak shape clearance for {route['name']}")
        centroid = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(centroid, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": centroid, "end": end, "radiusMetres": .1})
    if len(probes) != 7:
        raise ValueError("Unexpected Yak shape routes")
    points = meshes[0]["vertices"] + meshes[1]["vertices"]
    split = len(meshes[0]["vertices"])
    triangles = meshes[0]["triangles"] + [[index + split for index in face] for face in meshes[1]["triangles"]]
    outputs = {
        "source.vertices": struct.pack("<8sII", b"MSH050V1", split, len(points) - split) + b"".join(
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
        "schema": 1, "id": "SH050", "name": "Yak", "metresPerUnit": float(SCALE),
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
            "sourceId": "SH050", "revision": "a11fe16924d87d06021816f9037952afcbac08db",
            "frozenResource": pin(resource_raw),
            "frozenNativeArchive": {"artifact": "native-export.json", "faces": 928,
                                    **pin(archive_raw)},
            "separateNativeCandidate": {"artifact": "SH050-candidate.json", "faces": 480,
                                        "faceLimit": 512, **pin(candidate_raw)},
            "derivedHashes": {name: pin(data) for name, data in outputs.items()},
            "originalScaleRatio": [SCALE.numerator, SCALE.denominator],
            "engineeringReference": engineering["id"], "scope": "offline catalogue art inspection",
        },
    }
    outputs[STUDIO_NAME] = encode(package)
    outputs["SOURCE-GEOMETRY.json"] = encode({
        "id": "SH050", "metresPerUnit": float(SCALE), "sourceScaleRatio": [SCALE.numerator, SCALE.denominator],
        "archivedDetailFaces": 3448, "archivedNativeFaces": 928, "candidateNativeFaces": 480,
        "candidateNativeFaceLimit": 512, "supportGroups": 162,
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
                raise ValueError(f"Changed Yak output {name}")
        else:
            replace_output(path, content)


if __name__ == "__main__":
    main()
