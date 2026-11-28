"""Build the SH130 collision, visual, and Studio streams from pinned originals."""

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
ORIGINAL = ROOT / "assets/fleet/sh130"
INTEGRATION = ROOT / "integration/sh130-marmot-prospector"
STUDIO_NAME = "MARMOT-PROSPECTOR-STUDIO.json"
REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
SOURCE_HASHES = {'resource.toml': '868e1c743931bb52e3b8fb820368ca972ab1ca5d165bfbc13992caa5824222d5',
 'native-export.json': '9a3cbf380d0cdbe5feb66fc9a835ccf337fda575b56ea8b13c69137ffe1cbb39',
 'contract.toml': '83ce030b30b99f053df1a789a8f0656210cb01ec729b5db30c9166b8599951af',
 'engineering.json': 'f1669515728857220c23bcec4de82303e0971040fc65dfa74855e0c0f3ef87a6',
 'finite-dry-source.json': 'c188f6a3ccbd018560afee5b4a6c3d79342d7f1b0b2ed44d6ed26c1aeae27309',
 'LEGACY-FINITE-DRY.swift.txt': 'f005334e82ca9012d5ffb2eea64fb71a49ea0fcc93f706395445b7e8aaae5e65',
 'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
 'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
 'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
 'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
 'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d'}
SHAPE_NAMES = ('pressure deck',
 'aft envelope',
 'port drill beam',
 'port drill shaft',
 'port drill collar',
 'central drill beam',
 'central drill shaft',
 'central drill collar',
 'starboard drill beam',
 'starboard drill shaft',
 'starboard drill collar',
 'port radiator',
 'port nozzle',
 'port nozzle',
 'port nozzle',
 'starboard radiator',
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
NATIVE_COMPONENTS = (0,1,2,3,4,5,6,7,8,9,10,11,12,15,16,19)
COLLARS = (4, 7, 10)
COLLAR_SPANS = 8
RING_SAMPLING = ((64, 12), (8, 4))
DUAL_MEMBERSHIPS = 384
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
        raise ValueError("Changed frozen SH130 origin")
    inputs = {}
    for name in SOURCE_HASHES:
        parent = INTEGRATION if name.startswith("LEGACY-") else ORIGINAL
        if name.endswith(".rgba"):
            parent = ROOT / "assets/fleet/sh001"
        inputs[name] = (parent / name).read_bytes()
    for name, data in inputs.items():
        expected = provenance["paths"][name]
        if pin(data) != {"bytes": expected["bytes"], "sha256": SOURCE_HASHES[name]} or expected["sha256"] != SOURCE_HASHES[name]:
            raise ValueError(f"Changed original SH130 source {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode())
    detailed, native = resource["design"], resource["nativeDesign"]
    archive = json.loads(inputs["native-export.json"])
    contract = tomllib.loads(inputs["contract.toml"].decode())
    require(resource["id"] == archive["id"] == contract["id"] == "SH130", 'wrong original SH130 identity')
    require(native == archive["nativeDesign"], 'changed approved original native mesh')
    require(archive["coordinateUnits"] == "metres", 'wrong original metre units')
    require(archive["sourceSHA256"] == contract["source"]["geometrySHA256"], 'changed approved original mesh digest')
    require(len(native["faces"]) <= contract["geometry"]["nativeFaceLimit"] == 512, 'wrong original native face limit')
    catalogue, access = resource["catalogue"], contract["access"]
    require(catalogue["playerFlyable"] is catalogue["normalPurchaseEligible"] is False
            and catalogue["purchasePolicy"] == "pendingFabrication"
            and access["playerFlyable"] is True and access["purchasePolicy"] == "blueprintFabrication",
            "changed original Marmot Prospector access")
    require(len(detailed["faces"]) == 8484 and len(native["faces"]) == 448,
            "changed original Marmot Prospector face counts")
    finite = json.loads(inputs["finite-dry-source.json"])
    require(finite["metresPerSourceUnit"] == 1 and finite["coordinateUnits"] == "metres",
            "changed original Marmot Prospector calibration")
    for mesh in (detailed, native):
        require(mesh["paletteName"] == "slate-orange", 'wrong original palette')
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
                        "changed Marmot Prospector collar source grid")
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
                "Marmot Prospector triangle spans its local supports")
        meshes.append((model, membership))
    require(len(names) == 52 and [len(model["vertices"]) for model,_ in meshes] == [4306,250],
            "changed Marmot Prospector support or vertex count")
    require(sum(len(row)==2 for _,membership in meshes for row in membership) == DUAL_MEMBERSHIPS,
            "changed Marmot Prospector collar seams")
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
    require(finite["massKg"] == json.loads(inputs["engineering.json"])["dryMassKilograms"] == 92530, 'changed original dry mass')
    require(finite["id"] == "finite-intrinsic-empty-marmot-prospector", 'wrong original dry reference')
    probes = []
    for route in json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes()):
        require(route["clearanceMillimetres"] > PROBE_RADIUS * 1000, 'insufficient original probe clearance')
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(start, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": start, "end": end, "radiusMetres": PROBE_RADIUS})

    return {
        "schema": 1, "id": "SH130", "name": "Marmot Prospector", "metresPerUnit": 1, "roundingMetres": RADIUS,
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
            "sourceId": "SH130", "revision": provenance["revision"],
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
        "source.vertices": struct.pack("<8sII", b"MSH130V1", detail_count, native_count)
            + b"".join(struct.pack("<3d", *vertex) for vertex in vertices),
        "source-triangles.bin": b"".join(struct.pack("<3I", *face) for face in faces),
        "source-piece-membership.bin": packed,
        "PIECES.json": encode_json({"pieces": names, "roundingMetres": RADIUS}),
        "visual.bin": b"MRMTVIZ1" + encode_visual(detailed)[8:],
        "native-visual.bin": b"MRMTVIZ1" + encode_visual(native)[8:],
    }
    fans = json.loads((INTEGRATION / "SUPPORT-FAN.json").read_bytes())
    require(list(fans) == names, "wrong original Marmot Prospector fan order")
    require(all(0 <= index < len(membership) and slot in membership[index]
                for slot, polygons in enumerate(fans.values()) for face in polygons for index in face),
            "Marmot Prospector support fan spans groups")
    definition = studio_definition(
        provenance, inputs, resource, detailed, native, detail_membership, native_membership, streams
    )
    streams[STUDIO_NAME] = encode_json(definition)
    streams["SOURCE-GEOMETRY.json"] = encode_json({
        "id": "SH130", "name": "Marmot Prospector", "sourceUnits": "metres", "revision": REVISION,
        "references": {"detailed": [4306, 8484, 35], "native": [250, 448, 16]},
        "nativeFaceLimit": 512, "groups": 52, "collarCountEachLevel": 3, "dualMembershipVertices": DUAL_MEMBERSHIPS, "roundingMetres": RADIUS, "pins": SOURCE_HASHES,
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
            require(path.read_bytes() == data, f"changed SH130 encoding: {name}")
        else:
            replace_output(path, data)


if __name__ == "__main__":
    main()
