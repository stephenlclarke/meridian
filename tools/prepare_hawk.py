"""Build the SH147 collision, visual, and Studio streams from pinned originals."""

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
ORIGINAL = ROOT / "assets/fleet/sh147"
INTEGRATION = ROOT / "integration/sh147-hawk"
STUDIO_NAME = "HAWK-STUDIO.json"
REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
SOURCE_HASHES = {'resource.toml': '1ab7186108850acaf274f57702759d98dd11ccef597b83af60d38dd687a55b17',
 'native-export.json': 'b907bbb8a337db05a22dbf4eeee78811d89d52d61587e3f8524bfcbef1b12cb8',
 'contract.toml': 'f868fb1ab5eb44df24e1db6d427bd95e793e390799fabf64d4195b70be3994e1',
 'engineering.json': 'f392487ad2c5d5d6c9907bf3122823e0569eaa5b1c6fb842de7825696215c728',
 'finite-dry-source.json': '4884a0d9b7019fec230329621b07145acced466d9ffd43b6cee9ce270c2b666a',
 'LEGACY-FINITE-DRY.swift.txt': '502b519190023779406270bca80e63a3678dd0f8bfffe40552a63b02b4eacc0c',
 'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
 'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
 'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
 'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
 'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d'}
SHAPE_NAMES = ('primary pressure envelope',
 'closed structural hoop',
 'port brace',
 'port nozzle',
 'port nozzle',
 'port nozzle',
 'starboard brace',
 'starboard nozzle',
 'starboard nozzle',
 'starboard nozzle',
 'canopy',
 'canopy ridge',
 'port service line',
 'port instrument housing',
 'port louvre 1',
 'port louvre 2',
 'port louvre 3',
 'port louvre 4',
 'port louvre 5',
 'starboard service line',
 'starboard instrument housing',
 'starboard louvre 1',
 'starboard louvre 2',
 'starboard louvre 3',
 'starboard louvre 4',
 'starboard louvre 5')
