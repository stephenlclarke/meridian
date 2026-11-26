"""Build the SH098 collision, visual, and Studio streams from pinned originals."""

import argparse
from collections import Counter
from fractions import Fraction
from hashlib import sha256
import json
import math
from pathlib import Path
import struct
import tomllib

from atomic_asset_output import checked_directory, checked_file, replace_output
from encode_goshawk_visual import TEXTURES
from banteng_art_mesh import group_labels
from prepare_eagle_cruiser_geometry import encode_visual

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / "assets/fleet/sh098"
INTEGRATION = ROOT / "integration/sh098-anoa"
STUDIO_NAME = "ANOA-STUDIO.json"
REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
SOURCE_HASHES = {'resource.toml': '7d8f4db7245f16dcef000d9db86ce5d0314244ee5d342429b40f12dc22427e5b',
 'native-export.json': '45e0dd7a2d4dd503b8f99409cce0fda100e20a7a620a22ceb95e98e20535591c',
 'contract.toml': 'ed1addd7a35c62ff86bfb2af5fe3c4b83f826b8f98d36b003f48e050c0e48f5a',
 'engineering.json': '9b19e82a35a99ec7e89e4247cde8aa767f26f5c26057e1ee25c5420d4aa51ce0',
 'finite-dry-source.json': 'dbb58f69f679584db1947e0a4b8237b3b2bb0b5a06076ec4c0f6fdf5c53e6513',
 'LEGACY-FINITE-DRY.swift.txt': '3c4551f9396de69151c67863b7ded5c285ff143288e0e6665e8b35e4dcfa228c',
 'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
 'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
 'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
 'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
 'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d'}
