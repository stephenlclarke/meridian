"""Build the SH076 collision, visual, and Studio streams from pinned originals."""

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
from prepare_eagle_cruiser_geometry import encode_visual

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / "assets/fleet/sh076"
INTEGRATION = ROOT / "integration/sh076-hog"
STUDIO_NAME = "HOG-STUDIO.json"
REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
SOURCE_HASHES = {
    'resource.toml': '2e41bcded4d3214b77b8379daa0ef6bb8f7cd1ba9f83163b0647bd0d0ee007b8',
    'native-export.json': 'f07b3ecdd974a00bca8ab3e42676c9acfa3678308ef56e5f20be0e17bd8a17bb',
    'contract.toml': '799045a816586eff2fe721601e0d5a8ab46a3acc1d7e09351d423b5b39087b0c',
    'engineering.json': 'b38b9c87b12b236cb44d5ddcf8e5b5cadecd9029c33839691e23e24c8f59f871',
    'finite-dry-source.json': '2ed2ebeb56dfbba9d5bcfed5b1dc8cd72c155fcc2f29f8dc46fea1367e99bb7c',
    'LEGACY-FINITE-DRY.swift.txt': '3afe0bd30bea77a773ddb525afe1b210c2d456f6d6aca9f2f63638f715cd9fc7',
    'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
    'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
    'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
    'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d',
    'ORIGINAL-OWNERSHIP.json': '225d89aad33691cb344dfe34975f9c1da108931fcde79bc1aa0427bbcadc380a',
}
SCALE = Fraction(55_000, 1347)
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
        raise ValueError("Changed frozen SH076 origin")
    inputs = {}
    for name in SOURCE_HASHES:
        parent = INTEGRATION if name.startswith("LEGACY-") or name == "ORIGINAL-OWNERSHIP.json" else ORIGINAL
        if name.endswith(".rgba"):
            parent = ROOT / "assets/fleet/sh001"
        inputs[name] = (parent / name).read_bytes()
    for name, data in inputs.items():
        expected = provenance["paths"][name]
        if pin(data) != {"bytes": expected["bytes"], "sha256": SOURCE_HASHES[name]} or expected["sha256"] != SOURCE_HASHES[name]:
            raise ValueError(f"Changed original SH076 source {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode())
    detailed, native = resource["design"], resource["nativeDesign"]
    archive = json.loads(inputs["native-export.json"])
    contract = tomllib.loads(inputs["contract.toml"].decode())
    require(resource["id"] == archive["id"] == contract["id"] == "SH076", 'wrong original SH076 identity')
    require(native == archive["nativeDesign"], 'changed approved original native mesh')
    finite = json.loads(inputs["finite-dry-source.json"])
    require(Fraction(finite["metresPerSourceUnitExact"]) == SCALE and finite["metresPerSourceUnit"] == float(SCALE),
            "wrong original Hog calibration")
    require([len(mesh["faces"]) for mesh in (detailed, native)] == [868, 448], "wrong original face counts")
    require(archive["sourceSHA256"] == contract["source"]["geometrySHA256"], 'changed approved original mesh digest')
    require(len(native["faces"]) <= contract["geometry"]["nativeFaceLimit"] == 512, 'wrong original native face limit')
    for mesh in (detailed, native):
        require(mesh["paletteName"] == "ochre-charcoal", 'wrong original palette')
        require(mesh["license"] == mesh["appearanceLicense"] == "MIT", 'wrong original visual licence')
        require([item["asset"] for item in mesh["textures"]] == TEXTURES, 'wrong original texture atlas order')
    return provenance, inputs, resource, detailed, native


def studio_definition(provenance, inputs, resource, detailed, native, detail_membership, native_membership, streams):
    finite = json.loads(inputs["finite-dry-source.json"])["emptyDryReference"]
    require(finite["massKg"] == json.loads(inputs["engineering.json"])["dryMassKilograms"] == 1011126, 'changed original dry mass')
    require(finite["id"] == "finite-intrinsic-empty-hog", 'wrong original dry reference')
    probes = []
    for route in json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes()):
        require(route["clearanceMillimetres"] > PROBE_RADIUS * 1000, 'insufficient original probe clearance')
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(start, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": start, "end": end, "radiusMetres": PROBE_RADIUS})

    def mesh(source, groups):
        return {"vertices": source["vertices"], "triangles": source["faces"], "groups": list(groups)}

    return {
        "schema": 1, "id": "SH076", "name": "Hog", "metresPerUnit": float(SCALE), "roundingMetres": RADIUS,
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
            "sourceId": "SH076", "revision": provenance["revision"],
            "sourceHashes": {name: sha256(data).hexdigest() for name, data in inputs.items()},
            "derivedHashes": {name: pin(data) for name, data in streams.items()},
            "engineeringReference": finite["id"], "license": inputs["LICENSE"].decode(),
            "originalScaleRatio": [SCALE.numerator, SCALE.denominator], "scope": "catalogue inspection only",
        },
    }


