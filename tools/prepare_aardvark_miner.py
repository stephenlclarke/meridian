"""Build the SH143 collision, visual, and Studio streams from pinned originals."""

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
ORIGINAL = ROOT / "assets/fleet/sh143"
INTEGRATION = ROOT / "integration/sh143-aardvark-miner"
STUDIO_NAME = "AARDVARK-MINER-STUDIO.json"
REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
SOURCE_HASHES = {'resource.toml': 'fa77eb98d1f40c3897d265381f17bf1d8e37f08448cbeb2dc674ad4fc96bacca',
 'native-export.json': '584066fddc3a005ec72f3b6a919beaca43a63a79996614e258687b74b75e0eea',
 'contract.toml': '84c025f31ce75221465e6b3c2ea9b3d65f259c18bef1b7a82e8efa362972daea',
 'engineering.json': 'fb200eeb93a1c64b855e132b3b896b311271e65e38859fb6b09cf6aad0a5eb2f',
 'finite-dry-source.json': 'df19f55ed8254c2f7cca89de342f0bfd9a6f4582f7156ded9238ce7a0d59d1e3',
 'LEGACY-FINITE-DRY.swift.txt': 'b75bba9d057c3b79823dcd47f00421493649a0fada2c5ec0a5ad7949299ac075',
 'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
 'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
 'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
 'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
 'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d'}