OWNERSHIP_SHA = "e5c790227927e5891c0d0fbcbc680b7a1dde99c2ac93060b0afa708cf6c792ba"
CANDIDATE_SHA = "d150c102f20437c477023c9040841a8cc824fd56aa433bf2f25461503d214c81"
SCALE = Fraction(190, 9)
GROUP_COUNT = 88
CANDIDATE_VERTICES = 320
COLLAR_SEAMS = 368
RING_SAMPLING = ((48, 12), (24, 8), (8, 3))
COLLAR_SPANS = 8
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
        raise ValueError("Changed frozen SH098 origin")
    inputs = {}
    for name in SOURCE_HASHES:
        parent = INTEGRATION if name.startswith("LEGACY-") else ORIGINAL
        if name.endswith(".rgba"):
            parent = ROOT / "assets/fleet/sh001"
        inputs[name] = (parent / name).read_bytes()
    for name, data in inputs.items():
        expected = provenance["paths"][name]
        if pin(data) != {"bytes": expected["bytes"], "sha256": SOURCE_HASHES[name]} or expected["sha256"] != SOURCE_HASHES[name]:
            raise ValueError(f"Changed original SH098 source {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode())
    detailed, native = resource["design"], resource["nativeDesign"]
    archive = json.loads(inputs["native-export.json"])
    contract = tomllib.loads(inputs["contract.toml"].decode())
    require(resource["id"] == archive["id"] == contract["id"] == "SH098", 'wrong original SH098 identity')
    require(native == archive["nativeDesign"], 'changed approved original native mesh')
    finite = json.loads(inputs["finite-dry-source.json"])
    require(finite["coordinateUnits"] == "metres" and float(SCALE) == finite["metresPerSourceUnit"],
            "changed Anoa original calibration")
    require(archive["sourceSHA256"] == contract["source"]["geometrySHA256"], 'changed approved original mesh digest')
    require(len(native["faces"]) == 1928 and contract["geometry"]["nativeFaceLimit"] == 512,
            "changed Anoa archived face count or candidate ceiling")
    catalogue, access = resource["catalogue"], contract["access"]
    require(catalogue["playerFlyable"] is catalogue["normalPurchaseEligible"] is False
            and catalogue["purchasePolicy"] == "pendingFabrication"
            and access["playerFlyable"] is True and access["purchasePolicy"] == "blueprintFabrication",
            "changed original Anoa access")
    for mesh in (detailed, native):
        require(mesh["paletteName"] == "forest-silver", 'wrong original palette')
        require(mesh["license"] == mesh["appearanceLicense"] == "MIT", 'wrong original visual licence')
        require([item["asset"] for item in mesh["textures"]] == TEXTURES, 'wrong original texture atlas order')
    ownership_bytes = (INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes()
    candidate_bytes = (ORIGINAL / "GENERATED-COLLISION.json").read_bytes()
    require(sha256(ownership_bytes).hexdigest() == OWNERSHIP_SHA, "changed Anoa ownership")
    require(sha256(candidate_bytes).hexdigest() == CANDIDATE_SHA, "changed Anoa candidate")
    ownership = json.loads(ownership_bytes)["models"]
    candidate = json.loads(candidate_bytes)
    nominee = candidate["nativeCandidate"]
    require(len(nominee["faces"]) == 512 and len(candidate["parts"]) == 34
            and candidate["r3SHA256"] == SOURCE_HASHES["contract.toml"]
            and candidate["originalSourceSHA256"] == archive["sourceSHA256"],
            "wrong Anoa candidate identity")
    require(all(nominee[key] == native[key] for key in ("textures", "materials", "paletteName")),
            "changed Anoa candidate appearance")
    references = (detailed, native, nominee)
    parts = (ownership["design"]["parts"], ownership["nativeDesign"]["parts"], candidate["parts"])
    return provenance, inputs, resource, references, parts


def collision_inputs():
    provenance, inputs, resource, references, parts = load_source()
    labels = [group_labels(mesh, records) for mesh, records in zip(references, parts, strict=True)]
    ordinary = list(dict.fromkeys(label for values in labels for label in values if label[0] != "radial structural collar"))
    names = [f"{end} collar span {i+1}" for end in ("aft", "forward") for i in range(COLLAR_SPANS)]
    slots = {label: i for i, label in enumerate(ordinary, len(names))}
    counts = Counter(name for name, _ in ordinary)
    names += [name if counts[name] == 1 else f"{name} ({index})" for name, index in ordinary]
    require(len(names) == len(set(names)) == GROUP_COUNT, "changed Anoa support inventory")
    meshes = []
    for mesh, records, group_names, (major, minor) in zip(references, parts, labels, RING_SAMPLING, strict=True):
        collars = [record for record in records if record["name"] == "radial structural collar"]
        require(len(collars) == 2 and all(record["vertexCount"] == major*minor for record in collars),
                "wrong Anoa collar sampling")
        width = major // COLLAR_SPANS
        members = []
        for i, label in enumerate(group_names):
            if label[0] == "radial structural collar":
                index = label[1]-1
                offset = index*COLLAR_SPANS
                row = (i-collars[index]["vertexStart"]) // minor
                group, phase = divmod(row, width)
                neighbors = [group, (group-1) % COLLAR_SPANS] if phase == 0 else [group]
                members.append(tuple(sorted(offset+j for j in neighbors)))
            else:
                members.append((slots[label],))
        require(all(set.intersection(*(set(members[i]) for i in tri)) for tri in mesh["faces"]),
                "Anoa source triangle crosses collision supports")
        meshes.append((dict(mesh, vertices=[[a*float(SCALE) for a in point] for point in mesh["vertices"]]), members))
    require([len(mesh["vertices"]) for mesh, _ in meshes] == [3792,1028,CANDIDATE_VERTICES], "changed Anoa mesh vertices")
    require(sum(len(row)==2 for _, membership in meshes for row in membership) == COLLAR_SEAMS,
            "changed Anoa collar seams")
    return provenance, inputs, resource, references, names, meshes


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


def studio_definition(provenance, inputs, resource, detailed, candidate, streams):
    detail_model, _ = detailed
    finite = json.loads(inputs["finite-dry-source.json"])["emptyDryReference"]
    require(finite["massKg"] == json.loads(inputs["engineering.json"])["dryMassKilograms"] == 442139, 'changed original dry mass')
    require(finite["id"] == "finite-intrinsic-empty-anoa", 'wrong original dry reference')
    probes = []
    for route in json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes()):
        require(route["clearanceMillimetres"] > PROBE_RADIUS * 1000, 'insufficient original probe clearance')
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(start, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": start, "end": end, "radiusMetres": PROBE_RADIUS})

    return {
        "schema": 1, "id": "SH098", "name": "Anoa", "metresPerUnit": 1, "roundingMetres": RADIUS,
        "detail": studio_mesh(*detailed), "native": studio_mesh(*candidate),
        "materials": [
            {"name": m["name"], "rgb": [m["colour"] >> shift & 255 for shift in (24, 16, 8)],
             "alpha": m["colour"] & 255, "emissive": m["emissive"]}
            for m in detail_model["materials"]
        ],
        "faceMaterials": detail_model["faceMaterials"],
        "faceTextures": [
            {"uv": mapping["uv"], "texture": detail_model["textures"][mapping["texture"]]["asset"]
             if mapping["texture"] >= 0 else None}
            for mapping in detail_model["faceTextures"]
        ],
        "engineering": {
            "massKg": finite["massKg"],
            "centerOfMass": finite["centreMetres"],
            "inertia": [finite["inertiaAboutCentreOfMassKgM2"][i:i + 3] for i in range(0, 9, 3)],
        },
        "flight": {"maxSpeed": resource["game"]["speed"]},
        "clearances": probes,
        "provenance": {
            "sourceId": "SH098", "revision": provenance["revision"],
            "sourceHashes": {name: sha256(data).hexdigest() for name, data in inputs.items()},
            "derivedHashes": {name: pin(data) for name, data in streams.items()},
            "engineeringReference": finite["id"], "license": inputs["LICENSE"].decode(),
            "originalMetresPerUnit": float(SCALE), "scope": "catalogue inspection only",
        },
    }


def prepare():
    provenance, inputs, resource, _, names, meshes = collision_inputs()
    detail, native, candidate = meshes
    vertices = [point for model, _ in meshes for point in model["vertices"]]
    membership = [row for _, groups in meshes for row in groups]
    counts = [len(model["vertices"]) for model,_ in meshes]
    faces = []
    offset = 0
    for model, _ in meshes:
        faces.extend([index+offset for index in face] for face in model["faces"])
        offset += len(model["vertices"])
    packed = bytes(value for row in membership for value in (row if len(row)==2 else (*row, ABSENT)))
    streams = {
        "source.vertices": struct.pack("<8sIII", b"MSH098V1", *counts)
            + b"".join(struct.pack("<3d", *p) for p in vertices),
        "source-triangles.bin": b"".join(struct.pack("<3I", *f) for f in faces),
        "collision-piece-membership.bin": packed,
        "PIECES.json": encode_json({"pieces": names, "roundingMetres": RADIUS}),
        "visual.bin": b"ANOAVIZ1" + encode_visual(detail[0])[8:],
        "native-visual.bin": b"ANOAVIZ1" + encode_visual(native[0])[8:],
    }
    fans = json.loads((INTEGRATION / "SUPPORT-FAN.json").read_bytes())
    require(list(fans) == names, "wrong Anoa fan order")
    require(all(0 <= index < len(membership) and slot in membership[index]
                for slot, faces in enumerate(fans.values()) for face in faces for index in face),
            "Anoa local support fan spans pieces")
    streams[STUDIO_NAME] = encode_json(studio_definition(provenance, inputs, resource, detail, candidate, streams))
    streams["SOURCE-GEOMETRY.json"] = encode_json({
        "id": "SH098", "name": "Anoa", "sourceScaleRatio": [SCALE.numerator, SCALE.denominator], "revision": REVISION,
        "references": {"detailed": [3792, 6720, 218], "archivedNative": [1028, 1928, 34], "candidateNative": [320, 512, 34]},
        "nativeFaceLimit": 512, "groups": len(names), "dualMembershipVertices": COLLAR_SEAMS, "collarSpansPerRing": COLLAR_SPANS, "roundingMetres": RADIUS,
        "pins": SOURCE_HASHES | {"GENERATED-COLLISION.json": CANDIDATE_SHA, "ORIGINAL-OWNERSHIP.json": OWNERSHIP_SHA},
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
            require(path.read_bytes() == data, f"changed SH098 encoding: {name}")
        else:
            replace_output(path, data)


if __name__ == "__main__":
    main()
