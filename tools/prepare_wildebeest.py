"""Build the SH104 collision, visual, and Studio streams from pinned originals."""

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
ORIGINAL = ROOT / "assets/fleet/sh104"
INTEGRATION = ROOT / "integration/sh104-wildebeest"
STUDIO_NAME = "WILDEBEEST-STUDIO.json"
REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
SOURCE_HASHES = {'resource.toml': 'add0c3181155b9d19c8be7efe212fd75edc57cf7ba50b219bcccec2c587f88c2',
 'native-export.json': '23a2b24e48f5864516839ce895f8d472cc4842dc97fc19462097ba05f2356bf1',
 'contract.toml': '771cf33fa828f8a6d26daec429a61baf705393c9ad9c5651198a9d8f144603e9',
 'engineering.json': '00c2b843f18bdb74fbfd455479787ad9d62eb6e7f41c6effb59a43c22e83fff4',
 'finite-dry-source.json': '2d457629db04ac97a506ca596f086edfb39ca555e21971ca7eb1867d72a75c77',
 'LEGACY-FINITE-DRY.swift.txt': 'ae742b2d0fcf477cad3c2256c24f4e63ec3e8a810b0cc74e6ecfa9ce21f00db7',
 'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
 'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
 'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
 'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
 'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d'}
OWNERSHIP_SHA256 = "2ab37bfe27a6a8f70af0f9420f966c7a918f7bf157197ddac2e00d4d018247ef"
SCALE = Fraction(10000, 667)
SHAPE_NAMES = ('low freight deck',
 'raised aft pressure envelope',
 'dock lateral structural member 0',
 'dock lateral structural member 1',
 'dock transverse structural member 0',
 'dock transverse structural member 1',
 'dock aperture guidance marker 0',
 'dock aperture guidance marker 1',
 'cockpit glazing envelope',
 'cockpit connected structural crossmember',
 'engine_pair/negative side/connected support',
 'engine_pair/negative side/propulsion casing',
 'engine_pair/negative side/propulsion luminous recess',
 'engine_pair/positive side/connected support',
 'engine_pair/positive side/propulsion casing',
 'engine_pair/positive side/propulsion luminous recess')
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
        raise ValueError("Changed frozen SH104 origin")
    inputs = {}
    for name in SOURCE_HASHES:
        parent = INTEGRATION if name.startswith("LEGACY-") else ORIGINAL
        if name.endswith(".rgba"):
            parent = ROOT / "assets/fleet/sh001"
        inputs[name] = (parent / name).read_bytes()
    for name, data in inputs.items():
        expected = provenance["paths"][name]
        if pin(data) != {"bytes": expected["bytes"], "sha256": SOURCE_HASHES[name]} or expected["sha256"] != SOURCE_HASHES[name]:
            raise ValueError(f"Changed original SH104 source {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode())
    detailed, native = resource["design"], resource["nativeDesign"]
    archive = json.loads(inputs["native-export.json"])
    contract = tomllib.loads(inputs["contract.toml"].decode())
    require(resource["id"] == archive["id"] == contract["id"] == "SH104", 'wrong original SH104 identity')
    require(native == archive["nativeDesign"], 'changed approved original native mesh')
    require("coordinateUnits" not in archive, "changed original relative-unit archive")
    require(archive["sourceSHA256"] == contract["source"]["geometrySHA256"], 'changed approved original mesh digest')
    require(len(native["faces"]) <= contract["geometry"]["nativeFaceLimit"] == 512, 'wrong original native face limit')
    catalogue, access = resource["catalogue"], contract["access"]
    require(catalogue["playerFlyable"] is catalogue["normalPurchaseEligible"] is False
            and catalogue["purchasePolicy"] == "pendingFabrication"
            and access["playerFlyable"] is True and access["purchasePolicy"] == "blueprintFabrication",
            "changed original Wildebeest access")
    require(len(detailed["faces"]) == 736 and len(native["faces"]) == 424,
            "changed original Wildebeest face counts")
    finite = json.loads(inputs["finite-dry-source.json"])
    require(finite["metresPerSourceUnit"] == float(SCALE)
            and json.loads(inputs["engineering.json"])["metresPerSourceUnitExact"] == str(SCALE) and finite["coordinateUnits"] == "metres",
            "changed original Wildebeest calibration")
    for mesh in (detailed, native):
        require(mesh["paletteName"] == "rust-ivory", 'wrong original palette')
        require(mesh["license"] == mesh["appearanceLicense"] == "MIT", 'wrong original visual licence')
        require([item["asset"] for item in mesh["textures"]] == TEXTURES, 'wrong original texture atlas order')
    ownership = (INTEGRATION/"ORIGINAL-OWNERSHIP.json").read_bytes()
    require(sha256(ownership).hexdigest() == OWNERSHIP_SHA256, "changed original Wildebeest creator ownership")
    ownership = json.loads(ownership)
    require(ownership["id"] == "SH104", "wrong original Wildebeest ownership identity")
    part_sets = ownership["models"]
    for model, lod in ((detailed, "design"), (native, "nativeDesign")):
        part_indices = [list(range(part["vertexStart"],part["vertexStart"]+part["vertexCount"]))
                        for part in part_sets[lod]["parts"]]
        require(part_indices == components(model), "Wildebeest original components leave creator boundaries")
    return provenance, inputs, resource, detailed, native


def calibrated(model):
    return dict(model, vertices=[[axis * float(SCALE) for axis in point] for point in model["vertices"]])


def studio_definition(provenance, inputs, resource, detailed, native, detail_membership, native_membership, streams):
    finite = json.loads(inputs["finite-dry-source.json"])["emptyDryReference"]
    require(finite["massKg"] == json.loads(inputs["engineering.json"])["dryMassKilograms"] == 98146, 'changed original dry mass')
    require(finite["id"] == "finite-intrinsic-empty-wildebeest", 'wrong original dry reference')
    probes = []
    for route in json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes()):
        require(route["clearanceMillimetres"] > PROBE_RADIUS * 1000, 'insufficient original probe clearance')
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(start, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": start, "end": end, "radiusMetres": PROBE_RADIUS})

    def mesh(source, groups):
        return {"vertices": source["vertices"], "triangles": source["faces"], "groups": list(groups)}

    return {
        "schema": 1, "id": "SH104", "name": "Wildebeest", "metresPerUnit": 1, "roundingMetres": RADIUS,
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
            "sourceId": "SH104", "revision": provenance["revision"],
            "sourceHashes": {name: sha256(data).hexdigest() for name, data in inputs.items()},
            "derivedHashes": {name: pin(data) for name, data in streams.items()},
            "engineeringReference": finite["id"], "license": inputs["LICENSE"].decode(),
            "originalMetresPerUnit": float(SCALE), "scope": "catalogue inspection only",
        },
    }


