"""Build the SH088 collision, visual, and Studio streams from pinned originals."""

import argparse
from bisect import bisect_right
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
from banteng_art_mesh import group_labels
from prepare_eagle_cruiser_geometry import encode_visual

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / "assets/fleet/sh088"
INTEGRATION = ROOT / "integration/sh088-springbok-starliner"
STUDIO_NAME = "SPRINGBOK-STARLINER-STUDIO.json"
REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
SOURCE_HASHES = {'resource.toml': '9472ffe6e4ad44a82238624f50f4ef10b67f9364e8befc23dfa41f7a137f9606',
 'native-export.json': '806ed50db4f572d7bd50225c39e78c34555372ac3511675712d609dca019dd53',
 'contract.toml': 'a68eca7fb52dba02989f82a1c23a1201ac4ce28b2125f9a6a357fd4eb785ee82',
 'engineering.json': 'd860ec2ae784e6d8604882ea822ee48b5075db759f95322a82f69c48f9a96037',
 'finite-dry-source.json': 'f57aeed6909426bf5f97f518020f9aba93d1b4640cb667f727d59d65114662df',
 'LEGACY-FINITE-DRY.swift.txt': '5e1ca6bca6eb39331c4a821f7ef1f88d2297d45c9dbdd67e39c48c8e2829453d',
 'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
 'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
 'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
 'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
 'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d'}
