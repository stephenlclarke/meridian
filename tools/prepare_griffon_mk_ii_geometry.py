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
ORIGINAL = ROOT / "assets/fleet/sh072"
INTEGRATION = ROOT / "integration/sh072-griffon-mk-ii"
STUDIO_NAME = "GRIFFON-MK-II-STUDIO.json"
FROZEN = {
    'resource.toml': '376f8ee997f911c34ebd068636d9d37b9ee2c55131c38f3b3ae48fa308578ba0',
    'native-export.json': '484dc62f693a95584220a8f6b55a48052d657e9e4bce6784acf801fae2012e9e',
    'contract.toml': '4be1bb6109ec00b2bb05dbba194562054f10d27ee8ff4a105a310b7aa3370498',
    'engineering.json': '276171fa798535cc68af1bbce174f3defda19659c4a386113159557f73b91273',
    'finite-dry-source.json': '6b24e36f71e3dbf7214131db1eb0373aa9311c981cfec665cb33364252c559d1',
    'LEGACY-FINITE-DRY.swift.txt': 'aa18ead5d124ada86be42047175494b243a6d76a4a7d0d6606dde532aa6c8ea9',
    'LEGACY-PHYSICAL-CONTRACT.swift.txt': '2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
    'l337-patrol-diffuse.rgba': '9699517d12b4fbbb28746e00bed9ee65c19a098d14402b1d5160280c6f6bfc54',
    'l337-strike-diffuse.rgba': 'e56f149228caf90237a652d720ae549768bde1056af3d903dc8a5c243ad26e97',
    'l337-service-details.rgba': '35edd84f6ce527195dd8a9bcc039521a6efc5aa2333452bbbc30217b8372103d',
    'ORIGINAL-OWNERSHIP.json': '51564de83af450d111ee06564cb199929c3caa5de0b58c6efeb934f8d7f7fb55',
    'GENERATED-COLLISION.json': 'a162f31e9aa3d9de48018b235589f411dc50570c59bddb861e53f346c6ec96b1',
}

SCALE = Fraction(30000000, 3215237)


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
        require(pin(contents)["sha256"] == expected, f"changed Griffon Mk II source: {name}")
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
    require(collision["id"] == source["id"] == "SH072", "changed Griffon Mk II identity")
    require(collision["status"] == "GENERATED_CANDIDATE_ONLY_NOT_INSTALLED_OR_NATIVE_QUALIFIED", "wrong Griffon Mk II collision scope")
    require(visible_native["nativeDesign"] == source["nativeDesign"], "changed Griffon Mk II visible native archive")
    require(collision["originalSourceSHA256"] == visible_native["sourceSHA256"], "changed collision source lineage")
    require(finite["metresPerSourceUnit"] == float(SCALE), "changed generated Griffon Mk II calibration")
    owner = json.loads(originals["ORIGINAL-OWNERSHIP.json"])["models"]
    detail, archived_visual = metre_space(source["design"]), metre_space(source["nativeDesign"])
    shape = metre_space(collision["nativeCandidate"])
    slots, (detail_mesh, collision_mesh) = assemble((detail, owner["design"]["parts"]), (shape, collision["parts"]))
    counts = len(slots), len(detail["vertices"]), len(archived_visual["vertices"]), len(shape["vertices"])
    require(counts == (29, 952, 336, 228), "wrong Griffon Mk II vertex counts")
    require([len(mesh["faces"]) for mesh in (detail, archived_visual, shape)] == [1644, 596, 380],
            "wrong Griffon Mk II visible or collision triangle count")
    vertices = detail_mesh["vertices"] + collision_mesh["vertices"]
    split = len(detail_mesh["vertices"])
    faces = detail_mesh["triangles"] + [[index+split for index in face] for face in collision_mesh["triangles"]]
    outputs = {
        "source.vertices": struct.pack("<8sII", b"MSH072V1", split,len(vertices)-split) +
            b"".join(struct.pack("<3d",*point) for point in vertices),
        "source-triangles.bin": b"".join(struct.pack("<3I",*face) for face in faces),
        "source-piece-membership.bin": bytes(detail_mesh["groups"] + collision_mesh["groups"]),
        "PIECES.json": encode({"pieces":[{"name":name,"occurrence":ordinal} for name,ordinal in slots],"roundingMetres":.02}),
        "visual.bin": b"GRI2VIZ1" + encode_visual(detail)[8:],
        "native-visual.bin": b"GRI2VIZ1" + encode_visual(archived_visual)[8:],
    }
    readiness = json.loads((INTEGRATION / "candidate-support.json").read_bytes())
    probes = []
    for route in readiness["routes"]:
        require(route["clearanceMillimetres"] > 100, "failed Griffon Mk II artwork probe")
        start = [math.fsum(axis) / 3 for axis in zip(*route["triangle"], strict=True)]
        end = [a-b for a,b in zip(start,route["travel"],strict=True)]
        probes.append({"name":route["name"],"start":start,"end":end,"radiusMetres":.1})
    require(len(probes) == 7, "wrong Griffon Mk II artwork probe count")
    engineering=finite["emptyDryReference"]
    inertia=engineering["inertiaAboutCentreOfMassKgM2"]
    outputs[STUDIO_NAME] = encode({
        "schema": 1, "id":"SH072", "name":"Griffon Mk II", "metresPerUnit":1,
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
            "sourceId":"SH072","revision":"a11fe16924d87d06021816f9037952afcbac08db",
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
        "id": "SH072", "originalScaleRatio": [SCALE.numerator, SCALE.denominator],
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
            require(path.read_bytes() == content, f"changed Griffon Mk II output: {name}")
        else:
            replace_output(path, content)


if __name__ == "__main__":
    main()
