"""Encode the two visible archives and the separately sourced collision inspection."""
from fractions import Fraction
from pathlib import Path
import argparse
import hashlib
import json
import math
import struct
import tomllib

from atomic_asset_output import checked_directory, checked_file, replace_output
from banteng_art_mesh import assemble
from prepare_eagle_cruiser_geometry import encode_visual

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / "assets/fleet/sh073"
INTEGRATION = ROOT / "integration/sh073-griffon-spectre"
STUDIO_NAME = "GRIFFON-SPECTRE-STUDIO.json"
FROZEN = {
    'resource.toml': 'd386ff366bf2ea46931b4f6ed503f46b80d2457718dff64729465df9bac4b62f',
    'native-export.json': '0ceb33774f174b5cdd0b59a8b55c8f6e5c30edf94edca46937a3427bcd715a15',
    'contract.toml': '0b64d1c988e9863bf08810fd36047b50626182795da99f0e2df8f5b48ad85230',
    'engineering.json': '023d8539a45e25169613eb7899d98d8d34264c81c3b75d9ae6c6fce5037acdbb',
    'finite-dry-source.json': '02741837fc1fd18307bafabac5c85c8527d65ad1df7c58b41db4581e9064af8f',
    'LEGACY-FINITE-DRY.swift.txt': '8641038732fbdab625ca02b841c0d2f46505a3b269d232b4d70cd6c71a850bfe',
    'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
    'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
    'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
    'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d',
    'ORIGINAL-OWNERSHIP.json': '6bbd09e0121001ced743103120abbacba71cfb144b4277ba523551056fbe7328',
    'GENERATED-COLLISION.json': '5e89c788ea30e82189d2ec49854f1b1599e351ddaaa5f5f17e8bfe0f54cc0a4c',
}

SCALE = Fraction(1200000, 132799)


def encode(value):
    return (json.dumps(value, allow_nan=False, separators=(",", ":")) + "\n").encode()


def pin(value):
    return {"bytes": len(value), "sha256": hashlib.sha256(value).hexdigest()}


def metre_space(source):
    return dict(source, vertices=[[coordinate * float(SCALE) for coordinate in point] for point in source["vertices"]])


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def load_frozen():
    files = {}
    for name, expected in FROZEN.items():
        folder = INTEGRATION if name.startswith("LEGACY-") or name in ("CREATOR-BLUEPRINT.json", "ORIGINAL-OWNERSHIP.json") else ORIGINAL
        if name.endswith(".rgba"):
            folder = ROOT / "assets/fleet/sh001"
        contents = (folder / name).read_bytes()
        require(pin(contents)["sha256"] == expected, f"changed Griffon Spectre source: {name}")
        files[name] = contents
    return files