OWNERSHIP_SHA = "3b7357a8207ecba600a003e08758f210b3585b9ca760ec233f2957d071181716"
CANDIDATE_SHA = "81f7f30dcb6e3a397440abc4b09c0f5d7e7916efa5979c989ed3fce527c28c59"
SCALE = Fraction(4600, 481)
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
        raise ValueError("Changed frozen SH143 origin")
    inputs = {}
    for name in SOURCE_HASHES:
        parent = INTEGRATION if name.startswith("LEGACY-") else ORIGINAL
        if name.endswith(".rgba"):
            parent = ROOT / "assets/fleet/sh001"
        inputs[name] = (parent / name).read_bytes()
    for name, data in inputs.items():
        expected = provenance["paths"][name]
        if pin(data) != {"bytes": expected["bytes"], "sha256": SOURCE_HASHES[name]} or expected["sha256"] != SOURCE_HASHES[name]:
            raise ValueError(f"Changed original SH143 source {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode())
    detailed, native = resource["design"], resource["nativeDesign"]
    archive = json.loads(inputs["native-export.json"])
    contract = tomllib.loads(inputs["contract.toml"].decode())
    require(resource["id"] == archive["id"] == contract["id"] == "SH143", 'wrong original SH143 identity')
    require(native == archive["nativeDesign"], 'changed approved original native mesh')
    finite = json.loads(inputs["finite-dry-source.json"])
    require(finite["coordinateUnits"] == "metres" and float(SCALE) == finite["metresPerSourceUnit"],
            "changed Aardvark Miner original calibration")
    require(archive["sourceSHA256"] == contract["source"]["geometrySHA256"], 'changed approved original mesh digest')
    require(len(native["faces"]) == 1112 and contract["geometry"]["nativeFaceLimit"] == 512,
            "changed Aardvark Miner archived face count or candidate ceiling")
    catalogue, access = resource["catalogue"], contract["access"]
    require(catalogue["playerFlyable"] is catalogue["normalPurchaseEligible"] is False
            and catalogue["purchasePolicy"] == "pendingFabrication"
            and access["playerFlyable"] is True and access["purchasePolicy"] == "blueprintFabrication",
            "changed original Aardvark Miner access")
    for mesh in (detailed, native):
        require(mesh["paletteName"] == "slate-orange", 'wrong original palette')
        require(mesh["license"] == mesh["appearanceLicense"] == "MIT", 'wrong original visual licence')
        require([item["asset"] for item in mesh["textures"]] == TEXTURES, 'wrong original texture atlas order')
    ownership_bytes = (INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes()
    candidate_bytes = (ORIGINAL / "GENERATED-COLLISION.json").read_bytes()
    require(sha256(ownership_bytes).hexdigest() == OWNERSHIP_SHA, "changed Aardvark Miner ownership")
    require(sha256(candidate_bytes).hexdigest() == CANDIDATE_SHA, "changed Aardvark Miner candidate")
    ownership = json.loads(ownership_bytes)
    require(ownership["id"] == "SH143", "wrong Aardvark Miner ownership identity")
    ownership = ownership["models"]
    candidate = json.loads(candidate_bytes)
    nominee = candidate["nativeCandidate"]
    require(len(nominee["faces"]) == 512 and len(candidate["parts"]) == 34 and candidate["id"] == "SH143"
            and candidate["r3SHA256"] == SOURCE_HASHES["contract.toml"]
            and candidate["originalSourceSHA256"] == archive["sourceSHA256"],
            "wrong Aardvark Miner candidate identity")
    require(all(nominee[key] == native[key] for key in ("textures", "materials", "paletteName")),
            "changed Aardvark Miner candidate appearance")
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
                "Aardvark Miner support leaves its original creator part")
        name = part["name"]
        label_counts[name] += 1
        labels.extend([(name, label_counts[name])] * part["vertexCount"])
        last_face = following_face
    require(len(labels) == len(mesh["vertices"]) and last_face == len(mesh["faces"]),
            "incomplete Aardvark Miner creator parts")
    return labels


def collision_inputs():
    provenance, inputs, resource, references, parts = load_source()
    labels = [group_labels(mesh, records) for mesh, records in zip(references, parts, strict=True)]
    keys = list(dict.fromkeys(label for values in labels for label in values))
    slots = {key: index for index, key in enumerate(keys)}
    name_counts = Counter(name for name, _ in keys)
    names = [name if name_counts[name] == 1 else f"{name} ({ordinal})" for name, ordinal in keys]
    require(len(names) == len(set(names)) == 180, "changed Aardvark Miner support inventory")
    meshes = [
        (dict(mesh, vertices=[[a*float(SCALE) for a in point] for point in mesh["vertices"]]),
         [(slots[label],) for label in groups])
        for mesh, groups in zip(references, labels, strict=True)
    ]
    require([len(mesh["vertices"]) for mesh, _ in meshes] == [6768,624,324], "changed Aardvark Miner vertices")
    return provenance, inputs, resource, references, names, meshes


def studio_mesh(mesh, memberships):
    return {"vertices":mesh["vertices"], "triangles":mesh["faces"], "groups":[row[0] for row in memberships]}


def studio_definition(provenance, inputs, resource, detailed, candidate, streams):
    detail_model, _ = detailed
    finite = json.loads(inputs["finite-dry-source.json"])["emptyDryReference"]
    require(finite["massKg"] == json.loads(inputs["engineering.json"])["dryMassKilograms"] == 50705, 'changed original dry mass')
    require(finite["id"] == "finite-intrinsic-empty-aardvark-miner", 'wrong original dry reference')
    probes = []
    for route in json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes()):
        require(route["clearanceMillimetres"] > PROBE_RADIUS * 1000, 'insufficient original probe clearance')
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(start, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": start, "end": end, "radiusMetres": PROBE_RADIUS})

    return {
        "schema": 1, "id": "SH143", "name": "Aardvark Miner", "metresPerUnit": 1, "roundingMetres": RADIUS,
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
            "sourceId": "SH143", "revision": provenance["revision"],
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
        "source.vertices": struct.pack("<8sIII", b"MSH143V1", *counts)
            + b"".join(struct.pack("<3d", *p) for p in vertices),
        "source-triangles.bin": b"".join(struct.pack("<3I", *f) for f in faces),
        "source-piece-membership.bin": packed,
        "PIECES.json": encode_json({"pieces": names, "roundingMetres": RADIUS}),
        "visual.bin": b"ADKVIZ01" + encode_visual(detail[0])[8:],
        "native-visual.bin": b"ADKVIZ01" + encode_visual(native[0])[8:],
    }
    fans = json.loads((INTEGRATION / "SUPPORT-FAN.json").read_bytes())
    require(list(fans) == names, "wrong Aardvark Miner fan order")
    require(all(0 <= index < len(membership) and slot in membership[index]
                for slot, faces in enumerate(fans.values()) for face in faces for index in face),
            "Aardvark Miner local support fan spans pieces")
    streams[STUDIO_NAME] = encode_json(studio_definition(provenance, inputs, resource, detail, candidate, streams))
    streams["SOURCE-GEOMETRY.json"] = encode_json({
        "id": "SH143", "name": "Aardvark Miner", "sourceScaleRatio": [SCALE.numerator, SCALE.denominator], "revision": REVISION,
        "references": {"detailed": [6768, 12848, 180], "archivedNative": [624, 1112, 34], "candidateNative": [324, 512, 34]},
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
            require(path.read_bytes() == data, f"changed SH143 encoding: {name}")
        else:
            replace_output(path, data)


if __name__ == "__main__":
    main()
