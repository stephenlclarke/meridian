"""Rebuild the original Medusa inspection assets from authenticated source."""

import argparse
from gzip import decompress
from hashlib import sha256
import json
from pathlib import Path
import struct
import tomllib

import numpy as np
from scipy.spatial import ConvexHull

from atomic_asset_output import replace_output
from prepare_eagle_cruiser_geometry import encode_visual

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets/fleet/sh171"
EVIDENCE = ROOT / "integration/sh171-medusa"
SCALE = 5 / 41
ROUNDING = 0.02
COUNTS = ((82436, 162548), (39276, 76228))
MESH_NAMES = ("design", "nativeDesign")
CROWN = 63
CROWN_SECTORS = 16
CURVE_SPANS = 4
RECEIPT_SHA = "70e16aef81c70db0815c969143d60e9f57375251ed6e24515a44732524f87956"
SHIELD_SHA = "25f099509deb0b8e40bc2dafee0058920cb17f074283acf4324e884165da241c"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def load_originals():
    frozen = (EVIDENCE / "FROZEN-SOURCES.json").read_bytes()
    require(sha256(frozen).hexdigest() == RECEIPT_SHA, "Changed Medusa frozen source receipt")
    receipt = json.loads(frozen)
    require(receipt["revision"] == "a11fe16924d87d06021816f9037952afcbac08db"
            and receipt["tree"] == "68fec574df12ddf74acb14747e0dd6839f478642", "Changed archive identity")
    payloads = {}
    for name, path in (
        ("resource.toml", ASSETS / "resource.toml.gz"),
        ("native-export.json", ASSETS / "native-export.json.gz"),
        ("engineering.json", ASSETS / "engineering.json"),
        ("contract.toml", ASSETS / "contract.toml"),
        ("original-ownership.json", EVIDENCE / "original-ownership.json"),
        ("LICENSE", ASSETS / "LICENSE"),
    ):
        raw = path.read_bytes()
        payload = decompress(raw) if path.suffix == ".gz" else raw
        pin = receipt["paths"][name]
        require((len(payload), sha256(payload).hexdigest()) == (pin["bytes"], pin["sha256"]),
                f"Changed archived Medusa source {name}")
        payloads[name] = payload
    resource = tomllib.loads(payloads["resource.toml"].decode())
    archive = json.loads(payloads["native-export.json"])
    engineering = json.loads(payloads["engineering.json"])
    contract = tomllib.loads(payloads["contract.toml"].decode())
    owners = json.loads(payloads["original-ownership.json"])
    require(resource["id"] == archive["id"] == engineering["id"] == owners["id"] == contract["id"] == "SH171",
            "Changed Medusa identity")
    require(resource["nativeDesign"] == archive["nativeDesign"], "Changed original native artwork")
    require(archive["sourceSHA256"] == engineering["detailedSourceSHA256"] == owners["geometrySHA256"],
            "Changed Medusa archive geometry binding")
    require(engineering["componentSourceSHA256"] == receipt["paths"]["original-ownership.json"]["sha256"]
            and engineering["contractSHA256"] == receipt["paths"]["contract.toml"]["sha256"],
            "Changed Medusa engineering source binding")
    require(engineering["metresPerSourceUnitExact"] == "5/41" and engineering["metresPerSourceUnit"] == SCALE,
            "Changed Medusa scale")
    require(contract["geometry"]["nativeFaceLimit"] == 512
            and not resource["catalogue"]["playerFlyable"]
            and not resource["catalogue"]["normalPurchaseEligible"], "Changed Medusa flight or purchase policy")
    for key, counts in zip(MESH_NAMES, COUNTS, strict=True):
        model = resource[key]
        require((len(model["vertices"]), len(model["faces"])) == counts
                and model["license"] == "MIT" and not model["textures"]
                and model["paletteName"] == "pearl-teal-organic", f"Changed archived {key} artwork")
        require(model["materials"] == resource["design"]["materials"] and len(model["materials"]) == 7,
                f"Changed archived {key} materials")
        require(all(t["texture"] == -1 for t in model["faceTextures"]), f"Changed archived {key} texture assignments")
    return resource, owners["models"], engineering, receipt