NATIVE_COMPONENTS = (0,1,2,3,6,7,10)
COLLARS = (1,)
COLLAR_SPANS = 8
RING_SAMPLING = ((64, 12), (8, 4))
DUAL_MEMBERSHIPS = 128
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
        raise ValueError("Changed frozen SH147 origin")
    inputs = {}
    for name in SOURCE_HASHES:
        parent = INTEGRATION if name.startswith("LEGACY-") else ORIGINAL
        if name.endswith(".rgba"):
            parent = ROOT / "assets/fleet/sh001"
        inputs[name] = (parent / name).read_bytes()
    for name, data in inputs.items():
        expected = provenance["paths"][name]
        if pin(data) != {"bytes": expected["bytes"], "sha256": SOURCE_HASHES[name]} or expected["sha256"] != SOURCE_HASHES[name]:
            raise ValueError(f"Changed original SH147 source {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode())
    detailed, native = resource["design"], resource["nativeDesign"]
    archive = json.loads(inputs["native-export.json"])
    contract = tomllib.loads(inputs["contract.toml"].decode())
    require(resource["id"] == archive["id"] == contract["id"] == "SH147", 'wrong original SH147 identity')
    require(native == archive["nativeDesign"], 'changed approved original native mesh')
    require(archive["coordinateUnits"] == "metres", 'wrong original metre units')
    require(archive["sourceSHA256"] == contract["source"]["geometrySHA256"], 'changed approved original mesh digest')
    require(len(native["faces"]) <= contract["geometry"]["nativeFaceLimit"] == 512, 'wrong original native face limit')
    catalogue, access = resource["catalogue"], contract["access"]
    require(catalogue["playerFlyable"] is catalogue["normalPurchaseEligible"] is False
            and catalogue["purchasePolicy"] == "pendingFabrication"
            and access["playerFlyable"] is True and access["purchasePolicy"] == "blueprintFabrication",
            "changed original Hawk access")
    require(len(detailed["faces"]) == 3852 and len(native["faces"]) == 192,
            "changed original Hawk face counts")
    finite = json.loads(inputs["finite-dry-source.json"])
    require(finite["metresPerSourceUnit"] == 1 and finite["coordinateUnits"] == "metres",
            "changed original Hawk calibration")
    for mesh in (detailed, native):
        require(mesh["paletteName"] == "sand-maroon", 'wrong original palette')
        require(mesh["license"] == mesh["appearanceLicense"] == "MIT", 'wrong original visual licence')
        require([item["asset"] for item in mesh["textures"]] == TEXTURES, 'wrong original texture atlas order')
    return provenance, inputs, resource, detailed, native


def collision_inputs():
    provenance, inputs, resource, detailed, native = load_source()
    ordinary = tuple(dict.fromkeys(name for i,name in enumerate(SHAPE_NAMES) if i not in COLLARS))
    ring_names = [f"{SHAPE_NAMES[index]} span {i+1}" for index in COLLARS for i in range(COLLAR_SPANS)]
    names = [*ring_names, *ordinary]
    slots = {name: index for index,name in enumerate(names)}
    meshes = []
    for model, authored, (major, minor) in zip(
        (detailed,native), (tuple(range(len(SHAPE_NAMES))), NATIVE_COMPONENTS), RING_SAMPLING, strict=True
    ):
        membership = [None] * len(model["vertices"])
        for group, source_index in zip(components(model), authored, strict=True):
            if source_index in COLLARS:
                require(group == list(range(group[0],group[0]+major*minor)),
                        "changed Hawk collar source grid")
                base = COLLARS.index(source_index) * COLLAR_SPANS
                for local,index in enumerate(group):
                    span, phase = divmod(local // minor, major // COLLAR_SPANS)
                    primary = base + span
                    previous = base + (span - 1) % COLLAR_SPANS
                    membership[index] = (primary,) if phase else tuple(sorted((primary, previous)))
            else:
                for index in group:
                    membership[index] = (slots[SHAPE_NAMES[source_index]],)
        require(all(set.intersection(*(set(membership[index]) for index in face)) for face in model["faces"]),
                "Hawk triangle spans its local supports")
        meshes.append((model, membership))
    require(len(names) == 29 and [len(model["vertices"]) for model,_ in meshes] == [1976,108],
            "changed Hawk support or vertex count")
    require(sum(len(row)==2 for _,membership in meshes for row in membership) == DUAL_MEMBERSHIPS,
            "changed Hawk collar seams")
    return provenance, inputs, resource, names, meshes


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
    require(finite["massKg"] == json.loads(inputs["engineering.json"])["dryMassKilograms"] == 24869, 'changed original dry mass')
    require(finite["id"] == "finite-intrinsic-empty-hawk", 'wrong original dry reference')
    probes = []
    for route in json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes()):
        require(route["clearanceMillimetres"] > PROBE_RADIUS * 1000, 'insufficient original probe clearance')
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(start, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": start, "end": end, "radiusMetres": PROBE_RADIUS})

    return {
        "schema": 1, "id": "SH147", "name": "Hawk", "metresPerUnit": 1, "roundingMetres": RADIUS,
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
            "sourceId": "SH147", "revision": provenance["revision"],
            "sourceHashes": {name: sha256(data).hexdigest() for name, data in inputs.items()},
            "derivedHashes": {name: pin(data) for name, data in streams.items()},
            "engineeringReference": finite["id"], "license": inputs["LICENSE"].decode(),
            "originalMetresPerUnit": 1, "scope": "catalogue inspection only",
        },
    }


def prepare():
    provenance, inputs, resource, names, meshes = collision_inputs()
    (detailed, detail_membership), (native, native_membership) = meshes
    detail_count, native_count = len(detail_membership), len(native_membership)
    membership = detail_membership + native_membership
    packed = bytes(value for row in membership for value in (row if len(row)==2 else (*row,ABSENT)))
    vertices = detailed["vertices"] + native["vertices"]
    faces = detailed["faces"] + [[index + detail_count for index in face] for face in native["faces"]]
    streams = {
        "source.vertices": struct.pack("<8sII", b"MSH147V1", detail_count, native_count)
            + b"".join(struct.pack("<3d", *vertex) for vertex in vertices),
        "source-triangles.bin": b"".join(struct.pack("<3I", *face) for face in faces),
        "source-piece-membership.bin": packed,
        "PIECES.json": encode_json({"pieces": names, "roundingMetres": RADIUS}),
        "visual.bin": b"HWKVIZ01" + encode_visual(detailed)[8:],
        "native-visual.bin": b"HWKVIZ01" + encode_visual(native)[8:],
    }
    fans = json.loads((INTEGRATION / "SUPPORT-FAN.json").read_bytes())
    require(list(fans) == names, "wrong original Hawk fan order")
    require(all(0 <= index < len(membership) and slot in membership[index]
                for slot, polygons in enumerate(fans.values()) for face in polygons for index in face),
            "Hawk support fan spans groups")
    definition = studio_definition(
        provenance, inputs, resource, detailed, native, detail_membership, native_membership, streams
    )
    streams[STUDIO_NAME] = encode_json(definition)
    streams["SOURCE-GEOMETRY.json"] = encode_json({
        "id": "SH147", "name": "Hawk", "sourceUnits": "metres", "revision": REVISION,
        "references": {"detailed": [1976, 3852, 26], "native": [108, 192, 7]},
        "nativeFaceLimit": 512, "groups": 29, "collarCountEachLevel": 1, "dualMembershipVertices": DUAL_MEMBERSHIPS, "roundingMetres": RADIUS, "pins": SOURCE_HASHES,
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
            require(path.read_bytes() == data, f"changed SH147 encoding: {name}")
        else:
            replace_output(path, data)


if __name__ == "__main__":
    main()
