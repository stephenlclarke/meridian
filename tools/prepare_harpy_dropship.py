"""Build the SH046 collision, visual, and Studio streams from pinned originals."""

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
ORIGINAL = ROOT / "assets/fleet/sh046"
INTEGRATION = ROOT / "integration/sh046-harpy-dropship"
STUDIO_NAME = "HARPY-DROPSHIP-STUDIO.json"
REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
SOURCE_HASHES = {
    'resource.toml': '131631fcceebcfa276f94c762944cfe3f1bf1902360c0897258a8484ccc258e4',
    'native-export.json': '866a109262e8be09261fadffbd518e1e2f52635cce141f3af4823226a778a007',
    'contract.toml': '18403611eadeb9ccad245863016046fb2df9ef19041f428d52492fdda9574951',
    'engineering.json': 'ceedb96a59a0c93e86658dff9f3e5e92720eb9fc256c6f481f820cf917397ffb',
    'finite-dry-source.json': '8bb454f6ddfb7f55b62c276a80d3b6d195441baa166ea551003ea7a276351618',
    'LEGACY-FINITE-DRY.swift.txt': '1cc74f0d307b25e3079f75c22a95bb8e0265272ce6c571f9429dfad5047316dd',
    'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
    'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
    'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
    'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d',
}
SHAPE_NAMES = (
    "central-keel", "port-keel", "port-shoulder", "port-drive", "port-drive", "port-drive",
    "starboard-keel", "starboard-shoulder", "starboard-drive", "starboard-drive", "starboard-drive",
    "canopy", "canopy-rib", "belly-module", "forward-ramp", "port-skid", "starboard-skid",
    "port-service-pipe", "port-instrument-housing", *(f"port-louvre-{i}" for i in range(1, 6)),
    "starboard-service-pipe", "starboard-instrument-housing", *(f"starboard-louvre-{i}" for i in range(1, 6)),
)
NATIVE_COMPONENTS = (0, 1, 2, 3, 6, 7, 8, 11, 13, 14, 15, 16)
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
        raise ValueError("Changed frozen SH046 origin")
    inputs = {}
    for name in SOURCE_HASHES:
        parent = INTEGRATION if name.startswith("LEGACY-") else ORIGINAL
        if name.endswith(".rgba"):
            parent = ROOT / "assets/fleet/sh001"
        inputs[name] = (parent / name).read_bytes()
    for name, data in inputs.items():
        expected = provenance["paths"][name]
        if pin(data) != {"bytes": expected["bytes"], "sha256": SOURCE_HASHES[name]} or expected["sha256"] != SOURCE_HASHES[name]:
            raise ValueError(f"Changed original SH046 source {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode())
    detailed, native = resource["design"], resource["nativeDesign"]
    archive = json.loads(inputs["native-export.json"])
    contract = tomllib.loads(inputs["contract.toml"].decode())
    require(resource["id"] == archive["id"] == contract["id"] == "SH046", 'wrong original SH046 identity')
    require(native == archive["nativeDesign"], 'changed approved original native mesh')
    require(archive["coordinateUnits"] == "metres", 'wrong original metre units')
    require(archive["sourceSHA256"] == contract["source"]["geometrySHA256"], 'changed approved original mesh digest')
    require(len(native["faces"]) <= contract["geometry"]["nativeFaceLimit"] == 512, 'wrong original native face limit')
    for mesh in (detailed, native):
        require(mesh["paletteName"] == "sand-maroon", 'wrong original palette')
        require(mesh["license"] == mesh["appearanceLicense"] == "MIT", 'wrong original visual licence')
        require([item["asset"] for item in mesh["textures"]] == TEXTURES, 'wrong original texture atlas order')
    return provenance, inputs, resource, detailed, native


def studio_definition(provenance, inputs, resource, detailed, native, detail_membership, native_membership, streams):
    finite = json.loads(inputs["finite-dry-source.json"])["emptyDryReference"]
    require(finite["massKg"] == json.loads(inputs["engineering.json"])["dryMassKilograms"] == 51822, 'changed original dry mass')
    require(finite["id"] == "finite-intrinsic-empty-harpy-dropship", 'wrong original dry reference')
    probes = []
    for route in json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes()):
        require(route["clearanceMillimetres"] > PROBE_RADIUS * 1000, 'insufficient original probe clearance')
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(start, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": start, "end": end, "radiusMetres": PROBE_RADIUS})

    def mesh(source, groups):
        return {"vertices": source["vertices"], "triangles": source["faces"], "groups": list(groups)}

    return {
        "schema": 1, "id": "SH046", "name": "Harpy Dropship", "metresPerUnit": 1, "roundingMetres": RADIUS,
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
            "sourceId": "SH046", "revision": provenance["revision"],
            "sourceHashes": {name: sha256(data).hexdigest() for name, data in inputs.items()},
            "derivedHashes": {name: pin(data) for name, data in streams.items()},
            "engineeringReference": finite["id"], "license": inputs["LICENSE"].decode(),
            "originalMetresPerUnit": 1, "scope": "catalogue inspection only",
        },
    }


def prepare():
    provenance, inputs, resource, detailed, native = load_source()
    slots = {name: slot for slot, name in enumerate(dict.fromkeys(SHAPE_NAMES))}
    require(len(slots) == 27, 'wrong original support count')
    detail_membership = assignments(detailed, SHAPE_NAMES, slots)
    native_membership = assignments(native, [SHAPE_NAMES[i] for i in NATIVE_COMPONENTS], slots)
    detail_count, native_count = len(detail_membership), len(native_membership)
    require((detail_count, native_count) == (1628, 144), "wrong original vertex count")
    membership = detail_membership + native_membership
    vertices = detailed["vertices"] + native["vertices"]
    faces = detailed["faces"] + [[i + detail_count for i in face] for face in native["faces"]]
    streams = {
        "source.vertices": struct.pack("<8sII", b"MSH046V1", detail_count, native_count)
            + b"".join(struct.pack("<3d", *p) for p in vertices),
        "source-triangles.bin": b"".join(struct.pack("<3I", *f) for f in faces),
        "source-piece-membership.bin": membership,
        "PIECES.json": encode_json({"pieces": list(slots), "roundingMetres": RADIUS}),
        "visual.bin": b"DROPVIZ1" + encode_visual(detailed)[8:],
        "native-visual.bin": b"DROPVIZ1" + encode_visual(native)[8:],
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
        "id": "SH046", "name": "Harpy Dropship", "sourceUnits": "metres", "revision": REVISION,
        "references": {"detailed": [1628, 3132, 31], "native": [144, 240, 12]},
        "nativeFaceLimit": 512, "groups": 27, "roundingMetres": RADIUS, "pins": SOURCE_HASHES,
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
            require(path.read_bytes() == data, f"changed SH046 encoding: {name}")
        else:
            replace_output(path, data)


if __name__ == "__main__":
    main()