def intervals(part, native):
    """Return source ring ownership. Boundary rings belong to both adjacent supports."""
    count = part["vertexCount"]
    if part["index"] == CROWN:
        rows, sides = (144, 16) if native else (256, 24)
        require(count == rows * sides, "Changed original living crown ring layout")
        for step in range(CROWN_SECTORS):
            start = step * rows // CROWN_SECTORS
            end = (step + 1) * rows // CROWN_SECTORS
            yield [(ring % rows) * sides + side for ring in range(start, end + 1) for side in range(sides)]
        return

    if part["name"].startswith(("central flowing surface fold", "propulsion organ connected stem",
                                 "propulsion organ breathing fold", "curved seed propulsion organ")):
        rows = 20 if native else 36
        sides, remainder = divmod(count, rows)
        require(not remainder, "Changed curved Medusa support ring layout")
        for step in range(CURVE_SPANS):
            start = step * (rows - 1) // CURVE_SPANS
            end = (step + 1) * (rows - 1) // CURVE_SPANS
            yield list(range(start * sides, (end + 1) * sides))
        return

    yield list(range(count))


def ownership(model, parts, native):
    vertices, faces = len(model["vertices"]), len(model["faces"])
    groups = []
    face_groups = []
    vertex_at = face_at = 0
    for index, part in enumerate(parts):
        require((part["index"], part["vertexStart"], part["faceStart"]) == (index, vertex_at, face_at),
                "Changed Medusa part order")
        part_faces = model["faces"][face_at:face_at + part["faceCount"]]
        part_materials = model["faceMaterials"][face_at:face_at + part["faceCount"]]
        require(set(part_materials) == set(part["allowedSourceMaterials"]), "Changed Medusa part material")
        memberships = [set() for _ in range(part["vertexCount"])]
        intervals_for_part = list(intervals(part, native))
        for segment, indices in enumerate(intervals_for_part):
            name = part["name"]
            if len(intervals_for_part) > 1:
                name += f"/span-{segment + 1:02d}"
            for point in indices:
                memberships[point].add(len(groups))
            groups.append((name, [vertex_at + point for point in indices]))
        for face in part_faces:
            local = [point - vertex_at for point in face]
            require(all(0 <= point < len(memberships) for point in local), "Medusa face crosses original owners")
            shared = set.intersection(*(memberships[point] for point in local))
            require(bool(shared), "Medusa face crosses support boundary")
            face_groups.append(min(shared))
        vertex_at += part["vertexCount"]
        face_at += part["faceCount"]
    require((vertex_at, face_at, len(face_groups)) == (vertices, faces, faces),
            "Incomplete Medusa face and point ownership")
    return groups, face_groups


def ordered_fan(points, indices):
    """Use source points only and orient every convex support facet outwards."""
    unique, first = np.unique(points[indices], axis=0, return_index=True)
    hull = ConvexHull(unique)
    center = unique[hull.vertices].mean(axis=0)
    mapped = np.asarray(indices)[first]
    faces = []
    for a, b, c in hull.simplices.tolist():
        normal = np.cross(unique[b] - unique[a], unique[c] - unique[a])
        if np.dot(normal, unique[a] - center) < 0:
            b, c = c, b
        face = [int(mapped[j]) for j in (a, b, c)]
        start = face.index(min(face))
        faces.append(face[start:] + face[:start])
    return faces


def shield_inputs():
    resource, parts, engineering, receipt = load_originals()
    detailed, native = (resource[key] for key in MESH_NAMES)
    owners = [ownership(resource[key], parts[key]["parts"], key == "nativeDesign") for key in MESH_NAMES]
    points = np.array(detailed["vertices"] + native["vertices"], dtype=np.float64) * SCALE
    first_native = len(detailed["vertices"])
    group_points = []
    for (name, high), (other, low) in zip(owners[0][0], owners[1][0], strict=True):
        require(name == other, "Medusa supports differ across archived LODs")
        indices = high + [index + first_native for index in low]
        require(len(indices) <= 5344, f"Medusa support {name} exceeds the point limit")
        group_points.append((name, indices))
    return resource, parts, engineering, receipt, owners, points, group_points


def source_fans(points, groups, author):
    if author:
        shield = bytearray(struct.pack("<8sI", b"MDUSHLD1", len(groups)))
        for _, indices in groups:
            faces = ordered_fan(points, indices)
            shield.extend(struct.pack("<I", len(faces)))
            for face in faces:
                shield.extend(struct.pack("<3I", *face))
        return bytes(shield)
    shield = (ASSETS / "shield-fans.bin").read_bytes()
    require(sha256(shield).hexdigest() == SHIELD_SHA, "Changed MacBook Air Medusa shield triangulation")
    return shield


