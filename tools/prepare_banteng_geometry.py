"""Encode both frozen Banteng artworks and the separately measured native candidate."""

import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import tomllib

from atomic_asset_output import checked_directory, checked_file, replace_output
from banteng_art_mesh import SCALE, assemble
from prepare_eagle_cruiser_geometry import encode_visual

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / 'assets/fleet/sh052'
INTEGRATION = ROOT / 'integration/sh052-banteng'
STUDIO_NAME = 'BANTENG-STUDIO.json'
OWNERSHIP_SHA = '2345b4e5f585a564e6dab623a10226208b3797135705229f9757a0646e27ed7a'
FROZEN = {
    'resource.toml': 'a82799c42bfc9a9f8f52f9c5bae3305946a2bb92a8b4f710bd36cac41c172187',
    'native-export.json': '40711805c6a6596bd3f180c5db32aeeddcb93af8c137c302e45f617e0073a34b',
    'contract.toml': 'd68c0e8af3ba7c75ba30a23441b2adb2b380ccaf755c974c46c822a657c70cce',
    'engineering.json': '8afa4ff048b125121baecbac01edde17bff3cab5d2cdc4c6fb898a8d8ee605d5',
    'finite-dry-source.json': '2338cd9fb3246199757cbf12ec349bcc6b3923caf38475dc039e0dbad82f6cc4',
    'LEGACY-FINITE-DRY.swift.txt': '478d2eb90796a584dfdb3ef256fc57c6f7e0eaca517f5442000dcb384ed580b1',
    'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
    'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
    'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
    'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d',
}
CANDIDATE_SHA = "d850f1efe806473afc7412bd44ccf1bd9532f770d4671e786c59133ecff88ed5"


def encode(value):
    return (json.dumps(value, allow_nan=False, separators=(",", ":")) + "\n").encode()


def pin(data):
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def stream_visual(relative):
    metre_model = dict(relative, vertices=[[coordinate * float(SCALE) for coordinate in p] for p in relative["vertices"]])
    return b"BANTVIZ1" + encode_visual(metre_model)[8:]


def load_frozen():
    originals = {}
    for name, expected in FROZEN.items():
        folder = INTEGRATION if name.startswith("LEGACY-") else ORIGINAL
        if name.endswith(".rgba"):
            folder = ROOT / "assets/fleet/sh001"
        contents = (folder / name).read_bytes()
        if hashlib.sha256(contents).hexdigest() != expected:
            raise ValueError(f"Changed frozen Banteng source {name}")
        originals[name] = contents
    return originals


def build():
    originals = load_frozen()
    resource_raw = originals["resource.toml"]
    archive_raw = originals["native-export.json"]
    finite_raw = originals["finite-dry-source.json"]
    candidate_raw = (ORIGINAL / "SH052-project-candidate.json").read_bytes()
    if pin(candidate_raw)["sha256"] != CANDIDATE_SHA:
        raise ValueError("Changed project-authored Banteng candidate")
    source = tomllib.loads(resource_raw.decode())
    archive = json.loads(archive_raw)
    finite = json.loads(finite_raw)
    approved = json.loads(candidate_raw)
    if source["nativeDesign"] != archive["nativeDesign"] or archive["sourceSHA256"] != approved["originalSourceSHA256"]:
        raise ValueError("The original Banteng native archive does not match the candidate provenance")
    if float(SCALE) != finite["metresPerSourceUnit"]:
        raise ValueError("The frozen Banteng scale changed")
    detail, archived_native, candidate = source["design"], source["nativeDesign"], approved["nativeCandidate"]
    face_counts = [len(mesh["faces"]) for mesh in (detail, archived_native, candidate)]
    if face_counts != [5324, 1332, 484]:
        raise ValueError("Changed Banteng archival or candidate topology")
    ownership_raw = (INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes()
    if pin(ownership_raw)["sha256"] != OWNERSHIP_SHA:
        raise ValueError("Changed frozen Banteng part ownership")
    owners = json.loads(ownership_raw)["models"]
    slots, meshes = assemble((detail, owners["design"]["parts"]), (candidate, approved["parts"]))
    if len(slots) != 85 or [len(mesh["vertices"]) for mesh in meshes] != [3192, 288]:
        raise ValueError("Unexpected grouped Banteng shape")
    specimen = json.loads((INTEGRATION / "candidate-support.json").read_bytes())
    probes = []
    for route in specimen["routes"]:
        if route.get("clearanceMillimetres", 0) <= 100:
            raise ValueError(f"Insufficient Banteng shape clearance for {route['name']}")
        centroid = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(centroid, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": centroid, "end": end, "radiusMetres": .1})
    if len(probes) != 7:
        raise ValueError("Unexpected Banteng shape routes")
    points = meshes[0]["vertices"] + meshes[1]["vertices"]
    split = len(meshes[0]["vertices"])
    triangles = meshes[0]["triangles"] + [[index + split for index in face] for face in meshes[1]["triangles"]]
    outputs = {
        "source.vertices": struct.pack("<8sII", b"MSH052V1", split, len(points) - split) + b"".join(
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
        "schema": 1, "id": "SH052", "name": "Banteng", "metresPerUnit": float(SCALE),
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
            "sourceId": "SH052", "revision": "a11fe16924d87d06021816f9037952afcbac08db",
            "frozenResource": pin(resource_raw),
            "frozenNativeArchive": {"artifact": "native-export.json", "faces": 1332,
                                    **pin(archive_raw)},
            "separateNativeCandidate": {"artifact": "SH052-project-candidate.json", "faces": 484,
                                        "faceLimit": 512, "recipeOrigin": "project-authored", **pin(candidate_raw)},
            "derivedHashes": {name: pin(data) for name, data in outputs.items()},
            "originalScaleRatio": [SCALE.numerator, SCALE.denominator],
            "engineeringReference": engineering["id"], "scope": "offline catalogue art inspection",
        },
    }
    outputs[STUDIO_NAME] = encode(package)
    outputs["SOURCE-GEOMETRY.json"] = encode({
        "id": "SH052", "metresPerUnit": float(SCALE), "sourceScaleRatio": [SCALE.numerator, SCALE.denominator],
        "archivedDetailFaces": 5324, "archivedNativeFaces": 1332, "candidateNativeFaces": 484,
        "candidateNativeFaceLimit": 512, "supportGroups": 85,
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
                raise ValueError(f"Changed Banteng output {name}")
        else:
            replace_output(path, content)


if __name__ == "__main__":
    main()
