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
ORIGINAL = ROOT / "assets/fleet/sh071"
INTEGRATION = ROOT / "integration/sh071-griffon"
STUDIO_NAME = "GRIFFON-STUDIO.json"
FROZEN = {
    'resource.toml': '911a47c6e0044e0fc103951432f11b34bf3e2dcba83509f28c620d25e097e5c6',
    'native-export.json': 'd098411e4ded08d59a6ba55ef86c958e2f6d66a7ea2f03f383fb71aaa4654e7f',
    'contract.toml': 'aae0fbafceeb0adf78d050c184bdca3eb4b0a1f11240c83430aaa27e1a5c3519',
    'engineering.json': '4cf43c8db53f2e00177709bbd2761ef7360be2520a3576cb78a158bead2f31f7',
    'finite-dry-source.json': '7ce9ee10f8aba26dc4aa7de7cfa3da3d657343f133aad4fcad6a7b9b18be2afe',
    'LEGACY-FINITE-DRY.swift.txt': 'd86db50fbdebb2374d8e0eba2c423e2a185e0d8e451700ccac65fd6a8719302e',
    'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
    'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
    'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
    'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d',
    'ARCHIVED-COLLISION.json': '58422ae55aed68fad39c2a510afecae5b3d0a6c5568941a4639ec648a5109fb4',
    'ORIGINAL-OWNERSHIP.json': 'c8feae884db573e8219147d3f638d1fde73b928dc16c52e5d9a8492d5d9ebe99',
    'CREATOR-BLUEPRINT.json': '29ddf1d0c59dc236c059b3ce0c8053e8cdced78b7fff4b6248eae511575353ff',
    'LEGACY-CANDIDATE-RECIPE.py.txt': 'd3794b192555dffe491e224a4502c16fc34aa5c565f04cacdb30a542177b2d66',
    'LEGACY-CANDIDATE-TESTS.py.txt': '36d65ff4a88e5153ba43a969e6abf6d621b4609b6ea9a01f19c510d06edf6a7b',
    'LEGACY-FACET-FREEZE.py.txt': '32e564bf1952bd28375febb5d68721646ddd42b2dd920e4f0ed620f9fad67e67',
    'LEGACY-PROFILE-FREEZE.py.txt': '3917ad5219c80bb360d96a155a4684eb8991bf31abd8ff2992d60b32a37620b7',
}

SCALE = Fraction(60000, 6221)


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
        require(pin(contents)["sha256"] == expected, f"changed Griffon source: {name}")
        files[name] = contents
    return files


def build():
    originals = load_frozen()
    raw_source = originals["resource.toml"]
    raw_visible_native = originals["native-export.json"]
    raw_collision = originals["ARCHIVED-COLLISION.json"]
    source, visible_native = tomllib.loads(raw_source.decode()), json.loads(raw_visible_native)
    collision = json.loads(raw_collision)
    finite = json.loads(originals["finite-dry-source.json"])
    require(collision["catalogueID"] == source["id"] == "SH071", "changed Griffon identity")
    require(collision["status"] == "SOURCE_GEOMETRY_ONLY", "wrong Griffon collision scope")
    require(visible_native["nativeDesign"] == source["nativeDesign"], "changed Griffon visible native archive")
    require(collision["coordinateUnits"] == "metres" and collision["calibration"] == "60000/6221",
            "wrong Griffon collision calibration")
    require(collision["originalSourceSHA256"] == visible_native["sourceSHA256"], "changed collision source lineage")
    require(finite["metresPerSourceUnit"] == float(SCALE), "changed frozen Griffon calibration")
    owner = json.loads(originals["ORIGINAL-OWNERSHIP.json"])["models"]
    detail, archived_visual = metre_space(source["design"]), metre_space(source["nativeDesign"])
    shape = collision["nativeDesign"]
    slots, (detail_mesh, collision_mesh) = assemble((detail, owner["design"]["parts"]), (shape, collision["parts"]))
    counts = len(slots), len(detail["vertices"]), len(archived_visual["vertices"]), len(shape["vertices"])
    require(counts == (29, 952, 336, 294), "wrong Griffon vertex counts")
    require([len(mesh["faces"]) for mesh in (detail, archived_visual, shape)] == [1644, 596, 512],
            "wrong Griffon visible or collision triangle count")
    vertices = detail_mesh["vertices"] + collision_mesh["vertices"]
    split = len(detail_mesh["vertices"])
    faces = detail_mesh["triangles"] + [[index+split for index in face] for face in collision_mesh["triangles"]]
    outputs = {
        "source.vertices": struct.pack("<8sII", b"MSH071V1", split,len(vertices)-split) +
            b"".join(struct.pack("<3d",*point) for point in vertices),
        "source-triangles.bin": b"".join(struct.pack("<3I",*face) for face in faces),
        "source-piece-membership.bin": bytes(detail_mesh["groups"] + collision_mesh["groups"]),
        "PIECES.json": encode({"pieces":[{"name":name,"occurrence":ordinal} for name,ordinal in slots],"roundingMetres":.02}),
        "visual.bin": b"GRIFVIZ1" + encode_visual(detail)[8:],
        "native-visual.bin": b"GRIFVIZ1" + encode_visual(archived_visual)[8:],
    }
    readiness = json.loads((INTEGRATION / "candidate-support.json").read_bytes())
    probes = []
    for route in readiness["routes"]:
        require(route["clearanceMillimetres"] > 100, "failed Griffon artwork probe")
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a-b for a,b in zip(start,route["travel"],strict=True)]
        probes.append({"name":route["name"],"start":start,"end":end,"radiusMetres":.1})
    require(len(probes) == 7, "wrong Griffon artwork probe count")
    engineering=finite["emptyDryReference"]
    inertia=engineering["inertiaAboutCentreOfMassKgM2"]
    outputs[STUDIO_NAME] = encode({
        "schema": 1, "id":"SH071", "name":"Griffon", "metresPerUnit":1,
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
            "sourceId":"SH071","revision":"a11fe16924d87d06021816f9037952afcbac08db",
            "frozenResource":pin(raw_source),
            "frozenVisibleNative":{"artifact":"native-export.json","triangleCount":596,**pin(raw_visible_native)},
            "separatelyArchivedCollision":{"artifact":"ARCHIVED-COLLISION.json","triangleCount":512,"triangleLimit":512,
                                           "coordinateUnits":"metres","artifactRole":"frozen collision inspection",**pin(raw_collision)},
            "derivedHashes":{name:pin(data) for name,data in outputs.items()},
            "originalScaleRatio":[SCALE.numerator,SCALE.denominator],
            "engineeringReference":engineering["id"],"scope":"offline collision catalogue inspection",
        },
    })
    outputs["SOURCE-GEOMETRY.json"] = encode({
        "id": "SH071", "originalScaleRatio": [SCALE.numerator, SCALE.denominator],
        "archivedDetailedFaces": 1644, "archivedVisibleNativeFaces": 596,
        "separatelyArchivedCollisionFaces": 512, "collisionUnits": "metres",
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
            require(path.read_bytes() == content, f"changed Griffon output: {name}")
        else:
            replace_output(path, content)


if __name__ == "__main__":
    main()