SUPPORTS = (
    "lower-platform", "lower-service-well", "middle-platform", "middle-service-well",
    "upper-platform", "upper-service-well", "port-stack-brace", "starboard-stack-brace",
    "cockpit-glazing", "cockpit-crossmember", "port-engine-support", "port-engine-casing",
    "port-engine-recess", "starboard-engine-support", "starboard-engine-casing", "starboard-engine-recess",
)
DETAILED_SLOTS = (0, 1, 1, 1, 1, 2, 3, 3, 3, 3, 4, 5, 5, 5, 5, *range(6, 16))


def assignments(model, parts, slots):
    output = bytearray()
    face_offset = 0
    for part, slot in zip(parts, slots, strict=True):
        start = len(output)
        end = start + part["vertexCount"]
        next_face = face_offset + part["faceCount"]
        require(part["vertexStart"] == start and part["faceStart"] == face_offset, "changed original Hog part order")
        require(all(start <= i < end for face in model["faces"][face_offset:next_face] for i in face),
                "a Hog source triangle crosses its creator part")
        output.extend(bytes([slot]) * (end - start))
        face_offset = next_face
    require(len(output) == len(model["vertices"]) and face_offset == len(model["faces"]),
            "incomplete Hog source part inventory")
    return bytes(output)


def geometry(inputs, detailed, native):
    ownership = json.loads(inputs["ORIGINAL-OWNERSHIP.json"])["models"]
    return (SUPPORTS, assignments(detailed, ownership["design"]["parts"], DETAILED_SLOTS),
            assignments(native, ownership["nativeDesign"]["parts"], range(len(SUPPORTS))))


def metre_space(mesh):
    return dict(mesh, vertices=[[coordinate * float(SCALE) for coordinate in point] for point in mesh["vertices"]])


def prepare():
    provenance, inputs, resource, detailed, native = load_source()
    names, detail_membership, native_membership = geometry(inputs, detailed, native)
    slots = {name: slot for slot, name in enumerate(names)}
    detail_count, native_count = len(detail_membership), len(native_membership)
    require((detail_count, native_count) == (484, 256), "wrong original vertex count")
    membership = detail_membership + native_membership
    vertices = metre_space(detailed)["vertices"] + metre_space(native)["vertices"]
    faces = detailed["faces"] + [[i + detail_count for i in face] for face in native["faces"]]
    streams = {
        "source.vertices": struct.pack("<8sII", b"MSH076V1", detail_count, native_count)
            + b"".join(struct.pack("<3d", *p) for p in vertices),
        "source-triangles.bin": b"".join(struct.pack("<3I", *f) for f in faces),
        "source-piece-membership.bin": membership,
        "PIECES.json": encode_json({"pieces": list(slots), "roundingMetres": RADIUS}),
        "visual.bin": b"HOGMVIZ1" + encode_visual(metre_space(detailed))[8:],
        "native-visual.bin": b"HOGMVIZ1" + encode_visual(metre_space(native))[8:],
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
        "id": "SH076", "name": "Hog", "sourceUnits": "relative", "originalScaleRatio": [SCALE.numerator, SCALE.denominator], "revision": REVISION,
        "references": {"detailed": [484, 868, 25], "native": [256, 448, 16]},
        "nativeFaceLimit": 512, "groups": 16, "roundingMetres": RADIUS, "pins": SOURCE_HASHES,
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
            require(path.read_bytes() == data, f"changed SH076 encoding: {name}")
        else:
            replace_output(path, data)


if __name__ == "__main__":
    main()
