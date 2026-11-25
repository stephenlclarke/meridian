"""Build the SH091 collision, visual, and Studio streams from pinned originals."""

import argparse
from hashlib import sha256
import json
import math
from pathlib import Path
import struct
import tomllib

from dzo_collar_support import ABSENT, collar_spans
from atomic_asset_output import checked_directory, checked_file, replace_output
from encode_goshawk_visual import TEXTURES
from import_goshawk_envelope_source import components
from prepare_eagle_cruiser_geometry import encode_visual

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / "assets/fleet/sh091"
INTEGRATION = ROOT / "integration/sh091-gopher"
STUDIO_NAME = "GOPHER-STUDIO.json"
REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
SOURCE_HASHES = {'resource.toml': 'd2b63a1974ee9e5864299a63dd01f37b0fd7a643477c20c2a573ec7176aac221',
 'native-export.json': 'abe94c225194c86f78b64011b4de7218f5213639602cacbfd65b96d3bc1ad796',
 'contract.toml': 'eda1e9dceb8b3594fa99bb2d8ed9f21e2e7fab320ee1ae3dc154b1b82aa52f81',
 'engineering.json': '70e788982a0c7fb468ee835a913a122f5c7dec607e2bd3aef897e963fa0200e1',
 'finite-dry-source.json': '32a90cce716a7d304981d2166f34dd951744e3726162fa429667f63d16c0e69a',
 'LEGACY-FINITE-DRY.swift.txt': '3db1802f627c2b38b939781d93d69424d1a1d9c418d3d3038e9c190bc3ac4e7d',
 'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
 'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
 'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
 'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
 'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d'}