def read_fans(shield, group_points):
    require(shield[:12] == struct.pack("<8sI", b"MDUSHLD1", len(group_points)),
            "Changed Medusa shield format or group count")
    start = 12
    for name, indices in group_points:
        count, = struct.unpack_from("<I", shield, start)
        first = start + 4
        start = first + count * 12
        require(4 <= count <= 2 * len(indices) - 4 and start <= len(shield),
                f"Invalid Medusa shield facet count for {name}")
        faces = list(struct.iter_unpack("<3I", shield[first:start]))
        membership = set(indices)
        require(all(len(set(face)) == 3 and membership.issuperset(face) for face in faces),
                f"Medusa shield face crosses support {name}")
        yield name, indices, faces
    require(start == len(shield), "Trailing Medusa shield facets")


def prepare(author=False):
    resource, _, _, _, _, points, groups = shield_inputs()
    detailed = resource["design"]
    first_native = len(detailed["vertices"])
    shield = source_fans(points, groups, author)
    list(read_fans(shield, groups))
    metre_model = dict(detailed, vertices=points[:first_native].tolist())
    visual = b"MEDUVIZ1" + encode_visual(metre_model)[8:]
    point_stream = struct.pack("<8sII", b"MDU171V1", first_native, len(resource["nativeDesign"]["vertices"]))
    point_stream += np.asarray(points, dtype="<f8").tobytes()
    return {"visual.bin": visual, "source.vertices": point_stream, "shield-fans.bin": shield}


def studio_mesh(model, face_groups):
    """Duplicate a boundary point only when Studio needs a distinct group ID."""
    vertices, triangles, groups = [], [], []
    lookup = {}
    for source_face, group in zip(model["faces"], face_groups, strict=True):
        triangle = []
        for source in source_face:
            key = (source, group)
            if key not in lookup:
                lookup[key] = len(vertices)
                vertices.append(model["vertices"][source])
                groups.append(group)
            triangle.append(lookup[key])
        triangles.append(triangle)
    require(len(vertices) <= 100_000, "Medusa inspection exceeds Studio's mesh point limit")
    return {"vertices": vertices, "triangles": triangles, "groups": groups}


def studio_definition():
    resource, _, _, receipt, owners, _, _ = shield_inputs()
    detail, native = (resource[key] for key in MESH_NAMES)
    return {
        "schema": 2,
        "id": "SH171",
        "name": "Medusa",
        "metresPerUnit": SCALE,
        "roundingMetres": ROUNDING,
        "detail": studio_mesh(detail, owners[0][1]),
        "native": studio_mesh(native, owners[1][1]),
        "materials": [
            {"name": material["name"], "rgb": [material["colour"] >> shift & 255 for shift in (24, 16, 8)],
             "alpha": material["colour"] & 255, "emissive": material["emissive"]}
            for material in detail["materials"]
        ],
        "faceMaterials": detail["faceMaterials"],
        "nativeFaceMaterials": native["faceMaterials"],
        "faceTextures": [{"uv": face["uv"], "texture": None} for face in detail["faceTextures"]],
        "engineering": None,
        "flight": {},
        "clearances": [],
        "provenance": {
            "scope": "archived mesh inspection",
            "sourceId": "SH171",
            "revision": receipt["revision"],
            "sourceHashes": {name: pin["sha256"] for name, pin in receipt["paths"].items()},
            "nativeFaceCount": len(native["faces"]),
            "flightNativeFaceLimit": 512,
            "license": (ASSETS / "LICENSE").read_text(),
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--author-shield", action="store_true", help="derive a new fan from the original points")
    parser.add_argument("--studio-definition", type=Path, help="write a new read-only Studio definition")
    args = parser.parse_args()
    if args.studio_definition:
        definition = (json.dumps(studio_definition(), separators=(",", ":"), allow_nan=False) + "\n").encode()
        require(len(definition) <= 64 * 1024 * 1024, "Medusa definition exceeds the Studio input limit")
        with args.studio_definition.open("xb") as output:
            output.write(definition)
        print(f"{args.studio_definition} {len(definition)} {sha256(definition).hexdigest()}")
        return
    if args.check and args.author_shield:
        parser.error("--check uses the pinned fan")
    outputs = prepare(args.author_shield)
    for name, payload in outputs.items():
        path = ASSETS / name
        if args.check:
            require(path.read_bytes() == payload, f"Medusa asset differs from its checked source: {name}")
        else:
            replace_output(path, payload)
        print(f"{name} {len(payload)} {sha256(payload).hexdigest()}")


if __name__ == "__main__":
    main()
