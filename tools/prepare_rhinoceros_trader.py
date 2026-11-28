"""Build the SH067 collision, visual, and Studio streams from pinned originals."""

import argparse
from fractions import Fraction
from hashlib import sha256
import json
import math
from pathlib import Path
import struct
import tomllib

from banteng_art_mesh import assemble
from atomic_asset_output import checked_directory, checked_file, replace_output
from encode_goshawk_visual import TEXTURES
from prepare_eagle_cruiser_geometry import encode_visual

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / "assets/fleet/sh067"
INTEGRATION = ROOT / "integration/sh067-rhinoceros-trader"
STUDIO_NAME = "RHINOCEROS-TRADER-STUDIO.json"
REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
SOURCE_HASHES = {
    'resource.toml': '18b486ffe7d4adceefa606b31bcbc8a0bd4b5c79f40e6faa6c481a8b53052c60',
    'native-export.json': 'bf1744adc072c0c27f0afeb9411bf9447aab3005e7720c07d3da6b33caeee48e',
    'contract.toml': '18e33608d5ff332934812bfcf40b6f99cafdcc900727a0a5f2090ab183dd15af',
    'engineering.json': '2b383f5ee2356174eae999eadf563b8fa0b7c2e3c944fa3e8707cb511414aa51',
    'finite-dry-source.json': 'eaca4c578b3b73f36d6fe1b84b8534b8a0539f11ed3f9cc3045c8bc5a40ad4af',
    'LEGACY-FINITE-DRY.swift.txt': 'ab6736f7ab01981ddaa33ee474b985339c16a2f53ae7f356300d07a096d8e807',
    'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
    'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
    'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
    'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d',
    'ORIGINAL-OWNERSHIP.json': 'cbee1a1c760f4ee83d1504986db8bf49060428be987c0996086db603e1379896',
}
SCALE = Fraction(40_000, 3263)
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
        raise ValueError("Changed frozen SH067 origin")
    inputs = {}
    for name in SOURCE_HASHES:
        parent = INTEGRATION if name.startswith("LEGACY-") or name == "ORIGINAL-OWNERSHIP.json" else ORIGINAL
        if name.endswith(".rgba"):
            parent = ROOT / "assets/fleet/sh001"
        inputs[name] = (parent / name).read_bytes()
    for name, data in inputs.items():
        expected = provenance["paths"][name]
        if pin(data) != {"bytes": expected["bytes"], "sha256": SOURCE_HASHES[name]} or expected["sha256"] != SOURCE_HASHES[name]:
            raise ValueError(f"Changed original SH067 source {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode())
    detailed, native = resource["design"], resource["nativeDesign"]
    archive = json.loads(inputs["native-export.json"])
    contract = tomllib.loads(inputs["contract.toml"].decode())
    require(resource["id"] == archive["id"] == contract["id"] == "SH067", 'wrong original SH067 identity')
    require(native == archive["nativeDesign"], 'changed approved original native mesh')
    finite = json.loads(inputs["finite-dry-source.json"])
    require(Fraction(finite["metresPerSourceUnitExact"]) == SCALE and finite["metresPerSourceUnit"] == float(SCALE),
            "wrong original Rhinoceros Trader calibration")
    require([len(mesh["faces"]) for mesh in (detailed, native)] == [700, 388], "wrong original face counts")
    require(archive["sourceSHA256"] == contract["source"]["geometrySHA256"], 'changed approved original mesh digest')
    require(len(native["faces"]) <= contract["geometry"]["nativeFaceLimit"] == 512, 'wrong original native face limit')
    for mesh in (detailed, native):
        require(mesh["paletteName"] == "sand-graphite", 'wrong original palette')
        require(mesh["license"] == mesh["appearanceLicense"] == "MIT", 'wrong original visual licence')
        require([item["asset"] for item in mesh["textures"]] == TEXTURES, 'wrong original texture atlas order')
    return provenance, inputs, resource, detailed, native


def studio_definition(provenance, inputs, resource, detailed, native, detail_membership, native_membership, streams):
    finite = json.loads(inputs["finite-dry-source.json"])["emptyDryReference"]
    require(finite["massKg"] == json.loads(inputs["engineering.json"])["dryMassKilograms"] == 61087, 'changed original dry mass')
    require(finite["id"] == "finite-intrinsic-empty-rhinoceros-trader", 'wrong original dry reference')
    probes = []
    for route in json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes()):
        require(route["clearanceMillimetres"] > PROBE_RADIUS * 1000, 'insufficient original probe clearance')
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(start, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": start, "end": end, "radiusMetres": PROBE_RADIUS})

    def mesh(source, groups):
        return {"vertices": source["vertices"], "triangles": source["faces"], "groups": list(groups)}

    return {
        "schema": 1, "id": "SH067", "name": "Rhinoceros Trader", "metresPerUnit": float(SCALE), "roundingMetres": RADIUS,
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
            "sourceId": "SH067", "revision": provenance["revision"],
            "sourceHashes": {name: sha256(data).hexdigest() for name, data in inputs.items()},
            "derivedHashes": {name: pin(data) for name, data in streams.items()},
            "engineeringReference": finite["id"], "license": inputs["LICENSE"].decode(),
            "originalScaleRatio": [SCALE.numerator, SCALE.denominator], "scope": "catalogue inspection only",
        },
    }