SHAPE_NAMES = (
    'pressure-hull',
    'roll-frame',
    'bore-shaft',
    'bore-head',
    'port-frame-stay',
    'port-nozzle',
    'port-nozzle',
    'port-nozzle',
    'starboard-frame-stay',
    'starboard-nozzle',
    'starboard-nozzle',
    'starboard-nozzle',
    'canopy',
    'canopy-ridge',
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
NATIVE_COMPONENTS = (0,1,2,3,4,5,8,9,12)
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
        raise ValueError("Changed frozen SH091 origin")
    inputs = {}
    for name in SOURCE_HASHES:
        parent = INTEGRATION if name.startswith("LEGACY-") else ORIGINAL
        if name.endswith(".rgba"):
            parent = ROOT / "assets/fleet/sh001"
        inputs[name] = (parent / name).read_bytes()
    for name, data in inputs.items():
        expected = provenance["paths"][name]
        if pin(data) != {"bytes": expected["bytes"], "sha256": SOURCE_HASHES[name]} or expected["sha256"] != SOURCE_HASHES[name]:
            raise ValueError(f"Changed original SH091 source {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode())
    detailed, native = resource["design"], resource["nativeDesign"]
    archive = json.loads(inputs["native-export.json"])
    contract = tomllib.loads(inputs["contract.toml"].decode())
    require(resource["id"] == archive["id"] == contract["id"] == "SH091", 'wrong original SH091 identity')
    require(native == archive["nativeDesign"], 'changed approved original native mesh')
    require(archive["coordinateUnits"] == "metres", 'wrong original metre units')
    require(archive["sourceSHA256"] == contract["source"]["geometrySHA256"], 'changed approved original mesh digest')
    require(len(native["faces"]) <= contract["geometry"]["nativeFaceLimit"] == 512, 'wrong original native face limit')
    catalogue, access = resource["catalogue"], contract["access"]
    require(catalogue["playerFlyable"] is catalogue["normalPurchaseEligible"] is False
            and catalogue["purchasePolicy"] == "pendingFabrication"
            and access["playerFlyable"] is True and access["purchasePolicy"] == "blueprintFabrication",
            "changed original Gopher access")
    for mesh in (detailed, native):
        require(mesh["paletteName"] == "slate-orange", 'wrong original palette')
        require(mesh["license"] == mesh["appearanceLicense"] == "MIT", 'wrong original visual licence')
        require([item["asset"] for item in mesh["textures"]] == TEXTURES, 'wrong original texture atlas order')
    return provenance, inputs, resource, detailed, native


def collision_inputs():
    provenance, inputs, resource, detailed, native = load_source()
    slots = {name:index for index,name in enumerate(dict.fromkeys(SHAPE_NAMES))}
    source = (detailed, native)
    membership = (assignments(detailed,SHAPE_NAMES,slots),
                  assignments(native,[SHAPE_NAMES[i] for i in NATIVE_COMPONENTS],slots))
    combined = membership[0] + membership[1]
    names = []
    groups = []
    for label,index in slots.items():
        vertices = [i for i,slot in enumerate(combined) if slot==index]
        if label=="roll-frame":
            for span, ids in enumerate(collar_spans(vertices,len(membership[0])),1):
                names.append(f"roll-frame-span-{span}")
                groups.append(ids)
        else:
            names.append(label)
            groups.append(vertices)
    numbers = [[] for _ in combined]
    for index,group in enumerate(groups):
        for vertex in group:
            numbers[vertex].append(index)
    require(len(names)==31 and sum(len(row)==2 for row in numbers)==128
            and all(1<=len(row)<=2 for row in numbers), "wrong Gopher roll-frame grouping")
    detail_count = len(membership[0])
    rows = (numbers[:detail_count], numbers[detail_count:])
    for model, ids in zip(source, rows, strict=True):
        require(all(set.intersection(*(set(ids[i]) for i in face)) for face in model["faces"]),
                "Gopher face crosses roll-frame supports")
    return provenance, inputs, resource, source, names, rows


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


def studio_definition(provenance, inputs, resource, detailed, native, detail_membership, native_membership, streams):
    finite = json.loads(inputs["finite-dry-source.json"])["emptyDryReference"]
    require(finite["massKg"] == json.loads(inputs["engineering.json"])["dryMassKilograms"] == 20633, 'changed original dry mass')
    require(finite["id"] == "finite-intrinsic-empty-gopher", 'wrong original dry reference')
    probes = []
    for route in json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes()):
        require(route["clearanceMillimetres"] > PROBE_RADIUS * 1000, 'insufficient original probe clearance')
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(start, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": start, "end": end, "radiusMetres": PROBE_RADIUS})

    return {
        "schema": 1, "id": "SH091", "name": "Gopher", "metresPerUnit": 1, "roundingMetres": RADIUS,
        "detail": studio_mesh(detailed, detail_membership), "native": studio_mesh(native, native_membership),
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
            "sourceId": "SH091", "revision": provenance["revision"],
            "sourceHashes": {name: sha256(data).hexdigest() for name, data in inputs.items()},
            "derivedHashes": {name: pin(data) for name, data in streams.items()},
            "engineeringReference": finite["id"], "license": inputs["LICENSE"].decode(),
            "originalMetresPerUnit": 1, "scope": "catalogue inspection only",
        },
    }


def prepare():
    provenance, inputs, resource, (detailed,native), names, (detail_membership,native_membership) = collision_inputs()
    detail_count, native_count = len(detail_membership), len(native_membership)
    require((detail_count, native_count) == (2402,122), "wrong original vertex count")
    membership = detail_membership + native_membership
    vertices = detailed["vertices"] + native["vertices"]
    faces = detailed["faces"] + [[i+detail_count for i in face] for face in native["faces"]]
    packed = bytes(slot for row in membership for slot in (row if len(row)==2 else [row[0],ABSENT]))
    streams = {
        "source.vertices": struct.pack("<8sII", b"MSH091V1", detail_count, native_count)
            + b"".join(struct.pack("<3d", *p) for p in vertices),
        "source-triangles.bin": b"".join(struct.pack("<3I", *f) for f in faces),
        "collision-piece-membership.bin": packed,
        "PIECES.json": encode_json({"pieces": names, "roundingMetres": RADIUS}),
        "visual.bin": b"GOPHVIZ1" + encode_visual(detailed)[8:],
        "native-visual.bin": b"GOPHVIZ1" + encode_visual(native)[8:],
    }
    fans = json.loads((INTEGRATION / "SUPPORT-FAN.json").read_bytes())
    require(list(fans) == names, "wrong Gopher roll-frame fan order")
    require(all(0<=index<len(membership) and slot in membership[index]
                for slot, fan in enumerate(fans.values()) for face in fan for index in face),
            "Gopher support fan spans pieces")
    streams[STUDIO_NAME] = encode_json(studio_definition(
        provenance, inputs, resource, detailed, native, detail_membership, native_membership, streams
    ))
    streams["SOURCE-GEOMETRY.json"] = encode_json({
        "id": "SH091", "name": "Gopher", "sourceUnits": "metres", "revision": REVISION,
        "references": {"detailed": [2402,4696,28], "native": [122,212,9]},
        "nativeFaceLimit": 512, "groups": len(names), "dualMembershipVertices": 128,
        "roundingMetres": RADIUS, "pins": SOURCE_HASHES,
        "outputs": {name: pin(data) for name,data in streams.items()},
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
            require(path.read_bytes() == data, f"changed SH091 encoding: {name}")
        else:
            replace_output(path, data)


if __name__ == "__main__":
    main()
