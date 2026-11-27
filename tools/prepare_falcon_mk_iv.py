"""Build the SH139 collision, visual, and Studio streams from pinned originals."""

import argparse
from collections import Counter
from fractions import Fraction
from hashlib import sha256
import json
import math
from pathlib import Path
import struct
import tomllib

from atomic_asset_output import checked_directory, checked_file, replace_output
from encode_goshawk_visual import TEXTURES
from prepare_eagle_cruiser_geometry import encode_visual

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / "assets/fleet/sh139"
INTEGRATION = ROOT / "integration/sh139-falcon-mk-iv"
STUDIO_NAME = "FALCON-MK-IV-STUDIO.json"
REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
SOURCE_HASHES = {'resource.toml': '5aef4995471fb4cf5741a563894a96323db3390a8718e6bfa892516307ac5da7',
 'native-export.json': 'a57bfca16a9547bdcb8df24ae5fca763d06f42a2e3b03175e07f26d27ccae9aa',
 'contract.toml': 'd033cfb048e5fb53bb9688bc4841b7bb9476431401c6a8e1f1c82c5d3945e19c',
 'engineering.json': '6b0dde0431ccdd94c5e27ac53dc0e3bf7b23abb5a1cd65ec50bb3cbf1a39410b',
 'finite-dry-source.json': '51430ec42418190fc0c1178c94fa2cc2e969dc75b29c77988d176b2fa4043f6b',
 'LEGACY-FINITE-DRY.swift.txt': '7b9045a5edeb1ad7d8604913d014dcd914b2cf613f59159c28a1880396cc8233',
 'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
 'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
 'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
 'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
 'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d'}
OWNERSHIP_SHA = "eee93a9062468b5997d249585faa69ebcf4b6930b8b8278e3fae5f46e479f39e"
CANDIDATE_SHA = "3476e5bf11e0806c16b0a32e2d70a788cb2d6e23a7598eaac69ce8ff0fbd2953"
SCALE = Fraction(30000000, 3109567)
RADIUS = .02
PROBE_RADIUS = .1


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def encode_json(value):
    return (json.dumps(value, allow_nan=False, separators=(",", ":")) + "\n").encode()


def pin(data):
    return {"bytes": len(data), "sha256": sha256(data).hexdigest()}


