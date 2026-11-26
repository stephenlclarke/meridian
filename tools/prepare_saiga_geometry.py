"""Encode exact original Saiga artwork and the measured candidate for Hull Studio."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import tomllib

from atomic_asset_output import checked_directory, checked_file, replace_output
from prepare_eagle_cruiser_geometry import encode_visual
from saiga_art_mesh import SCALE, assemble, collision_groups, collision_membership

ROOT = Path(__file__).resolve().parents[1]
INTEGRATION = ROOT / "integration/sh061-saiga"
ORIGINAL = ROOT / "assets/fleet/sh061"
STUDIO_NAME = "SAIGA-STUDIO.json"
FROZEN = {
    'resource.toml': 'f3177e424313874098556204aa0be6f62096ee0412bcac9175bcc6a76db9f71d',
    'native-export.json': 'ac825d17ed17bfdec737597211647411b2e96ecd0bbf9788e8b449e0aaf2d312',
    'contract.toml': 'a5b9e65f897bebac9f94ab7ba88c5f38908bf13da7c85c5c3eff5b74ed96a4a4',
    'engineering.json': '7e94b9a638bcc373da033d68c322706ed0261abf2f10b7544e3bf601c8683ff0',
    'decimal80-record.json': '7e94b9a638bcc373da033d68c322706ed0261abf2f10b7544e3bf601c8683ff0',
    'ORIGINAL-OWNERSHIP.json': '00205460ab315a511c3040f992bacbfb5f4b81bec3b97b0279dcb2902c81011d',
    'SOURCE-COMPONENTS-MAP.json': '14597def726535eaba7de1dfe1d34ac922da23f907b582b945cfa45f59fcd61c',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
    'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
    'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
    'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d',
}
INTEGRATION_SOURCES = frozenset({"ORIGINAL-OWNERSHIP.json", "SOURCE-COMPONENTS-MAP.json", "decimal80-record.json"})
CANDIDATE_SHA = "bbf4c74a40f00781be3444a7a6d47c2b8421e3e3256565ae481ad8a155039dd7"

def encode(value):
    return (json.dumps(value, allow_nan=False, separators=(",", ":")) + "\n").encode()


def pin(data):
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def stream_visual(relative):
    model = dict(relative, vertices=[[coordinate * float(SCALE) for coordinate in point] for point in relative["vertices"]])
    return b"SAIGVIZ1" + encode_visual(model)[8:]


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def load_frozen():
    sources = {}
    for name, digest in FROZEN.items():
        folder = INTEGRATION if name in INTEGRATION_SOURCES else ORIGINAL
        if name.endswith(".rgba"):
            folder = ROOT / "assets/fleet/sh001"
        artifact = (folder / name).read_bytes()
        require(pin(artifact)["sha256"] == digest, f"changed frozen Saiga source: {name}")
        sources[name] = artifact
    return sources


def build():
    originals = load_frozen()
    raw_source = originals["resource.toml"]
    raw_archive = originals["native-export.json"]
    raw_engineering = originals["engineering.json"]
    require(raw_engineering == originals["decimal80-record.json"], "changed archived Saiga engineering record")
    raw_candidate = (ORIGINAL / "SH061-project-candidate.json").read_bytes()
    source, native_archive = tomllib.loads(raw_source.decode()), json.loads(raw_archive)
    engineering, candidate = json.loads(raw_engineering), json.loads(raw_candidate)
    require(pin(raw_candidate)["sha256"] == CANDIDATE_SHA, "changed Saiga project candidate")
    require(source["nativeDesign"] == native_archive["nativeDesign"], "changed Saiga native archive")
    require(native_archive["sourceSHA256"] == candidate["originalSourceSHA256"], "obsolete Saiga candidate provenance")
    require(engineering["metresPerSourceUnit"] == float(SCALE) and engineering["metresPerSourceUnitExact"] == "20",
            "changed original Saiga scale")
    require(engineering["id"] == "SH061" and engineering["dryMassKilograms"] == 284291,
            "changed Saiga archived dry estimate")
    owners = json.loads(originals["ORIGINAL-OWNERSHIP.json"])["models"]
    detailed, native = (source["design"], owners["design"]["parts"]), (candidate["nativeCandidate"], candidate["parts"])
    artwork_slots, meshes = assemble(detailed, native)
    groups = collision_groups(detailed, native, artwork_slots, meshes)
    points = meshes[0]["vertices"] + meshes[1]["vertices"]
    split = len(meshes[0]["vertices"])
    require((len(artwork_slots), split, len(points)-split) == (58, 3216, 296), "changed Saiga source vertex counts")
    faces = meshes[0]["triangles"] + [[i + split for i in face] for face in meshes[1]["triangles"]]
    require(len(faces) == 5776 + 512, "changed Saiga detail or candidate face count")
    sector_membership = collision_membership(len(points), groups)
    outputs = {
        "source.vertices": struct.pack("<8sII", b"MSH061V1", split, len(points)-split) + b"".join(
            struct.pack("<3d", *(coordinate * float(SCALE) for coordinate in point)) for point in points),
        "source-triangles.bin": b"".join(struct.pack("<3I", *face) for face in faces),
        "source-piece-membership.bin": bytes(meshes[0]["groups"] + meshes[1]["groups"]),
        "collision-piece-membership.bin": sector_membership,
        "PIECES.json": encode({"pieces": list(groups), "roundingMetres": .02}),
        "visual.bin": stream_visual(source["design"]),
        "archive-native-visual.bin": stream_visual(source["nativeDesign"]),
        "candidate-native-visual.bin": stream_visual(candidate["nativeCandidate"]),
    }
    readiness = json.loads((INTEGRATION / "candidate-sector-support.json").read_bytes())
    probes = []
    for route in readiness["routes"]:
        if route["name"] not in ("port-outboard-upper", "starboard-outboard-upper", "ventral-outside-collar"):
            continue
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a - b for a, b in zip(start, route["travel"], strict=True)]
        probes.append({"name": route["name"], "start": start, "end": end, "radiusMetres": .1})
    require(len(probes) == 3, "wrong Saiga artwork probe count")
    detail_mesh = source["design"]
    inertia = engineering["inertiaAboutCentreOfMassKgM2"]
    outputs[STUDIO_NAME] = encode({
        "schema": 1, "id": "SH061", "name": "Saiga", "metresPerUnit": float(SCALE),
        "roundingMetres": .02, "detail": meshes[0], "native": meshes[1],
        "materials": [
            {"name": material["name"], "rgb": [material["colour"] >> shift & 255 for shift in (24, 16, 8)],
             "alpha": material["colour"] & 255, "emissive": material["emissive"]}
            for material in detail_mesh["materials"]
        ],
        "faceMaterials": detail_mesh["faceMaterials"],
        "faceTextures": [
            {"uv": mapping["uv"], "texture": detail_mesh["textures"][mapping["texture"]]["asset"]
             if mapping["texture"] >= 0 else None}
            for mapping in detail_mesh["faceTextures"]
        ],
        "engineering": {"massKg": engineering["dryMassKilograms"], "centerOfMass": engineering["centreOfMassMetres"],
                        "inertia": [inertia[i:i+3] for i in (0, 3, 6)]},
        "flight": {"maxSpeed": source["game"]["speed"]},
        "clearances": probes,
        "provenance": {
            "sourceId": "SH061", "revision": "a11fe16924d87d06021816f9037952afcbac08db",
            "frozenResource": pin(raw_source),
            "frozenNativeArchive": {"artifact": "native-export.json", "faces": 1720, **pin(raw_archive)},
            "separateNativeCandidate": {"artifact": "SH061-project-candidate.json", "faces": 512,
                                        "faceLimit": 512, "recipeOrigin": "project-authored", **pin(raw_candidate)},
            "derivedHashes": {name: pin(data) for name, data in outputs.items()},
            "originalScaleRatio": [20, 1],
            "engineeringReference": {"id": engineering["id"], "archive": "engineering.json", **pin(raw_engineering)},
            "scope": "offline catalogue art inspection",
        },
    })
    outputs["SOURCE-GEOMETRY.json"] = encode({
        "id": "SH061", "originalScaleRatio": [20, 1],
        "archivedDetailFaces": 5776, "archivedNativeFaces": 1720,
        "candidateNativeFaces": 512, "candidateNativeFaceLimit": 512,
        "artworkGroups": len(artwork_slots), "collisionGroups": len(groups),
        "outputs": {name: pin(data) for name, data in outputs.items()},
    })
    return outputs


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    checked_directory(ORIGINAL)
    checked_directory(INTEGRATION)
    for name, content in build().items():
        path = checked_file((INTEGRATION if name == STUDIO_NAME else ORIGINAL) / name)
        if args.check:
            require(path.read_bytes() == content, f"changed Saiga output: {name}")
        else:
            replace_output(path, content)


if __name__ == "__main__":
    main()