def build():
    originals = load_frozen()
    raw_source = originals["resource.toml"]
    raw_visible_native = originals["native-export.json"]
    raw_collision = originals["GENERATED-COLLISION.json"]
    source, visible_native = tomllib.loads(raw_source.decode()), json.loads(raw_visible_native)
    collision = json.loads(raw_collision)
    finite = json.loads(originals["finite-dry-source.json"])
    require(collision["id"] == source["id"] == "SH073", "changed Griffon Spectre identity")
    require(collision["status"] == "GENERATED_CANDIDATE_ONLY_NOT_INSTALLED_OR_NATIVE_QUALIFIED", "wrong Griffon Spectre collision scope")
    require(visible_native["nativeDesign"] == source["nativeDesign"], "changed Griffon Spectre visible native archive")
    require(collision["originalSourceSHA256"] == visible_native["sourceSHA256"], "changed collision source lineage")
    require(finite["metresPerSourceUnit"] == float(SCALE), "changed generated Griffon Spectre calibration")
    owner = json.loads(originals["ORIGINAL-OWNERSHIP.json"])["models"]
    detail, archived_visual = metre_space(source["design"]), metre_space(source["nativeDesign"])
    shape = metre_space(collision["nativeCandidate"])
    slots, (detail_mesh, collision_mesh) = assemble((detail, owner["design"]["parts"]), (shape, collision["parts"]))
    counts = len(slots), len(detail["vertices"]), len(archived_visual["vertices"]), len(shape["vertices"])
    require(counts == (29, 952, 336, 228), "wrong Griffon Spectre vertex counts")
    require([len(mesh["faces"]) for mesh in (detail, archived_visual, shape)] == [1644, 596, 380],
            "wrong Griffon Spectre visible or collision triangle count")
    vertices = detail_mesh["vertices"] + collision_mesh["vertices"]
    split = len(detail_mesh["vertices"])
    faces = detail_mesh["triangles"] + [[index+split for index in face] for face in collision_mesh["triangles"]]
    outputs = {
        "source.vertices": struct.pack("<8sII", b"MSH073V1", split,len(vertices)-split) +
            b"".join(struct.pack("<3d",*point) for point in vertices),
        "source-triangles.bin": b"".join(struct.pack("<3I",*face) for face in faces),
        "source-piece-membership.bin": bytes(detail_mesh["groups"] + collision_mesh["groups"]),
        "PIECES.json": encode({"pieces":[{"name":name,"occurrence":ordinal} for name,ordinal in slots],"roundingMetres":.02}),
        "visual.bin": b"GRISVIZ1" + encode_visual(detail)[8:],
        "native-visual.bin": b"GRISVIZ1" + encode_visual(archived_visual)[8:],
    }
    readiness = json.loads((INTEGRATION / "candidate-support.json").read_bytes())
    probes = []
    for route in readiness["routes"]:
        require(route["clearanceMillimetres"] > 100, "failed Griffon Spectre artwork probe")
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a-b for a,b in zip(start,route["travel"],strict=True)]
        probes.append({"name":route["name"],"start":start,"end":end,"radiusMetres":.1})
    require(len(probes) == 7, "wrong Griffon Spectre artwork probe count")
    engineering=finite["emptyDryReference"]
    inertia=engineering["inertiaAboutCentreOfMassKgM2"]
    outputs[STUDIO_NAME] = encode({
        "schema": 1, "id":"SH073", "name":"Griffon Spectre", "metresPerUnit":1,
        "roundingMetres":.02, "detail":detail_mesh, "native":collision_mesh,
        "materials":[
            {"name":m["name"],"rgb":[m["colour"]>>shift & 255 for shift in (24,16,8)],
             "alpha":m["colour"] & 255,"emissive":m["emissive"]}
            for m in detail["materials"]],
        "faceMaterials":detail["faceMaterials"],
        "faceTextures":[{"uv":m["uv"],"texture":detail["textures"][m["texture"]]["asset"] if m["texture"]>=0 else None}
                        for m in detail["faceTextures"]],
        "engineering":{"massKg":engineering["massKg"],"centerOfMass":engineering["centreMetres"],
                       "inertia":[inertia[i:i+3] for i in (0,3,6)]},
        "flight":{"maxSpeed":source["game"]["speed"]},
        "clearances":probes,
        "provenance":{
            "sourceId":"SH073","revision":"a11fe16924d87d06021816f9037952afcbac08db",
            "frozenResource":pin(raw_source),
            "frozenVisibleNative":{"artifact":"native-export.json","triangleCount":596,**pin(raw_visible_native)},
            "generatedInspectionCollision":{"artifact":"GENERATED-COLLISION.json","triangleCount":380,"triangleLimit":512,
                                           "coordinateUnits":"relative","artifactRole":"generated collision inspection",**pin(raw_collision)},
            "derivedHashes":{name:pin(data) for name,data in outputs.items()},
            "originalScaleRatio":[SCALE.numerator,SCALE.denominator],
            "engineeringReference":engineering["id"],"scope":"offline collision catalogue inspection",
        },
    })
    outputs["SOURCE-GEOMETRY.json"] = encode({
        "id": "SH073", "originalScaleRatio": [SCALE.numerator, SCALE.denominator],
        "archivedDetailedFaces": 1644, "archivedVisibleNativeFaces": 596,
        "generatedInspectionCollisionFaces": 380, "collisionUnits": "relative",
        "collisionSupportGroups": len(slots),
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
            require(path.read_bytes() == content, f"changed Griffon Spectre output: {name}")
        else:
            replace_output(path, content)


if __name__ == "__main__":
    main()