def load_source():
    provenance = json.loads((INTEGRATION / "FROZEN-SOURCES.json").read_bytes())
    if provenance["revision"] != REVISION or set(provenance["paths"]) != set(SOURCE_HASHES):
        raise ValueError("Changed frozen SH139 origin")
    inputs = {}
    for name in SOURCE_HASHES:
        parent = INTEGRATION if name.startswith("LEGACY-") else ORIGINAL
        if name.endswith(".rgba"):
            parent = ROOT / "assets/fleet/sh001"
        inputs[name] = (parent / name).read_bytes()
    for name, data in inputs.items():
        expected = provenance["paths"][name]
        if pin(data) != {"bytes": expected["bytes"], "sha256": SOURCE_HASHES[name]} or expected["sha256"] != SOURCE_HASHES[name]:
            raise ValueError(f"Changed original SH139 source {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode())
    detailed, native = resource["design"], resource["nativeDesign"]
    archive = json.loads(inputs["native-export.json"])
    contract = tomllib.loads(inputs["contract.toml"].decode())
    require(resource["id"] == archive["id"] == contract["id"] == "SH139", 'wrong original SH139 identity')
    require(native == archive["nativeDesign"], 'changed approved original native mesh')
    finite = json.loads(inputs["finite-dry-source.json"])
    require(finite["coordinateUnits"] == "metres" and float(SCALE) == finite["metresPerSourceUnit"],
            "changed Falcon Mk IV original calibration")
    require(archive["sourceSHA256"] == contract["source"]["geometrySHA256"], 'changed approved original mesh digest')
    require(len(native["faces"]) == 636 and contract["geometry"]["nativeFaceLimit"] == 512,
            "changed Falcon Mk IV archived face count or candidate ceiling")
    catalogue, access = resource["catalogue"], contract["access"]
    require(catalogue["playerFlyable"] is catalogue["normalPurchaseEligible"] is False
            and catalogue["purchasePolicy"] == "pendingFabrication"
            and access["playerFlyable"] is True and access["purchasePolicy"] == "blueprintFabrication",
            "changed original Falcon Mk IV access")
    for mesh in (detailed, native):
        require(mesh["paletteName"] == "ice-blue", 'wrong original palette')
        require(mesh["license"] == mesh["appearanceLicense"] == "MIT", 'wrong original visual licence')
        require([item["asset"] for item in mesh["textures"]] == TEXTURES, 'wrong original texture atlas order')
    ownership_bytes = (INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes()
    candidate_bytes = (ORIGINAL / "GENERATED-COLLISION.json").read_bytes()
    require(sha256(ownership_bytes).hexdigest() == OWNERSHIP_SHA, "changed Falcon Mk IV ownership")
    require(sha256(candidate_bytes).hexdigest() == CANDIDATE_SHA, "changed Falcon Mk IV candidate")
    ownership = json.loads(ownership_bytes)
    require(ownership["id"] == "SH139", "wrong Falcon Mk IV ownership identity")
    ownership = ownership["models"]
    candidate = json.loads(candidate_bytes)
    nominee = candidate["nativeCandidate"]
    require(len(nominee["faces"]) == 512 and len(candidate["parts"]) == 21 and candidate["id"] == "SH139"
            and candidate["r3SHA256"] == SOURCE_HASHES["contract.toml"]
            and candidate["originalSourceSHA256"] == archive["sourceSHA256"],
            "wrong Falcon Mk IV candidate identity")
    require(all(nominee[key] == native[key] for key in ("textures", "materials", "paletteName")),
            "changed Falcon Mk IV candidate appearance")
    references = (detailed, native, nominee)
    parts = (ownership["design"]["parts"], ownership["nativeDesign"]["parts"], candidate["parts"])
    return provenance, inputs, resource, references, parts


def group_labels(mesh, parts):
    label_counts = Counter()
    labels = []
    last_face = 0
    for part in parts:
        start = len(labels)
        end = start + part["vertexCount"]
        following_face = last_face + part["faceCount"]
        require((part["vertexStart"], part["faceStart"]) == (start, last_face)
                and following_face <= len(mesh["faces"])
                and all(start <= i < end for face in mesh["faces"][last_face:following_face] for i in face),
                "Falcon Mk IV support leaves its original creator part")
        name = part["name"]
        label_counts[name] += 1
        labels.extend([(name, label_counts[name])] * part["vertexCount"])
        last_face = following_face
    require(len(labels) == len(mesh["vertices"]) and last_face == len(mesh["faces"]),
            "incomplete Falcon Mk IV creator parts")
    return labels


def collision_inputs():
    provenance, inputs, resource, references, parts = load_source()
    labels = [group_labels(mesh, records) for mesh, records in zip(references, parts, strict=True)]
    keys = list(dict.fromkeys(label for values in labels for label in values))
    slots = {key: index for index, key in enumerate(keys)}
    name_counts = Counter(name for name, _ in keys)
    names = [name if name_counts[name] == 1 else f"{name} ({ordinal})" for name, ordinal in keys]
    require(len(names) == len(set(names)) == 73, "changed Falcon Mk IV support inventory")
    meshes = [
        (dict(mesh, vertices=[[a*float(SCALE) for a in point] for point in mesh["vertices"]]),
         [(slots[label],) for label in groups])
        for mesh, groups in zip(references, labels, strict=True)
    ]
    require([len(mesh["vertices"]) for mesh, _ in meshes] == [1024,360,298], "changed Falcon Mk IV vertices")
    return provenance, inputs, resource, references, names, meshes


def studio_mesh(mesh, memberships):
    return {"vertices":mesh["vertices"], "triangles":mesh["faces"], "groups":[row[0] for row in memberships]}


def studio_definition(provenance, inputs, resource, detailed, candidate, streams):
    detail_model, _ = detailed
    finite = json.loads(inputs["finite-dry-source.json"])["emptyDryReference"]
    require(finite["massKg"] == json.loads(inputs["engineering.json"])["dryMassKilograms"] == 37662, 'changed original dry mass')
    require(finite["id"] == "finite-intrinsic-empty-falcon-mk-iv", 'wrong original dry reference')
    probes = []
    for route in json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes()):
        require(route["clearanceMillimetres"] > PROBE_RADIUS * 1000, 'insufficient original probe clearance')
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(start, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": start, "end": end, "radiusMetres": PROBE_RADIUS})

    return {
        "schema": 1, "id": "SH139", "name": "Falcon Mk IV", "metresPerUnit": 1, "roundingMetres": RADIUS,
        "detail": studio_mesh(*detailed), "native": studio_mesh(*candidate),
        "materials": [
            {"name": m["name"], "rgb": [m["colour"] >> shift & 255 for shift in (24, 16, 8)],
             "alpha": m["colour"] & 255, "emissive": m["emissive"]}
            for m in detail_model["materials"]
        ],
        "faceMaterials": detail_model["faceMaterials"],
        "faceTextures": [
            {"uv": mapping["uv"], "texture": detail_model["textures"][mapping["texture"]]["asset"]
             if mapping["texture"] >= 0 else None}
            for mapping in detail_model["faceTextures"]
        ],
        "engineering": {
            "massKg": finite["massKg"],
            "centerOfMass": finite["centreMetres"],
            "inertia": [finite["inertiaAboutCentreOfMassKgM2"][i:i + 3] for i in range(0, 9, 3)],
        },
        "flight": {"maxSpeed": resource["game"]["speed"]},
        "clearances": probes,
        "provenance": {
            "sourceId": "SH139", "revision": provenance["revision"],
            "sourceHashes": {name: sha256(data).hexdigest() for name, data in inputs.items()},
            "derivedHashes": {name: pin(data) for name, data in streams.items()},
            "engineeringReference": finite["id"], "license": inputs["LICENSE"].decode(),
            "originalMetresPerUnit": float(SCALE), "scope": "catalogue inspection only",
        },
    }


def prepare():
    provenance, inputs, resource, _, names, meshes = collision_inputs()
    detail, native, candidate = meshes
    vertices = [point for model, _ in meshes for point in model["vertices"]]
    membership = [row for _, groups in meshes for row in groups]
    counts = [len(model["vertices"]) for model,_ in meshes]
    faces = []
    offset = 0
    for model, _ in meshes:
        faces.extend([index+offset for index in face] for face in model["faces"])
        offset += len(model["vertices"])
    packed = bytes(row[0] for row in membership)
    streams = {
        "source.vertices": struct.pack("<8sIII", b"MSH139V1", *counts)
            + b"".join(struct.pack("<3d", *p) for p in vertices),
        "source-triangles.bin": b"".join(struct.pack("<3I", *f) for f in faces),
        "source-piece-membership.bin": packed,
        "PIECES.json": encode_json({"pieces": names, "roundingMetres": RADIUS}),
        "visual.bin": b"FLC4VIZ1" + encode_visual(detail[0])[8:],
        "native-visual.bin": b"FLC4VIZ1" + encode_visual(native[0])[8:],
    }
    fans = json.loads((INTEGRATION / "SUPPORT-FAN.json").read_bytes())
    require(list(fans) == names, "wrong Falcon Mk IV fan order")
    require(all(0 <= index < len(membership) and slot in membership[index]
                for slot, faces in enumerate(fans.values()) for face in faces for index in face),
            "Falcon Mk IV local support fan spans pieces")
    streams[STUDIO_NAME] = encode_json(studio_definition(provenance, inputs, resource, detail, candidate, streams))
    streams["SOURCE-GEOMETRY.json"] = encode_json({
        "id": "SH139", "name": "Falcon Mk IV", "sourceScaleRatio": [SCALE.numerator, SCALE.denominator], "revision": REVISION,
        "references": {"detailed": [1024, 1756, 73], "archivedNative": [360, 636, 21], "candidateNative": [298, 512, 21]},
        "nativeFaceLimit": 512, "groups": len(names), "roundingMetres": RADIUS,
        "pins": SOURCE_HASHES | {"GENERATED-COLLISION.json": CANDIDATE_SHA, "ORIGINAL-OWNERSHIP.json": OWNERSHIP_SHA},
        "outputs": {name: pin(data) for name, data in streams.items()},
    })
    return streams


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    checked_directory(ORIGINAL)
    checked_directory(INTEGRATION)
    streams = prepare()
    paths = {name: checked_file((INTEGRATION if name == STUDIO_NAME else ORIGINAL) / name) for name in streams}
    for name, data in streams.items():
        path = paths[name]
        if args.check:
            require(path.read_bytes() == data, f"changed SH139 encoding: {name}")
        else:
            replace_output(path, data)


if __name__ == "__main__":
    main()
