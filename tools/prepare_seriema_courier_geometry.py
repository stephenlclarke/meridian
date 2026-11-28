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
ORIGINAL = ROOT / "assets/fleet/sh063"
INTEGRATION = ROOT / "integration/sh063-seriema-courier"
STUDIO_NAME = "SERIEMA-COURIER-STUDIO.json"
FROZEN = {
    'resource.toml': '296b86cff9f25fbe83e60f6efaddb2b3f59474d7d9ff53df83eb054bc1ea0efa',
    'native-export.json': '5965142a4526b4db6eaff87acdb62ecc83082e9f6eea34be99a5745bae546a12',
    'contract.toml': '234fb257ba59abe14540cb18f87c9fbf62332f0d6172b91487b67bc355a81faa',
    'engineering.json': 'eba997ff8fcb3cca2040d7237778fe29e19553383aee330c0155e2f5e44806dd',
    'finite-dry-source.json': 'c08ed9285d464511cb580ee4032c66ed4294a304e3f76b9a23913d076535f059',
    'LEGACY-FINITE-DRY.swift.txt': '0e1e8ebe0a04dc3e37ae83a22c1fcabed1df705972b6ec6e2843bbb459012d12',
    'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
    'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
    'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
    'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d',
    'ORIGINAL-OWNERSHIP.json': 'c97ff5f6027e5df2d70d450da5c2aafc2b1b7b0ad7935231ec65eba81598dfee',
    'CREATOR-CANDIDATE.json': '01fe5a05526d0380715e7187992f447c66277d3ed708daf3655204a5d10a394d',
    'CREATOR-BLUEPRINT.json': '461ff3ad5a14b8bcf0d4219aa90bb90467ad35b929d264715ce1a03683b154ed',
    'LEGACY-CANDIDATE-RECIPE.py.txt': '8efd00e4fdc2b9378b421c7cf99a3ecb9c6234f69dc0d797c541e7bccb957ca2',
    'LEGACY-CANDIDATE-TESTS.py.txt': 'd051d4b86e65931b8a1e7ec5448501a449de7fd0743c3d473fdd435285066fe5',
}

SCALE = Fraction(75, 11)


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
        require(pin(contents)["sha256"] == expected, f"changed Seriema Courier source: {name}")
        files[name] = contents
    return files


def build():
    originals = load_frozen()
    raw_source = originals["resource.toml"]
    raw_visible_native = originals["native-export.json"]
    raw_collision = originals["CREATOR-CANDIDATE.json"]
    source, visible_native = tomllib.loads(raw_source.decode()), json.loads(raw_visible_native)
    collision = json.loads(raw_collision)
    finite = json.loads(originals["finite-dry-source.json"])
    require(collision["catalogueID"] == source["id"] == "SH063", "changed Seriema Courier identity")
    require(collision["status"] == "SOURCE_GEOMETRY_ONLY", "wrong Seriema Courier collision scope")
    require(visible_native["nativeDesign"] == source["nativeDesign"], "changed Seriema Courier visible native archive")
    require(collision["coordinateUnits"] == "metres" and collision["calibration"] == "75/11",
            "wrong Seriema Courier collision calibration")
    require(collision["originalSourceSHA256"] == visible_native["sourceSHA256"], "changed collision source lineage")
    require(finite["metresPerSourceUnit"] == float(SCALE), "changed frozen Seriema Courier calibration")
    owner = json.loads(originals["ORIGINAL-OWNERSHIP.json"])["models"]
    detail, archived_visual = metre_space(source["design"]), metre_space(source["nativeDesign"])
    shape = collision["nativeDesign"]
    slots, (detail_mesh, collision_mesh) = assemble((detail, owner["design"]["parts"]), (shape, collision["parts"]))
    counts = len(slots), len(detail["vertices"]), len(archived_visual["vertices"]), len(shape["vertices"])
    require(counts == (30, 960, 344, 296), "wrong Seriema Courier vertex counts")
    require([len(mesh["faces"]) for mesh in (detail, archived_visual, shape)] == [1656, 608, 512],
            "wrong Seriema Courier visible or collision triangle count")
    vertices = detail_mesh["vertices"] + collision_mesh["vertices"]
    split = len(detail_mesh["vertices"])
    faces = detail_mesh["triangles"] + [[index+split for index in face] for face in collision_mesh["triangles"]]
    outputs = {
        "source.vertices": struct.pack("<8sII", b"MSH063V1", split,len(vertices)-split) +
            b"".join(struct.pack("<3d",*point) for point in vertices),
        "source-triangles.bin": b"".join(struct.pack("<3I",*face) for face in faces),
        "source-piece-membership.bin": bytes(detail_mesh["groups"] + collision_mesh["groups"]),
        "PIECES.json": encode({"pieces":[{"name":name,"occurrence":ordinal} for name,ordinal in slots],"roundingMetres":.02}),
        "visual.bin": b"SERIVIZ1" + encode_visual(detail)[8:],
        "native-visual.bin": b"SERIVIZ1" + encode_visual(archived_visual)[8:],
    }
    readiness = json.loads((INTEGRATION / "candidate-support.json").read_bytes())
    probes = []
    for route in readiness["routes"]:
        require(route["clearanceMillimetres"] > 100, "failed Seriema Courier artwork probe")
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a-b for a,b in zip(start,route["travel"],strict=True)]
        probes.append({"name":route["name"],"start":start,"end":end,"radiusMetres":.1})
    require(len(probes) == 7, "wrong Seriema Courier artwork probe count")
    engineering=finite["emptyDryReference"]
    inertia=engineering["inertiaAboutCentreOfMassKgM2"]
    outputs[STUDIO_NAME] = encode({
        "schema": 1, "id":"SH063", "name":"Seriema Courier", "metresPerUnit":1,
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
            "sourceId":"SH063","revision":"a11fe16924d87d06021816f9037952afcbac08db",
            "frozenResource":pin(raw_source),
            "frozenVisibleNative":{"artifact":"native-export.json","triangleCount":608,**pin(raw_visible_native)},
            "separatelyArchivedCollision":{"artifact":"CREATOR-CANDIDATE.json","triangleCount":512,"triangleLimit":512,
                                           "coordinateUnits":"metres","generatorReproduction":"mismatch",**pin(raw_collision)},
            "derivedHashes":{name:pin(data) for name,data in outputs.items()},
            "originalScaleRatio":[SCALE.numerator,SCALE.denominator],
            "engineeringReference":engineering["id"],"scope":"offline collision catalogue inspection",
        },
    })
    outputs["SOURCE-GEOMETRY.json"] = encode({
        "id": "SH063", "originalScaleRatio": [SCALE.numerator, SCALE.denominator],
        "archivedDetailedFaces": 1656, "archivedVisibleNativeFaces": 608,
        "separatelyArchivedCollisionFaces": 512, "collisionUnits": "metres",
        "collisionSupportGroups": len(slots), "generatorReproduction": "mismatch",
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
            require(path.read_bytes() == content, f"changed Seriema Courier output: {name}")
        else:
            replace_output(path, content)


if __name__ == "__main__":
    main()
