"""Build the SH068 collision, visual, and Studio streams from pinned originals."""

import argparse
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
ORIGINAL = ROOT / "assets/fleet/sh068"
INTEGRATION = ROOT / "integration/sh068-badger"
STUDIO_NAME = "BADGER-STUDIO.json"
REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
SOURCE_HASHES = {
    'resource.toml': '65124fb26fab82b10bae12f292f3f1a710001ca8b0f1639f9ebf0bdb4e142be0',
    'native-export.json': 'a5c3627c7a4815bb7ad4552f0be45be2c00b399e6c9d4bbc8561c0a3308cac4f',
    'contract.toml': 'f715749177b8a3897f5af27b32f991205965847a1aa90da63ee37c5f11f74196',
    'engineering.json': '3281be54b432227dcdb00159dd93ec099f80a52a193d6c35e942683189925b36',
    'finite-dry-source.json': 'd7e99f8112ddaf3506071bc9324e04694ca82f60f4b90da8b4b3a89335df34aa',
    'LEGACY-FINITE-DRY.swift.txt': 'f5fcd58670987d923423f7641af4ac50773981b5bcdd6147242ccfb65e436a4b',
    'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
    'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
    'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
    'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d',
}
SHAPE_NAMES = (
    'central-hull',
    'cross-wing',
    'port-pylon',
    'port-forward-pod',
    'port-forward-collar',
    'port-engine',
    'port-engine',
    'port-engine',
    'starboard-pylon',
    'starboard-forward-pod',
    'starboard-forward-collar',
    'starboard-engine',
    'starboard-engine',
    'starboard-engine',
    'dorsal-canopy',
    'dorsal-cap',
    'port-service-pipe',
    'port-instrument-housing',
    'port-louvre-1',
    'port-louvre-2',
    'port-louvre-3',
    'port-louvre-4',
    'port-louvre-5',
    'starboard-service-pipe',
    'starboard-instrument-housing',
    'starboard-louvre-1',
    'starboard-louvre-2',
    'starboard-louvre-3',
    'starboard-louvre-4',
    'starboard-louvre-5',
)
NATIVE_COMPONENTS = (0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 14)
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
        raise ValueError("Changed frozen SH068 origin")
    inputs = {}
    for name in SOURCE_HASHES:
        parent = INTEGRATION if name.startswith("LEGACY-") else ORIGINAL
        if name.endswith(".rgba"):
            parent = ROOT / "assets/fleet/sh001"
        inputs[name] = (parent / name).read_bytes()
    for name, data in inputs.items():
        expected = provenance["paths"][name]
        if pin(data) != {"bytes": expected["bytes"], "sha256": SOURCE_HASHES[name]} or expected["sha256"] != SOURCE_HASHES[name]:
            raise ValueError(f"Changed original SH068 source {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode())
    detailed, native = resource["design"], resource["nativeDesign"]
    archive = json.loads(inputs["native-export.json"])
    contract = tomllib.loads(inputs["contract.toml"].decode())
    require(resource["id"] == archive["id"] == contract["id"] == "SH068", 'wrong original SH068 identity')
    require(native == archive["nativeDesign"], 'changed approved original native mesh')
    require(archive["coordinateUnits"] == "metres", 'wrong original metre units')
    require(archive["sourceSHA256"] == contract["source"]["geometrySHA256"], 'changed approved original mesh digest')
    require(len(native["faces"]) <= contract["geometry"]["nativeFaceLimit"] == 512, 'wrong original native face limit')
    for mesh in (detailed, native):
        require(mesh["paletteName"] == "slate-orange", 'wrong original palette')
        require(mesh["license"] == mesh["appearanceLicense"] == "MIT", 'wrong original visual licence')
        require([item["asset"] for item in mesh["textures"]] == TEXTURES, 'wrong original texture atlas order')
    return provenance, inputs, resource, detailed, native


def studio_definition(provenance, inputs, resource, detailed, native, detail_membership, native_membership, streams):
    finite = json.loads(inputs["finite-dry-source.json"])["emptyDryReference"]
    require(finite["massKg"] == json.loads(inputs["engineering.json"])["dryMassKilograms"] == 33622, 'changed original dry mass')
    require(finite["id"] == "finite-intrinsic-empty-badger", 'wrong original dry reference')
    probes = []
    for route in json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes()):
        require(route["clearanceMillimetres"] > PROBE_RADIUS * 1000, 'insufficient original probe clearance')
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(start, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": start, "end": end, "radiusMetres": PROBE_RADIUS})

    def mesh(source, groups):
        return {"vertices": source["vertices"], "triangles": source["faces"], "groups": list(groups)}

    return {
        "schema": 1, "id": "SH068", "name": "Badger", "metresPerUnit": 1, "roundingMetres": RADIUS,
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
            "sourceId": "SH068", "revision": provenance["revision"],
            "sourceHashes": {name: sha256(data).hexdigest() for name, data in inputs.items()},
            "derivedHashes": {name: pin(data) for name, data in streams.items()},
            "engineeringReference": finite["id"], "license": inputs["LICENSE"].decode(),
            "originalMetresPerUnit": 1, "scope": "catalogue inspection only",
        },
    }


def prepare():
    provenance, inputs, resource, detailed, native = load_source()
    slots = {name: slot for slot, name in enumerate(dict.fromkeys(SHAPE_NAMES))}
    require(len(slots) == 26, 'wrong original support count')
    detail_membership = assignments(detailed, SHAPE_NAMES, slots)
    native_membership = assignments(native, [SHAPE_NAMES[i] for i in NATIVE_COMPONENTS], slots)
    detail_count, native_count = len(detail_membership), len(native_membership)
    require((detail_count, native_count) == (3012, 160), "wrong original vertex count")
    membership = detail_membership + native_membership
    vertices = detailed["vertices"] + native["vertices"]
    faces = detailed["faces"] + [[i + detail_count for i in face] for face in native["faces"]]
    streams = {
        "source.vertices": struct.pack("<8sII", b"MSH068V1", detail_count, native_count)
            + b"".join(struct.pack("<3d", *p) for p in vertices),
        "source-triangles.bin": b"".join(struct.pack("<3I", *f) for f in faces),
        "source-piece-membership.bin": membership,
        "PIECES.json": encode_json({"pieces": list(slots), "roundingMetres": RADIUS}),
        "visual.bin": b"BADGVIZ1" + encode_visual(detailed)[8:],
        "native-visual.bin": b"BADGVIZ1" + encode_visual(native)[8:],
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
        "id": "SH068", "name": "Badger", "sourceUnits": "metres", "revision": REVISION,
        "references": {"detailed": [3012, 5912, 30], "native": [160, 284, 11]},
        "nativeFaceLimit": 512, "groups": 26, "roundingMetres": RADIUS, "pins": SOURCE_HASHES,
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
            require(path.read_bytes() == data, f"changed SH068 encoding: {name}")
        else:
            replace_output(path, data)


if __name__ == "__main__":
    main()
