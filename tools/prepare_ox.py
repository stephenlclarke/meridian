"""Build the SH099 collision, visual, and Studio streams from pinned originals."""

import argparse
from fractions import Fraction
from hashlib import sha256
import json
import math
from pathlib import Path
import struct
import tomllib

from atomic_asset_output import checked_directory, checked_file, replace_output
from encode_goshawk_visual import TEXTURES
from import_goshawk_envelope_source import components
from prepare_eagle_cruiser_geometry import encode_visual

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / "assets/fleet/sh099"
INTEGRATION = ROOT / "integration/sh099-ox"
STUDIO_NAME = "OX-STUDIO.json"
REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
SOURCE_HASHES = {'resource.toml': '8ca94634b7ff176b1f0b16ab4bc4fb74ac3aceb669257ec22dfe116d5dc98b4e',
 'native-export.json': 'f19fd9f13328792188cba031e7e6abf9391ab3e04c34426550a401b839242293',
 'contract.toml': '4576979bb379ce9e5560be83fbe2085973136518104c7f144ed9fe1aaaa869cc',
 'engineering.json': '279e5e2869b3a6add8f368e82e6f21d90ee2cde69fd18e0b8995b14f1e68269f',
 'finite-dry-source.json': '058d124a4e32efdab2999baa72e22eaa437e0066476ebdd39e9c769822bc4161',
 'LEGACY-FINITE-DRY.swift.txt': 'e99c829de183e8386ecf7c1224e2a560a8cb4b5264b64aeeb5040a868b52ac72',
 'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
 'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
 'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
 'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
 'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d'}
OWNERSHIP_SHA256 = "506a921e0426844a97cb13283ba716026e081f60e0901499ae499b231c43b106"
SCALE = Fraction(27500, 997)
SHAPE_NAMES = ('lower freight deck',
 'upper freight deck',
 'port deck brace',
 'starboard deck brace',
 'cockpit glazing',
 'cockpit crossmember',
 'port drive support',
 'port drive casing',
 'port luminous recess',
 'starboard drive support',
 'starboard drive casing',
 'starboard luminous recess')
RADIUS = .02
PROBE_RADIUS = .1


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def encode_json(value):
    return (json.dumps(value, allow_nan=False, separators=(",", ":")) + "\n").encode()


def assignments(mesh, labels, slots):
    output = [None] * len(mesh["vertices"])
    for group, label in zip(components(mesh), labels, strict=True):
        for vertex in group:
            output[vertex] = slots[label]
    require(all(output[a] == output[b] == output[c] for a, b, c in mesh["faces"]), 'original triangle spans supports')
    return bytes(output)


def pin(data):
    return {"bytes": len(data), "sha256": sha256(data).hexdigest()}