def geometry(inputs, detailed, native):
    ownership = json.loads(inputs["ORIGINAL-OWNERSHIP.json"])["models"]
    slots, (first, second) = assemble((detailed, ownership["design"]["parts"]),
                                     (native, ownership["nativeDesign"]["parts"]))
    require(len(slots) == 13, "wrong original Rhinoceros Trader support count")
    names = [f"{name}[{ordinal}]" for name, ordinal in slots]
    return names, bytes(first["groups"]), bytes(second["groups"])


def metre_space(mesh):
    return dict(mesh, vertices=[[coordinate * float(SCALE) for coordinate in point] for point in mesh["vertices"]])


def prepare():
    provenance, inputs, resource, detailed, native = load_source()
    names, detail_membership, native_membership = geometry(inputs, detailed, native)
    slots = {name: slot for slot, name in enumerate(names)}
    detail_count, native_count = len(detail_membership), len(native_membership)
    require((detail_count, native_count) == (376, 220), "wrong original vertex count")
    membership = detail_membership + native_membership
    vertices = metre_space(detailed)["vertices"] + metre_space(native)["vertices"]
    faces = detailed["faces"] + [[i + detail_count for i in face] for face in native["faces"]]
    streams = {
        "source.vertices": struct.pack("<8sII", b"MSH067V1", detail_count, native_count)
            + b"".join(struct.pack("<3d", *p) for p in vertices),
        "source-triangles.bin": b"".join(struct.pack("<3I", *f) for f in faces),
        "source-piece-membership.bin": membership,
        "PIECES.json": encode_json({"pieces": list(slots), "roundingMetres": RADIUS}),
        "visual.bin": b"RHINVIZ1" + encode_visual(metre_space(detailed))[8:],
        "native-visual.bin": b"RHINVIZ1" + encode_visual(metre_space(native))[8:],
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
        "id": "SH067", "name": "Rhinoceros Trader", "sourceUnits": "relative", "originalScaleRatio": [SCALE.numerator, SCALE.denominator], "revision": REVISION,
        "references": {"detailed": [376, 700, 13], "native": [220, 388, 13]},
        "nativeFaceLimit": 512, "groups": 13, "roundingMetres": RADIUS, "pins": SOURCE_HASHES,
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
            require(path.read_bytes() == data, f"changed SH067 encoding: {name}")
        else:
            replace_output(path, data)


if __name__ == "__main__":
    main()