def prepare():
    provenance, inputs, resource, detailed, native = load_source()
    slots = {name: slot for slot, name in enumerate(dict.fromkeys(SHAPE_NAMES))}
    require(len(slots) == 16, 'wrong original support count')
    detail_membership = assignments(detailed, SHAPE_NAMES, slots)
    native_membership = assignments(native, SHAPE_NAMES, slots)
    detailed, native = calibrated(detailed), calibrated(native)
    detail_count, native_count = len(detail_membership), len(native_membership)
    require((detail_count, native_count) == (400, 244), "wrong original vertex count")
    membership = detail_membership + native_membership
    vertices = detailed["vertices"] + native["vertices"]
    faces = detailed["faces"] + [[i + detail_count for i in face] for face in native["faces"]]
    streams = {
        "source.vertices": struct.pack("<8sII", b"MSH104V1", detail_count, native_count)
            + b"".join(struct.pack("<3d", *p) for p in vertices),
        "source-triangles.bin": b"".join(struct.pack("<3I", *f) for f in faces),
        "source-piece-membership.bin": membership,
        "PIECES.json": encode_json({"pieces": list(slots), "roundingMetres": RADIUS}),
        "visual.bin": b"WLDVIZ01" + encode_visual(detailed)[8:],
        "native-visual.bin": b"WLDVIZ01" + encode_visual(native)[8:],
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
        "id": "SH104", "name": "Wildebeest", "sourceScaleRatio": [SCALE.numerator, SCALE.denominator], "revision": REVISION,
        "references": {"detailed": [400, 736, 16], "native": [244, 424, 16]},
        "nativeFaceLimit": 512, "groups": 16, "roundingMetres": RADIUS, "pins": SOURCE_HASHES | {"ORIGINAL-OWNERSHIP.json": OWNERSHIP_SHA256},
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
            require(path.read_bytes() == data, f"changed SH104 encoding: {name}")
        else:
            replace_output(path, data)


if __name__ == "__main__":
    main()