def load_source():
    provenance = json.loads((INTEGRATION / "FROZEN-SOURCES.json").read_bytes())
    if provenance["revision"] != REVISION or set(provenance["paths"]) != set(SOURCE_HASHES):
        raise ValueError("Changed frozen SH099 origin")
    inputs = {}
    for name in SOURCE_HASHES:
        parent = INTEGRATION if name.startswith("LEGACY-") else ORIGINAL
        if name.endswith(".rgba"):
            parent = ROOT / "assets/fleet/sh001"
        inputs[name] = (parent / name).read_bytes()
    for name, data in inputs.items():
        expected = provenance["paths"][name]
        if pin(data) != {"bytes": expected["bytes"], "sha256": SOURCE_HASHES[name]} or expected["sha256"] != SOURCE_HASHES[name]:
            raise ValueError(f"Changed original SH099 source {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode())
    detailed, native = resource["design"], resource["nativeDesign"]
    archive = json.loads(inputs["native-export.json"])
    contract = tomllib.loads(inputs["contract.toml"].decode())
    require(resource["id"] == archive["id"] == contract["id"] == "SH099", 'wrong original SH099 identity')
    require(native == archive["nativeDesign"], 'changed approved original native mesh')
    require("coordinateUnits" not in archive, "changed original relative-unit archive")
    require(archive["sourceSHA256"] == contract["source"]["geometrySHA256"], 'changed approved original mesh digest')
    require(len(native["faces"]) <= contract["geometry"]["nativeFaceLimit"] == 512, 'wrong original native face limit')
    catalogue, access = resource["catalogue"], contract["access"]
    require(catalogue["playerFlyable"] is catalogue["normalPurchaseEligible"] is False
            and catalogue["purchasePolicy"] == "pendingFabrication"
            and access["playerFlyable"] is True and access["purchasePolicy"] == "blueprintFabrication",
            "changed original Ox access")
    require(len(detailed["faces"]) == 688 and len(native["faces"]) == 376,
            "changed original Ox face counts")
    finite = json.loads(inputs["finite-dry-source.json"])
    require(finite["metresPerSourceUnit"] == float(SCALE) and finite["coordinateUnits"] == "metres",
            "changed original Ox calibration")
    for mesh in (detailed, native):
        require(mesh["paletteName"] == "sand-graphite", 'wrong original palette')
        require(mesh["license"] == mesh["appearanceLicense"] == "MIT", 'wrong original visual licence')
        require([item["asset"] for item in mesh["textures"]] == TEXTURES, 'wrong original texture atlas order')
    ownership = (INTEGRATION/"ORIGINAL-OWNERSHIP.json").read_bytes()
    require(sha256(ownership).hexdigest() == OWNERSHIP_SHA256, "changed original Ox creator ownership")
    part_sets = json.loads(ownership)["models"]
    for model, lod in ((detailed, "design"), (native, "nativeDesign")):
        part_indices = [list(range(part["vertexStart"],part["vertexStart"]+part["vertexCount"]))
                        for part in part_sets[lod]["parts"]]
        require(part_indices == components(model), "Ox original components leave creator boundaries")
    return provenance, inputs, resource, detailed, native


def calibrated(model):
    return dict(model, vertices=[[axis * float(SCALE) for axis in point] for point in model["vertices"]])


def studio_definition(provenance, inputs, resource, detailed, native, detail_membership, native_membership, streams):
    finite = json.loads(inputs["finite-dry-source.json"])["emptyDryReference"]
    require(finite["massKg"] == json.loads(inputs["engineering.json"])["dryMassKilograms"] == 524258, 'changed original dry mass')
    require(finite["id"] == "finite-intrinsic-empty-ox", 'wrong original dry reference')
    probes = []
    for route in json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes()):
        require(route["clearanceMillimetres"] > PROBE_RADIUS * 1000, 'insufficient original probe clearance')
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(start, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": start, "end": end, "radiusMetres": PROBE_RADIUS})

    def mesh(source, groups):
        return {"vertices": source["vertices"], "triangles": source["faces"], "groups": list(groups)}

    return {
        "schema": 1, "id": "SH099", "name": "Ox", "metresPerUnit": 1, "roundingMetres": RADIUS,
        "detail": mesh(detailed, detail_membership), "native": mesh(native, native_membership),
        "materials": [
            {"name": m["name"], "rgb": [m["colour"] >> shift & 255 for shift in (24, 16, 8)],
             "alpha": m["colour"] & 255, "emissive": m["emissive"]}
            for m in detailed["materials"]
        ],
        "faceMaterials": detailed["faceMaterials"],
        "faceTextures": [
            {"uv": mapping["uv"], "texture": detailed["textures"][mapping["texture"]]["asset"]
             if mapping["texture"] >= 0 else None}
            for mapping in detailed["faceTextures"]
        ],
        "engineering": {
            "massKg": finite["massKg"],
            "centerOfMass": finite["centreMetres"],
            "inertia": [finite["inertiaAboutCentreOfMassKgM2"][i:i + 3] for i in range(0, 9, 3)],
        },
        "flight": {"maxSpeed": resource["game"]["speed"]},
        "clearances": probes,
        "provenance": {
            "sourceId": "SH099", "revision": provenance["revision"],
            "sourceHashes": {name: sha256(data).hexdigest() for name, data in inputs.items()},
            "derivedHashes": {name: pin(data) for name, data in streams.items()},
            "engineeringReference": finite["id"], "license": inputs["LICENSE"].decode(),
            "originalMetresPerUnit": float(SCALE), "scope": "catalogue inspection only",
        },
    }


def prepare():
    provenance, inputs, resource, detailed, native = load_source()
    slots = {name: slot for slot, name in enumerate(dict.fromkeys(SHAPE_NAMES))}
    require(len(slots) == 12, 'wrong original support count')
    detail_membership = assignments(detailed, SHAPE_NAMES, slots)
    native_membership = assignments(native, SHAPE_NAMES, slots)
    detailed, native = calibrated(detailed), calibrated(native)
    detail_count, native_count = len(detail_membership), len(native_membership)
    require((detail_count, native_count) == (368, 212), "wrong original vertex count")
    membership = detail_membership + native_membership
    vertices = detailed["vertices"] + native["vertices"]
    faces = detailed["faces"] + [[i + detail_count for i in face] for face in native["faces"]]
    streams = {
        "source.vertices": struct.pack("<8sII", b"MSH099V1", detail_count, native_count)
            + b"".join(struct.pack("<3d", *p) for p in vertices),
        "source-triangles.bin": b"".join(struct.pack("<3I", *f) for f in faces),
        "source-piece-membership.bin": membership,
        "PIECES.json": encode_json({"pieces": list(slots), "roundingMetres": RADIUS}),
        "visual.bin": b"OXENVIZ1" + encode_visual(detailed)[8:],
        "native-visual.bin": b"OXENVIZ1" + encode_visual(native)[8:],
    }
    fans = json.loads((INTEGRATION / "SUPPORT-FAN.json").read_bytes())
    require(list(fans) == list(slots), 'wrong original fan order')
    require(all(0 <= index < len(membership) and membership[index] == slots[name]
                for name, faces in fans.items() for face in faces for index in face),
            "original support fan spans groups")
    definition = studio_definition(
        provenance, inputs, resource, detailed, native, detail_membership, native_membership, streams
    )
    streams[STUDIO_NAME] = encode_json(definition)
    streams["SOURCE-GEOMETRY.json"] = encode_json({
        "id": "SH099", "name": "Ox", "sourceScaleRatio": [SCALE.numerator, SCALE.denominator], "revision": REVISION,
        "references": {"detailed": [368, 688, 12], "native": [212, 376, 12]},
        "nativeFaceLimit": 512, "groups": 12, "roundingMetres": RADIUS, "pins": SOURCE_HASHES | {"ORIGINAL-OWNERSHIP.json": OWNERSHIP_SHA256},
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
            require(path.read_bytes() == data, f"changed SH099 encoding: {name}")
        else:
            replace_output(path, data)


if __name__ == "__main__":
    main()