OWNERSHIP_SHA = "7d670474398ce0a9d7b1401f4bb46c502cdd418e1c3208b7f97556ca8a539904"
CANDIDATE_SHA = "5e4dc38ce8569065b04e59be6333bf4e36817b530ebe7eababf6a3a78c699359"
SCALE = Fraction(64000000, 4107151)
RING_SAMPLING = ((45, 12), (23, 8), (13, 4))
ARC_SPANS = 12
APERTURE_X = 12.0
BOW_GAP_Z = 29.5
ABSENT = 255
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
        raise ValueError("Changed frozen SH088 origin")
    inputs = {}
    for name in SOURCE_HASHES:
        parent = INTEGRATION if name.startswith("LEGACY-") else ORIGINAL
        if name.endswith(".rgba"):
            parent = ROOT / "assets/fleet/sh001"
        inputs[name] = (parent / name).read_bytes()
    for name, data in inputs.items():
        expected = provenance["paths"][name]
        if pin(data) != {"bytes": expected["bytes"], "sha256": SOURCE_HASHES[name]} or expected["sha256"] != SOURCE_HASHES[name]:
            raise ValueError(f"Changed original SH088 source {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode())
    detailed, native = resource["design"], resource["nativeDesign"]
    archive = json.loads(inputs["native-export.json"])
    contract = tomllib.loads(inputs["contract.toml"].decode())
    require(resource["id"] == archive["id"] == contract["id"] == "SH088", 'wrong original SH088 identity')
    require(native == archive["nativeDesign"], 'changed approved original native mesh')
    finite = json.loads(inputs["finite-dry-source.json"])
    require(finite["coordinateUnits"] == "metres" and float(SCALE) == finite["metresPerSourceUnit"],
            "changed Springbok Starliner original calibration")
    require(archive["sourceSHA256"] == contract["source"]["geometrySHA256"], 'changed approved original mesh digest')
    require(len(native["faces"]) == 912 and contract["geometry"]["nativeFaceLimit"] == 512,
            "changed Springbok Starliner archived face count or candidate ceiling")
    catalogue, access = resource["catalogue"], contract["access"]
    require(catalogue["playerFlyable"] is catalogue["normalPurchaseEligible"] is False
            and catalogue["purchasePolicy"] == "pendingFabrication"
            and access["playerFlyable"] is True and access["purchasePolicy"] == "blueprintFabrication",
            "changed original Springbok Starliner access")
    for mesh in (detailed, native):
        require(mesh["paletteName"] == "violet-pearl", 'wrong original palette')
        require(mesh["license"] == mesh["appearanceLicense"] == "MIT", 'wrong original visual licence')
        require([item["asset"] for item in mesh["textures"]] == TEXTURES, 'wrong original texture atlas order')
    ownership_bytes = (INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes()
    candidate_bytes = (ORIGINAL / "GENERATED-COLLISION.json").read_bytes()
    require(sha256(ownership_bytes).hexdigest() == OWNERSHIP_SHA, "changed Springbok Starliner ownership")
    require(sha256(candidate_bytes).hexdigest() == CANDIDATE_SHA, "changed Springbok Starliner candidate")
    ownership = json.loads(ownership_bytes)["models"]
    candidate = json.loads(candidate_bytes)
    nominee = candidate["nativeCandidate"]
    require(len(nominee["faces"]) == 504 and len(candidate["parts"]) == 16
            and candidate["r3SHA256"] == SOURCE_HASHES["contract.toml"]
            and candidate["originalSourceSHA256"] == archive["sourceSHA256"],
            "wrong Springbok Starliner candidate identity")
    require(all(nominee[key] == native[key] for key in ("textures", "materials", "paletteName")),
            "changed Springbok Starliner candidate appearance")
    references = (detailed, native, nominee)
    parts = (ownership["design"]["parts"], ownership["nativeDesign"]["parts"], candidate["parts"])
    return provenance, inputs, resource, references, parts


def collision_inputs():
    provenance, inputs, resource, references, parts = load_source()
    labels = [group_labels(mesh, records) for mesh, records in zip(references, parts, strict=True)]
    ordinary = dict.fromkeys(label for values in labels for label in values if label[0] != "open curved loop")
    counts = Counter(label[0] for label in ordinary)
    names = [f"open arc span {i+1}" for i in range(ARC_SPANS)] + [
        label if counts[label] == 1 else f"{label} [{number}]" for label, number in ordinary
    ]
    slots = {label: i for i, label in enumerate(ordinary, ARC_SPANS)}
    require(len(names) == len(set(names)) == 37, "changed Springbok Starliner support inventory")
    meshes = []
    for mesh, records, group_names, (major, minor) in zip(references, parts, labels, RING_SAMPLING, strict=True):
        require(records[0]["name"] == "open curved loop" and records[0]["vertexCount"] == major*minor,
                "wrong Springbok Starliner closed-loop inventory")
        limits = [(i*(major-1)+ARC_SPANS//2)//ARC_SPANS for i in range(ARC_SPANS+1)]
        members = []
        for i, label in enumerate(group_names):
            if label[0] == "open curved loop":
                row = i // minor
                group = min(bisect_right(limits,row)-1,ARC_SPANS-1)
                is_seam = 0 < group and row == limits[group]
                members.append((group-1,group) if is_seam else (group,))
            else:
                members.append((slots[label],))
        require(all(set.intersection(*(set(members[i]) for i in tri)) for tri in mesh["faces"]),
                "Springbok Starliner source triangle crosses collision supports")
        meshes.append((dict(mesh, vertices=[[a*float(SCALE) for a in point] for point in mesh["vertices"]]), members))
    require([len(mesh["vertices"]) for mesh, _ in meshes] == [1460, 488, 284], "changed Springbok Starliner vertices")
    require(sum(len(row)==2 for _, membership in meshes for row in membership) == 264,
            "changed Springbok Starliner shield seams")
    return provenance, inputs, resource, references, names, meshes


def studio_mesh(mesh, memberships):
    vertices, groups, triangles, indices = [], [], [], {}
    for triangle in mesh["faces"]:
        group = min(set.intersection(*(set(memberships[i]) for i in triangle)))
        translated = []
        for vertex in triangle:
            key = (vertex, group)
            if key not in indices:
                indices[key] = len(vertices)
                vertices.append(mesh["vertices"][vertex])
                groups.append(group)
            translated.append(indices[key])
        triangles.append(translated)
    return {"vertices": vertices, "triangles": triangles, "groups": groups}


def studio_definition(provenance, inputs, resource, detailed, candidate, streams):
    detail_model, _ = detailed
    finite = json.loads(inputs["finite-dry-source.json"])["emptyDryReference"]
    require(finite["massKg"] == json.loads(inputs["engineering.json"])["dryMassKilograms"] == 89404, 'changed original dry mass')
    require(finite["id"] == "finite-intrinsic-empty-springbok-starliner", 'wrong original dry reference')
    probes = []
    for route in json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes()):
        require(route["clearanceMillimetres"] > PROBE_RADIUS * 1000, 'insufficient original probe clearance')
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(start, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": start, "end": end, "radiusMetres": PROBE_RADIUS})

    return {
        "schema": 1, "id": "SH088", "name": "Springbok Starliner", "metresPerUnit": 1, "roundingMetres": RADIUS,
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
            "sourceId": "SH088", "revision": provenance["revision"],
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
    packed = bytes(value for row in membership for value in (row if len(row)==2 else (*row, ABSENT)))
    streams = {
        "source.vertices": struct.pack("<8sIII", b"MSH088V1", *counts)
            + b"".join(struct.pack("<3d", *p) for p in vertices),
        "source-triangles.bin": b"".join(struct.pack("<3I", *f) for f in faces),
        "collision-piece-membership.bin": packed,
        "PIECES.json": encode_json({"pieces": names, "roundingMetres": RADIUS}),
        "visual.bin": b"SPBKVIZ1" + encode_visual(detail[0])[8:],
        "native-visual.bin": b"SPBKVIZ1" + encode_visual(native[0])[8:],
    }
    fans = json.loads((INTEGRATION / "SUPPORT-FAN.json").read_bytes())
    require(list(fans) == names, "wrong Springbok Starliner fan order")
    require(all(0 <= index < len(membership) and slot in membership[index]
                for slot, faces in enumerate(fans.values()) for face in faces for index in face),
            "Springbok Starliner local support fan spans pieces")
    streams[STUDIO_NAME] = encode_json(studio_definition(provenance, inputs, resource, detail, candidate, streams))
    streams["SOURCE-GEOMETRY.json"] = encode_json({
        "id": "SH088", "name": "Springbok Starliner", "sourceScaleRatio": [SCALE.numerator, SCALE.denominator], "revision": REVISION,
        "references": {"detailed": [1460, 2672, 62], "archivedNative": [488, 912, 16], "candidateNative": [284, 504, 16]},
        "nativeFaceLimit": 512, "groups": len(names), "dualMembershipVertices": 264, "roundingMetres": RADIUS,
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
            require(path.read_bytes() == data, f"changed SH088 encoding: {name}")
        else:
            replace_output(path, data)


if __name__ == "__main__":
    main()
