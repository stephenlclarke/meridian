"""Replay Rabbit Explorer's two original visuals and its separate inspection artwork."""

import argparse
import hashlib
import json
import math
import struct

from atomic_asset_output import checked_directory, checked_file, replace_output
from prepare_eagle_cruiser_geometry import encode_visual
from rabbit_explorer_art_mesh import INTEGRATION, ORIGINAL, SCALE, collision_groups, collision_membership, load, metre_space, require

STUDIO_NAME = "RABBIT-EXPLORER-STUDIO.json"
STUDIO_PROBES = ("dorsal-exterior", "ventral-exterior", "port-outer-arc", "starboard-outer-arc")


def encode(value):
    return (json.dumps(value, allow_nan=False, separators=(",", ":")) + "\n").encode()


def pin(data):
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def visual(model):
    return b"RABBVIZ1" + encode_visual(metre_space(model))[8:]


def build():
    inputs, resource, engineering, source, candidate = load()
    detailed = source["detailed"]
    artwork, (detail_mesh, native_mesh), groups = collision_groups(detailed, source["candidateNative"], candidate=True)
    points = detail_mesh["vertices"] + native_mesh["vertices"]
    split = len(detail_mesh["vertices"])
    faces = detail_mesh["triangles"] + [[index + split for index in face] for face in native_mesh["triangles"]]
    require(len(artwork) == 24 and len(groups) == 35, "changed Rabbit Explorer group counts")
    outputs = {
        "source.vertices": struct.pack("<8sII", b"MSH066V1", split, len(points) - split) + b"".join(
            struct.pack("<3d", *(coordinate * float(SCALE) for coordinate in point)) for point in points),
        "source-triangles.bin": b"".join(struct.pack("<3I", *face) for face in faces),
        "source-piece-membership.bin": bytes(detail_mesh["groups"] + native_mesh["groups"]),
        "collision-piece-membership.bin": collision_membership(len(points), groups),
        "PIECES.json": encode({"pieces": list(groups), "roundingMetres": .02}),
        "visual.bin": visual(detailed[0]),
        "archive-native-visual.bin": visual(source["archiveNative"][0]),
        "candidate-native-visual.bin": visual(source["candidateNative"][0]),
    }
    routes = json.loads((INTEGRATION / "CANDIDATE-ROUTES.json").read_bytes())["routes"]
    probes = []
    for route in routes:
        if route["name"] in STUDIO_PROBES:
            require(route["clearanceMillimetres"] > 100, "changed Rabbit Explorer artwork probe guard")
            start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
            end = [a - b for a, b in zip(start, route["travel"], strict=True)]
            probes.append({"name": route["name"], "start": start, "end": end, "radiusMetres": .1})
    require(len(probes) == len(STUDIO_PROBES), "changed Rabbit Explorer artwork probe count")
    detail = detailed[0]
    inertia = engineering["inertiaAboutCentreOfMassKgM2"]
    outputs[STUDIO_NAME] = encode({
        "schema": 1, "id": "SH066", "name": "Rabbit Explorer", "metresPerUnit": float(SCALE), "roundingMetres": .02,
        "detail": detail_mesh, "native": native_mesh,
        "materials": [
            {"name": material["name"], "rgb": [material["colour"] >> shift & 255 for shift in (24, 16, 8)],
             "alpha": material["colour"] & 255, "emissive": material["emissive"]}
            for material in detail["materials"]
        ],
        "faceMaterials": detail["faceMaterials"],
        "faceTextures": [
            {"uv": row["uv"], "texture": detail["textures"][row["texture"]]["asset"] if row["texture"] >= 0 else None}
            for row in detail["faceTextures"]
        ],
        "engineering": {"massKg": engineering["massKg"], "centerOfMass": engineering["centreMetres"],
                        "inertia": [inertia[i:i+3] for i in (0, 3, 6)]},
        "flight": {"maxSpeed": resource["game"]["speed"]},
        "clearances": probes,
        "provenance": {
            "sourceId": "SH066", "revision": "a11fe16924d87d06021816f9037952afcbac08db",
            "frozenResource": pin(inputs["resource.toml"]),
            "frozenNativeArchive": {"artifact": "native-export.json", "faces": 888, **pin(inputs["native-export.json"])},
            "generatedInspectionCandidate": {
                "artifact": "SH066-candidate.json", "faces": 480, "faceLimit": 512,
                "recipeOrigin": "frozen creator recipe", **pin(candidate),
            },
            "derivedHashes": {name: pin(value) for name, value in outputs.items()},
            "originalScaleRatio": [SCALE.numerator, SCALE.denominator],
            "engineeringReference": engineering["id"], "scope": "offline catalogue art inspection",
        },
    })
    outputs["SOURCE-GEOMETRY.json"] = encode({
        "id": "SH066", "originalScaleRatio": [SCALE.numerator, SCALE.denominator],
        "archivedDetailFaces": 2648, "archivedNativeFaces": 888,
        "candidateNativeFaces": 480, "candidateNativeFaceLimit": 512,
        "artworkGroups": len(artwork), "collisionGroups": len(groups),
        "outputs": {name: pin(value) for name, value in outputs.items()},
    })
    return outputs


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    checked_directory(ORIGINAL)
    checked_directory(INTEGRATION)
    for name, value in build().items():
        path = checked_file((INTEGRATION if name == STUDIO_NAME else ORIGINAL) / name)
        if args.check:
            require(path.read_bytes() == value, f"changed Rabbit Explorer output: {name}")
        else:
            replace_output(path, value)


if __name__ == "__main__":
    main()
