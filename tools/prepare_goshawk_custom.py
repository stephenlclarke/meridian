"""Stage and rebuild only the SH010 original geometry, ownership and visual."""

import argparse
from decimal import Decimal
from fractions import Fraction
import hashlib
import itertools
import json
import math
from pathlib import Path
import struct
import subprocess
import sys
import tomllib

from encode_goshawk_visual import encode_model
from import_goshawk_envelope_source import components, partition_ranges

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets/fleet/sh010"
LEGACY_REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
CONTRACT = "contract.toml"
NATIVE_SOURCE = "native-source.json"
PHYSICAL = "physical.json"
OWNERSHIP = "ownership.json"
PINS = {
    "resource.toml": ("Sources/L337Core/Resources/Ships/SH010.toml", "66f8a18a52854fad92f8c0b1cb76ef3d0b6f7405d6e8e66e944b1b9e63bb89d3"),
    CONTRACT: ("docs/fleet-design-v2/ships/SH010.toml", "755f776227dcb1d7816b5e426fbe239a98f0f9d1df46fc1ebdbd90c5b15e5983"),
    NATIVE_SOURCE: ("Sources/L337Core/Resources/OriginalFleetShowroomModels/SH010.json", "6b2fcf012eaf7cea3bede3f95679c80dd74517f39cffc8a6356ea173070cfced"),
    "native-export-manifest.json": ("Sources/L337Core/Resources/OriginalFleetShowroomModels/manifest.json", "118b05757dc54096faa15dd79801f38999b87fe9424f1b3513133ead94f71971"),
    PHYSICAL: ("Sources/L337Core/Resources/OriginalFleetPhysical/SH010.json", "30aba17c31f65a5d1bcb9a29455f45bb876628ce5906d4af5cf16f249d2a79a6"),
    OWNERSHIP: ("docs/milestones/fleet-physical/creator-ownership-records/SH010.json", "5b3b4ff988a01b91e592fd73b048b2af8712c40b071406402def30b62f312bf9"),
    "LICENSE": ("LICENSE", "033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766"),
}
DESIGN_SHA = "3e131707177a4b934ad969dc62da49963adf9aa6a71985e6dcf21c8ba9dd179c"
SCALE = Fraction(15_000_000, 1_644_589)
COUNTS = ((2104, 3532, 169), (568, 1012, 31))
MAGIC = b"MSH010V1"
VISUAL_MAGIC = b"SH010VZ1"
MANIFEST_SHA = "211b24b5c673607092be3d83c7868a6b423be2c463bb159c35b53c3422a460f7"
TEXTURE_SOURCE_SHA = "96c4c9699b81f50be5a8aefeeca74a6e4870e0bb981d1cd33a9ff2921cd2bd3c"


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def pin(raw):
    return {"bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def json_bytes(value):
    return (json.dumps(value, indent=2, allow_nan=False) + "\n").encode()


def authenticated(inputs):
    for name, (_, expected) in PINS.items():
        require(pin(inputs[name])["sha256"] == expected, f"Changed SH010 source: {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode(), parse_float=Decimal)
    contract = tomllib.loads(inputs[CONTRACT].decode(), parse_float=Decimal)
    export = json.loads(inputs[NATIVE_SOURCE], parse_float=Decimal)
    engineering = json.loads(inputs[PHYSICAL])
    ownership = json.loads(inputs[OWNERSHIP])
    index = json.loads(inputs["native-export-manifest.json"])
    require(all(x["id"] == "SH010" for x in (resource, contract, export, engineering, ownership)), "Wrong SH010 identity")
    require(resource["name"] == contract["name"] == "Goshawk Mk II Custom", "Changed hull name")
    info = contract["source"]
    require(info["geometryFile"] == export["sourcePath"] == "replacement-hulls/SH010-design.toml", "Changed original design path")
    require(info["geometrySHA256"] == export["sourceSHA256"] == engineering["detailedSourceSHA256"] == ownership["geometrySHA256"] == DESIGN_SHA, "Changed original design pin")
    require(engineering["contractSHA256"] == ownership["contractSHA256"] == pin(inputs[CONTRACT])["sha256"], "Changed engineering design binding")
    require(engineering["componentSourceSHA256"] == pin(inputs[OWNERSHIP])["sha256"], "Changed creator component binding")
    require(Fraction(engineering["metresPerSourceUnitExact"]) == SCALE and engineering["sourceOriginUnchanged"], "Changed adopted calibration")
    require(resource["nativeDesign"] == export["nativeDesign"], "Original native display and resource disagree")
    expected = {"id": "SH010", "file": "SH010.json", "sourcePath": info["geometryFile"], "sourceSHA256": DESIGN_SHA, **pin(inputs[NATIVE_SOURCE])}
    require([r for r in index["models"] if r["id"] == "SH010"] == [expected], "SH010 native exporter does not bind the original")
    require(contract["geometry"]["nativeFaceLimit"] == 512, "Changed native qualification limit")
    require(contract["performance"]["maxSpeed"] == Decimal("40.0"), "Changed SH010 source speed")
    return resource, ownership, engineering


def custom_partition():
    detailed, native, names = partition_ranges()
    for side, d, n in (("left", 2072, 536), ("right", 2088, 552)):
        for suffix, offset in (("upright", 0), ("longitudinal-member", 8)):
            piece = len(names)
            names.append(f"{side}-lower-mount-{suffix}")
            for table, start in ((detailed, d + offset), (native, n + offset)):
                require(all(i not in table for i in range(start, start + 8)), "Overlapping mount membership")
                table.update((i, piece) for i in range(start, start + 8))
    require(len(names) == 73, "Wrong number of local supports")
    return detailed, native, names


def stage_variant(model, owner, expected, pieces):
    vertex_count, face_count, component_count = expected
    vertices, faces = model["vertices"], model["faces"]
    require((len(vertices), len(faces), len(owner["parts"])) == expected, "Changed source topology")
    require(set(pieces) == set(range(vertex_count)), "Unassigned source vertices")
    require(model["paletteName"] == "forest-silver" and model["license"] == model["appearanceLicense"] == "MIT", "Changed original appearance")
    calibrated = [tuple(float(Fraction(x) * SCALE) for x in point) for point in vertices]
    require(all(len(p) == 3 and all(math.isfinite(x) for x in p) for p in calibrated), "Invalid calibrated point")
    require(all(len(f) == 3 and len(set(f)) == 3 and all(type(i) is int and 0 <= i < vertex_count for i in f) for f in faces), "Invalid face")
    require(all(len({pieces[i] for i in f}) == 1 for f in faces), "A triangle crosses local supports")
    groups = components(model)
    require(len(groups) == component_count, "Changed original connected components")
    cursor = 0
    records = []
    for index, (group, original) in enumerate(zip(groups, owner["parts"], strict=True)):
        start, count = original["vertexStart"], original["vertexCount"]
        first_face, face_len = original["faceStart"], original["faceCount"]
        require(original["index"] == index and group == list(range(start, start + count)), "Creator vertex partition mismatch")
        require(first_face == cursor and all(start <= v < start + count for f in faces[first_face:first_face + face_len] for v in f), "Creator face partition mismatch")
        cursor += face_len
        require(len({pieces[i] for i in group}) == 1, "Creator component split")
        actual_runs = [{"material": key, "faces": sum(1 for _ in run)} for key, run in itertools.groupby(model["faceMaterials"][first_face:cursor])]
        require(actual_runs == original["exactMaterialRuns"], "Creator material run mismatch")
        records.append({"component": index, "name": original["name"], "startVertex": start, "vertexCount": count,
                        "startFace": first_face, "faceCount": face_len, "piece": pieces[start]})
    require(cursor == face_count, "Unassigned source faces")
    return calibrated, faces, records


def build(inputs):
    resource, owner, engineering = authenticated(inputs)
    detail_pieces, native_pieces, names = custom_partition()
    detailed, native = (stage_variant(resource[key], owner["models"][key], expected, table)
                        for key, expected, table in (
                            ("design", COUNTS[0], detail_pieces),
                            ("nativeDesign", COUNTS[1], native_pieces),
                        ))
    vertices = struct.pack("<8sII", MAGIC, COUNTS[0][0], COUNTS[1][0])
    faces = bytearray()
    membership = bytearray()
    for (points, triangles, _), table, offset in ((detailed, detail_pieces, 0), (native, native_pieces, COUNTS[0][0])):
        vertices += b"".join(struct.pack("<3d", *v) for v in points)
        faces.extend(b"".join(struct.pack("<3I", *(i + offset for i in f)) for f in triangles))
        membership.extend(table[i] for i in range(len(points)))
    calibrated_model = dict(resource["design"], vertices=detailed[0])
    encoded = {
        "source.vertices": vertices,
        "source-triangles.bin": bytes(faces),
        "source-piece-membership.bin": bytes(membership),
        "COMPONENT-PIECES.json": json_bytes({"pieces": names, "detailed": detailed[2], "nativeCandidate": native[2]}),
        "visual.bin": encode_model(calibrated_model, vertices, faces, MAGIC, (COUNTS[0][0], COUNTS[1][0], COUNTS[0][1]), VISUAL_MAGIC),
    }
    bounds = [[f(axis) for axis in zip(*v)] for v in (detailed[0], native[0]) for f in (min, max)]
    require(abs(bounds[1][2] - bounds[0][2] - 30) < 1e-12, "Changed engineering length")
    texture_manifest = (ROOT / "assets/fleet/sh001/TEXTURE-SOURCE.json").read_bytes()
    require(pin(texture_manifest)["sha256"] == TEXTURE_SOURCE_SHA, "Changed shared original texture manifest")
    texture_by_id = {asset["id"]: asset for asset in json.loads(texture_manifest)["assets"]}
    shared_textures = {}
    for texture in resource["design"]["textures"]:
        record = texture_by_id[texture["asset"]]
        name = record["rgbaFile"]
        path = f"assets/fleet/sh001/{name}"
        reference = {"bytes": record["width"]*record["height"]*4, "sha256": record["rgbaSHA256"]}
        require(pin((ROOT / path).read_bytes()) == reference, f"Changed original texture: {path}")
        shared_textures[path] = {**reference, "source": f"Sources/L337Core/Resources/OriginalShipTextures/{name}", "license": record["license"]}
    manifest = {
        "id": "SH010", "legacyRevision": LEGACY_REVISION,
        "originalDesignSHA256": DESIGN_SHA,
        "metresPerSourceUnit": str(SCALE), "axes": "+Z forward, +Y up; original origin",
        "detailed": {"vertices": COUNTS[0][0], "faces": COUNTS[0][1], "components": COUNTS[0][2], "boundsMetres": bounds[:2]},
        "nativeCandidate": {"vertices": COUNTS[1][0], "faces": COUNTS[1][1], "components": COUNTS[1][2], "boundsMetres": bounds[2:], "exactFaceLimit": 512},
        "physicalAuthority": {"status": engineering["status"], "dryMassKg": engineering["dryMassKilograms"], "sha256": pin(inputs[PHYSICAL])["sha256"]},
        "collision": {"pieces": len(names), "roundingMetres": 0.02,
                      "scope": "Convex local supports enclose both source meshes. Service wells are bridged locally; the central forebody channel is tested separately. Native qualification and ordinary placement pending."},
        "legacyInputs": {n: {"path": p, **pin(inputs[n])} for n, (p, _) in PINS.items()},
        "encoded": {name: pin(data) for name, data in encoded.items()},
        "sharedTextures": shared_textures,
    }
    encoded["MANIFEST.json"] = json_bytes(manifest)
    require(pin(encoded["MANIFEST.json"])["sha256"] == MANIFEST_SHA, "Changed approved SH010 manifest encoding")
    return encoded


def main():
    require(sys.flags.optimize == 0, "SH010 preparation requires assertions in the original partition helper")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--legacy-repo", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.check:
        inputs = {name: (ASSETS / name).read_bytes() for name in PINS}
    else:
        if args.legacy_repo is None:
            parser.error("--legacy-repo is required to stage the immutable originals")
        repo = args.legacy_repo.resolve(strict=True)
        inputs = {
            name: subprocess.check_output(
                ["git", "cat-file", "blob", f"{LEGACY_REVISION}:{path}"], cwd=repo
            )
            for name, (path, _) in PINS.items()
        }
    outputs = {**inputs, **build(inputs)}
    if args.check:
        for name, blob in outputs.items():
            require((ASSETS / name).read_bytes() == blob, f"Changed SH010 encoded asset: {name}")
    else:
        ASSETS.mkdir(parents=True, exist_ok=True)
        for name, blob in outputs.items():
            (ASSETS / name).write_bytes(blob)
    print(f"SH010 original detailed/native geometry, creator ownership and 73 local supports verified ({len(outputs)} assets)")


if __name__ == "__main__":
    main()
