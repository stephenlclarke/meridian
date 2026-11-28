"""Build the SH169 collision, visual, and Studio streams from pinned originals."""

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
from prepare_eagle_cruiser_geometry import encode_visual

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / "assets/fleet/sh169"
INTEGRATION = ROOT / "integration/sh169-caracara-mk-i"
STUDIO_NAME = "CARACARA-MK-I-STUDIO.json"
REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
SOURCE_HASHES = {'resource.toml': '1ffb117ac1f05b759d9e06c52fdd0fdb4d7d4b7aa3cae3d1fdbe6998111f195d',
 'native-export.json': '7dfb960a6b09d36e2425269bda78af408982799de17a1f717e7e8095d90d1003',
 'contract.toml': '45ee45d2ed594bf07c527138655844269d6c8987740d8f547cc8ef4818f0a2e1',
 'engineering.json': '206b709130bbe749f089e84127abaf79218ecadcef3cb5ca5821b0d60d55821c',
 'finite-dry-source.json': 'ec9377446ede6fe605e47b8b78cceeb64a2ee8f02c810e4f367d5f01c8d8157b',
 'LEGACY-FINITE-DRY.swift.txt': 'e7c6adc54754991ad50f5ebc912fe4bb3bc843fb6b4b3d91560bb70cf9a0745b',
 'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
 'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
 'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
 'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
 'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d'}
