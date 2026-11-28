"""Build the SH100 collision, visual, and Studio streams from pinned originals."""

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
ORIGINAL = ROOT / "assets/fleet/sh100"
INTEGRATION = ROOT / "integration/sh100-ox-mk-ii"
STUDIO_NAME = "OX-MK-II-STUDIO.json"
REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
SOURCE_HASHES = {'resource.toml': 'fbc34e8a189ef2e500633c7a89945b4e97596525bd94ce4ac6e61e30f34e9875',
 'native-export.json': 'eeb0dd0b4ca0ae983d19bce2241b83b1b6a7cb436fe8edd45d270fddbdd9bfcc',
 'contract.toml': '51052443fd83b5480ab4b35a91cda57dc820284ed4b128430a567a1063d48ea3',
 'engineering.json': '60a443b54ccbd7fc643c1775e62b601087dc8a0ab399c30aa5b2d4fe8e3bf797',
 'finite-dry-source.json': '129bd4871798d208b9cb9c5fa1fc778caf5085daaec66307ea6039f40c72baa7',
 'LEGACY-FINITE-DRY.swift.txt': '86fa9dab0c5d090e985d04aa829083692303b927e2470aac085deb5befee3451',
 'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
 'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
 'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
 'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
 'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d'}
OWNERSHIP_SHA256 = "88782bdde90ffaea98517a3f4c32aedeb9184c092edaea6f40e924505441f7a2"
SCALE = Fraction(2200000, 82469)
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
        raise ValueError("Changed frozen SH100 origin")
    inputs = {}
    for name in SOURCE_HASHES:
        parent = INTEGRATION if name.startswith("LEGACY-") else ORIGINAL
        if name.endswith(".rgba"):
            parent = ROOT / "assets/fleet/sh001"
        inputs[name] = (parent / name).read_bytes()
    for name, data in inputs.items():
        expected = provenance["paths"][name]
        if pin(data) != {"bytes": expected["bytes"], "sha256": SOURCE_HASHES[name]} or expected["sha256"] != SOURCE_HASHES[name]:
            raise ValueError(f"Changed original SH100 source {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode())
    detailed, native = resource["design"], resource["nativeDesign"]
    archive = json.loads(inputs["native-export.json"])
    contract = tomllib.loads(inputs["contract.toml"].decode())
    require(resource["id"] == archive["id"] == contract["id"] == "SH100", 'wrong original SH100 identity')
    require(native == archive["nativeDesign"], 'changed approved original native mesh')
    require("coordinateUnits" not in archive, "changed original relative-unit archive")
    require(archive["sourceSHA256"] == contract["source"]["geometrySHA256"], 'changed approved original mesh digest')
    require(len(native["faces"]) <= contract["geometry"]["nativeFaceLimit"] == 512, 'wrong original native face limit')
    catalogue, access = resource["catalogue"], contract["access"]
    require(catalogue["playerFlyable"] is catalogue["normalPurchaseEligible"] is False
            and catalogue["purchasePolicy"] == "pendingFabrication"
            and access["playerFlyable"] is True and access["purchasePolicy"] == "blueprintFabrication",
            "changed original Ox Mk II access")
    require(len(detailed["faces"]) == 688 and len(native["faces"]) == 376,
            "changed original Ox Mk II face counts")
    finite = json.loads(inputs["finite-dry-source.json"])
    require(finite["metresPerSourceUnit"] == float(SCALE) and finite["coordinateUnits"] == "metres",
            "changed original Ox Mk II calibration")
    for mesh in (detailed, native):
        require(mesh["paletteName"] == "sand-graphite", 'wrong original palette')
        require(mesh["license"] == mesh["appearanceLicense"] == "MIT", 'wrong original visual licence')
        require([item["asset"] for item in mesh["textures"]] == TEXTURES, 'wrong original texture atlas order')
    ownership = (INTEGRATION/"ORIGINAL-OWNERSHIP.json").read_bytes()
    require(sha256(ownership).hexdigest() == OWNERSHIP_SHA256, "changed original Ox Mk II creator ownership")
    part_sets = json.loads(ownership)["models"]
    for model, lod in ((detailed, "design"), (native, "nativeDesign")):
        part_indices = [list(range(part["vertexStart"],part["vertexStart"]+part["vertexCount"]))
                        for part in part_sets[lod]["parts"]]
        require(part_indices == components(model), "Ox Mk II original components leave creator boundaries")
    return provenance, inputs, resource, detailed, native


def calibrated(model):
    return dict(model, vertices=[[axis * float(SCALE) for axis in point] for point in model["vertices"]])


def studio_definition(provenance, inputs, resource, detailed, native, detail_membership, native_membership, streams):
    finite = json.loads(inputs["finite-dry-source.json"])["emptyDryReference"]
    require(finite["massKg"] == json.loads(inputs["engineering.json"])["dryMassKilograms"] == 515013, 'changed original dry mass')
    require(finite["id"] == "finite-intrinsic-empty-ox-mk-ii", 'wrong original dry reference')
    probes = []
    for route in json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes()):
        require(route["clearanceMillimetres"] > PROBE_RADIUS * 1000, 'insufficient original probe clearance')
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(start, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": start, "end": end, "radiusMetres": PROBE_RADIUS})

    def mesh(source, groups):
        return {"vertices": source["vertices"], "triangles": source["faces"], "groups": list(groups)}

    return {
        "schema": 1, "id": "SH100", "name": "Ox Mk II", "metresPerUnit": 1, "roundingMetres": RADIUS,
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
            "sourceId": "SH100", "revision": provenance["revision"],
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
        "source.vertices": struct.pack("<8sII", b"MSH100V1", detail_count, native_count)
            + b"".join(struct.pack("<3d", *p) for p in vertices),
        "source-triangles.bin": b"".join(struct.pack("<3I", *f) for f in faces),
        "source-piece-membership.bin": membership,
        "PIECES.json": encode_json({"pieces": list(slots), "roundingMetres": RADIUS}),
        "visual.bin": b"OMKIVIZ1" + encode_visual(detailed)[8:],
        "native-visual.bin": b"OMKIVIZ1" + encode_visual(native)[8:],
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
        "id": "SH100", "name": "Ox Mk II", "sourceScaleRatio": [SCALE.numerator, SCALE.denominator], "revision": REVISION,
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
            require(path.read_bytes() == data, f"changed SH100 encoding: {name}")
        else:
            replace_output(path, data)


if __name__ == "__main__":
    main()
