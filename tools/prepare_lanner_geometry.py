"""Encode both frozen Lanner artworks and the separately measured native candidate."""

import argparse
from fractions import Fraction
import hashlib
import json
import math
from pathlib import Path
import struct
import tomllib

from atomic_asset_output import checked_directory, checked_file, replace_output
from lanner_art_mesh import assemble
from prepare_eagle_cruiser_geometry import encode_visual

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / 'assets/fleet/sh048'
INTEGRATION = ROOT / 'integration/sh048-lanner'
STUDIO_NAME = 'LANNER-STUDIO.json'
SCALE = Fraction(5_000_000, 597_893)
OWNERSHIP_SHA = 'ffadcbf087c16b54f922fe3fb1ed8b568cec2034194c56e2f3233f5134dd7f29'
FROZEN = {
    'resource.toml': '614a29ae15792706a524deaee382c66318f6f05567a743ee4c2ffb28247215bf',
    'native-export.json': 'e9204ee8e06aefaf52ec0c1d203fec40ea3dd3f9ed9ab505e3186c4fc3ead8ae',
    'contract.toml': 'f8ed0cb8dcbc123a34d9c6718041de1913b6c63c4958adfbe455f1500dcacaaf',
    'engineering.json': 'cc12450d3978c0168bbdf7f0973457d2d2f1c59f3868309d642a991b37f0e64d',
    'finite-dry-source.json': '12e44a2bac7bf73a44ceda11d4ff4ad07cb0c249bbdcbbfa0b1c98e264ac5e10',
    'LEGACY-FINITE-DRY.swift.txt': 'e90d8bfe7c3c46306c618e043be387a194473196d599f8d1e9680d9f9cde6204',
    'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
    'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
    'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
    'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d',
}
CANDIDATE_SHA = "23fcbce1da185ab97a296712fc550d10f040130b8e5715936b0331170d3f4b47"


def encode(value):
    return (json.dumps(value, allow_nan=False, separators=(",", ":")) + "\n").encode()


def pin(data):
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def stream_visual(relative):
    metre_model = dict(relative, vertices=[[coordinate * float(SCALE) for coordinate in p] for p in relative["vertices"]])
    return b"LANNVIZ1" + encode_visual(metre_model)[8:]


def load_frozen():
    originals = {}
    for name, expected in FROZEN.items():
        folder = INTEGRATION if name.startswith("LEGACY-") else ORIGINAL
        if name.endswith(".rgba"):
            folder = ROOT / "assets/fleet/sh001"
        contents = (folder / name).read_bytes()
        if hashlib.sha256(contents).hexdigest() != expected:
            raise ValueError(f"Changed frozen Lanner source {name}")
        originals[name] = contents
    return originals


def build():
    originals = load_frozen()
    resource_raw = originals["resource.toml"]
    archive_raw = originals["native-export.json"]
    finite_raw = originals["finite-dry-source.json"]
    candidate_raw = (ORIGINAL / "SH048-candidate.json").read_bytes()
    if pin(candidate_raw)["sha256"] != CANDIDATE_SHA:
        raise ValueError("Changed creator-pinned Lanner candidate")
    source = tomllib.loads(resource_raw.decode())
    archive = json.loads(archive_raw)
    finite = json.loads(finite_raw)
    approved = json.loads(candidate_raw)
    if source["nativeDesign"] != archive["nativeDesign"] or archive["sourceSHA256"] != approved["originalSourceSHA256"]:
        raise ValueError("The original Lanner native archive does not match the candidate provenance")
    if float(SCALE) != finite["metresPerSourceUnit"]:
        raise ValueError("The frozen Lanner scale changed")
    detail, archived_native, candidate = source["design"], source["nativeDesign"], approved["nativeCandidate"]
    face_counts = [len(mesh["faces"]) for mesh in (detail, archived_native, candidate)]
    if face_counts != [2696, 936, 456]:
        raise ValueError("Changed Lanner archival or candidate topology")
    ownership_raw = (INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes()
    if pin(ownership_raw)["sha256"] != OWNERSHIP_SHA:
        raise ValueError("Changed frozen Lanner part ownership")
    owners = json.loads(ownership_raw)["models"]
    slots, meshes = assemble((detail, owners["design"]["parts"], 12), (candidate, approved["parts"], 4))
    if len(slots) != 75 or [len(mesh["vertices"]) for mesh in meshes] != [1608, 308]:
        raise ValueError("Unexpected grouped Lanner shape")
    specimen = json.loads((INTEGRATION / "candidate-support.json").read_bytes())
    probes = []
    for route in specimen["routes"]:
        if route.get("clearanceMillimetres", 0) <= 100:
            raise ValueError(f"Insufficient Lanner shape clearance for {route['name']}")
        centroid = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(centroid, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": centroid, "end": end, "radiusMetres": .1})
    if len(probes) != 7:
        raise ValueError("Unexpected Lanner shape routes")
    points = meshes[0]["vertices"] + meshes[1]["vertices"]
    split = len(meshes[0]["vertices"])
    triangles = meshes[0]["triangles"] + [[index + split for index in face] for face in meshes[1]["triangles"]]
    outputs = {
        "source.vertices": struct.pack("<8sII", b"MSH048V1", split, len(points) - split) + b"".join(
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
        "schema": 1, "id": "SH048", "name": "Lanner", "metresPerUnit": float(SCALE),
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
            "sourceId": "SH048", "revision": "a11fe16924d87d06021816f9037952afcbac08db",
            "frozenResource": pin(resource_raw),
            "frozenNativeArchive": {"artifact": "native-export.json", "faces": 936,
                                    **pin(archive_raw)},
            "separateNativeCandidate": {"artifact": "SH048-candidate.json", "faces": 456,
                                        "faceLimit": 512, **pin(candidate_raw)},
            "derivedHashes": {name: pin(data) for name, data in outputs.items()},
            "originalScaleRatio": [SCALE.numerator, SCALE.denominator],
            "engineeringReference": engineering["id"], "scope": "offline catalogue art inspection",
        },
    }
    outputs[STUDIO_NAME] = encode(package)
    outputs["SOURCE-GEOMETRY.json"] = encode({
        "id": "SH048", "metresPerUnit": float(SCALE), "sourceScaleRatio": [SCALE.numerator, SCALE.denominator],
        "archivedDetailFaces": 2696, "archivedNativeFaces": 936, "candidateNativeFaces": 456,
        "candidateNativeFaceLimit": 512, "supportGroups": 75,
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
                raise ValueError(f"Changed Lanner output {name}")
        else:
            replace_output(path, content)


if __name__ == "__main__":
    main()