OWNERSHIP_SHA = "d2a82a8c5a9ff54e8e8691da2e058f0f03d6e66d207f5d0ce854853c5240aad8"
CANDIDATE_SHA = "1267caabaad060f4fa7b55c6da62d12d46a7afe5820de93c3e43e9c4520bf222"
SCALE = Fraction(7500, 697)
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
        raise ValueError("Changed frozen SH169 origin")
    inputs = {}
    for name in SOURCE_HASHES:
        parent = INTEGRATION if name.startswith("LEGACY-") else ORIGINAL
        if name.endswith(".rgba"):
            parent = ROOT / "assets/fleet/sh001"
        inputs[name] = (parent / name).read_bytes()
    for name, data in inputs.items():
        expected = provenance["paths"][name]
        if pin(data) != {"bytes": expected["bytes"], "sha256": SOURCE_HASHES[name]} or expected["sha256"] != SOURCE_HASHES[name]:
            raise ValueError(f"Changed original SH169 source {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode())
    detailed, native = resource["design"], resource["nativeDesign"]
    archive = json.loads(inputs["native-export.json"])
    contract = tomllib.loads(inputs["contract.toml"].decode())
    require(resource["id"] == archive["id"] == contract["id"] == "SH169", 'wrong original SH169 identity')
    require(native == archive["nativeDesign"], 'changed approved original native mesh')
    finite = json.loads(inputs["finite-dry-source.json"])
    require(finite["coordinateUnits"] == "metres" and float(SCALE) == finite["metresPerSourceUnit"],
            "changed Caracara Mk I original calibration")
    require(archive["sourceSHA256"] == contract["source"]["geometrySHA256"], 'changed approved original mesh digest')
    require(len(native["faces"]) == 1364 and contract["geometry"]["nativeFaceLimit"] == 512,
            "changed Caracara Mk I archived face count or candidate ceiling")
    catalogue, access = resource["catalogue"], contract["access"]
    require(catalogue["playerFlyable"] is catalogue["normalPurchaseEligible"] is False
            and catalogue["purchasePolicy"] == "pendingFabrication"
            and access["playerFlyable"] is True and access["purchasePolicy"] == "blueprintFabrication",
            "changed original Caracara Mk I access")
    for mesh in (detailed, native):
        require(mesh["paletteName"] == "petrol-brass", 'wrong original palette')
        require(mesh["license"] == mesh["appearanceLicense"] == "MIT", 'wrong original visual licence')
        require([item["asset"] for item in mesh["textures"]] == TEXTURES, 'wrong original texture atlas order')
    ownership_bytes = (INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes()
    candidate_bytes = (ORIGINAL / "GENERATED-COLLISION.json").read_bytes()
    require(sha256(ownership_bytes).hexdigest() == OWNERSHIP_SHA, "changed Caracara Mk I ownership")
    require(sha256(candidate_bytes).hexdigest() == CANDIDATE_SHA, "changed Caracara Mk I candidate")
    ownership = json.loads(ownership_bytes)
    require(ownership["id"] == "SH169", "wrong Caracara Mk I ownership identity")
    ownership = ownership["models"]
    candidate = json.loads(candidate_bytes)
    nominee = candidate["nativeCandidate"]
    require(len(nominee["faces"]) == 512 and len(candidate["parts"]) == 21 and candidate["id"] == "SH169"
            and candidate["r3SHA256"] == SOURCE_HASHES["contract.toml"]
            and candidate["originalSourceSHA256"] == archive["sourceSHA256"],
            "wrong Caracara Mk I candidate identity")
    require(all(nominee[key] == native[key] for key in ("textures", "materials", "paletteName")),
            "changed Caracara Mk I candidate appearance")
    references = (detailed, native, nominee)
    parts = (ownership["design"]["parts"], ownership["nativeDesign"]["parts"], candidate["parts"])
    return provenance, inputs, resource, references, parts


def group_labels(mesh, parts):
    label_counts = Counter()
    labels = []
    last_face = 0
    for part in parts:
        start = len(labels)
        end = start + part["vertexCount"]
        following_face = last_face + part["faceCount"]
        require((part["vertexStart"], part["faceStart"]) == (start, last_face)
                and following_face <= len(mesh["faces"])
                and all(start <= i < end for face in mesh["faces"][last_face:following_face] for i in face),
                "Caracara Mk I support leaves its original creator part")
        name = part["name"]
        label_counts[name] += 1
        labels.extend([(name, label_counts[name])] * part["vertexCount"])
        last_face = following_face
    require(len(labels) == len(mesh["vertices"]) and last_face == len(mesh["faces"]),
            "incomplete Caracara Mk I creator parts")
    return labels


def collision_inputs():
    provenance, inputs, resource, references, bindings = load_source()
    named_slots = {}
    memberships = []
    for mesh, records, minor_samples in zip(references, bindings, (12, 8, 4), strict=True):
        labels = group_labels(mesh, records)
        slots = []
        for part in records:
            start, count = part["vertexStart"], part["vertexCount"]
            name = labels[start]
            if not part["creatorMethod"].startswith("Mesh.torus"):
                slot = named_slots.setdefault(name, len(named_slots))
                slots.extend([(slot,)] * count)
                continue
            closed = part["faceCount"] == 2 * count
            require(count % minor_samples == 0 and part["faceCount"] == 2 * count - (0 if closed else 4),
                    "changed original arc lattice")
            rows = count // minor_samples
            segments = rows if closed else rows - 1
            span_count = 8 if closed else 6
            require(segments >= span_count, "original arc has too few rows")
            spans = [named_slots.setdefault((*name, span), len(named_slots)) for span in range(span_count)]
            for row in range(rows):
                left = (row - 1) % segments if closed else max(0, row - 1)
                right = row if closed else min(row, segments - 1)
                adjacent = tuple(sorted({spans[s * span_count // segments] for s in (left, right)}))
                slots.extend([adjacent] * minor_samples)
        require(len(slots) == len(mesh["vertices"]) and all(
            set.intersection(*(set(slots[index]) for index in face)) for face in mesh["faces"]),
            "original arc face crosses its creator or local support")
        memberships.append(slots)
    names = [f"{name} {ordinal} span {span[0]+1}" if span else (name if ordinal == 1 else f"{name} ({ordinal})")
             for name, ordinal, *span in named_slots]
    require(len(names) == len(set(names)) == 81, "wrong arc support count")
    require(sum(len(row) == 2 for membership in memberships for row in membership) == 384,
            "wrong arc seam vertex count")
    scaled = [dict(mesh, vertices=[[v * float(SCALE) for v in point] for point in mesh["vertices"]])
              for mesh in references]
    require([len(mesh["vertices"]) for mesh in scaled] == [2104, 720, 294], "wrong original arc model counts")
    return provenance,inputs,resource,references,names,list(zip(scaled,memberships,strict=True))


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
    require(finite["massKg"] == json.loads(inputs["engineering.json"])["dryMassKilograms"] == 40500, 'changed original dry mass')
    require(finite["id"] == "finite-intrinsic-empty-caracara-mk-i", 'wrong original dry reference')
    probes = []
    for route in json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes()):
        require(route["clearanceMillimetres"] > PROBE_RADIUS * 1000, 'insufficient original probe clearance')
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(start, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": start, "end": end, "radiusMetres": PROBE_RADIUS})

    return {
        "schema": 1, "id": "SH169", "name": "Caracara Mk I", "metresPerUnit": 1, "roundingMetres": RADIUS,
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
            "sourceId": "SH169", "revision": provenance["revision"],
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
    packed = bytes(value for row in membership for value in (row if len(row) == 2 else (*row, 255)))
    streams = {
        "source.vertices": struct.pack("<8sIII", b"MSH169V1", *counts)
            + b"".join(struct.pack("<3d", *p) for p in vertices),
        "source-triangles.bin": b"".join(struct.pack("<3I", *f) for f in faces),
        "source-piece-membership.bin": packed,
        "PIECES.json": encode_json({"pieces": names, "roundingMetres": RADIUS}),
        "visual.bin": b"SH169VIS" + encode_visual(detail[0])[8:],
        "native-visual.bin": b"SH169VIS" + encode_visual(native[0])[8:],
    }
    fans = json.loads((INTEGRATION / "SUPPORT-FAN.json").read_bytes())
    require(list(fans) == names, "wrong Caracara Mk I fan order")
    require(all(0 <= index < len(membership) and slot in membership[index]
                for slot, faces in enumerate(fans.values()) for face in faces for index in face),
            "Caracara Mk I local support fan spans pieces")
    streams[STUDIO_NAME] = encode_json(studio_definition(provenance, inputs, resource, detail, candidate, streams))
    streams["SOURCE-GEOMETRY.json"] = encode_json({
        "id": "SH169", "name": "Caracara Mk I", "sourceScaleRatio": [SCALE.numerator, SCALE.denominator], "revision": REVISION,
        "references": {"detailed": [2104, 3948, 67], "archivedNative": [720, 1364, 21], "candidateNative": [294, 512, 21]},
        "nativeFaceLimit": 512, "groups": len(names), "roundingMetres": RADIUS,
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
            require(path.read_bytes() == data, f"changed SH169 encoding: {name}")
        else:
            replace_output(path, data)


if __name__ == "__main__":
    main()
