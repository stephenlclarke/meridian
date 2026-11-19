"""Authenticate and pack the original Raven with source-owned local supports."""

import argparse
from collections import Counter
from decimal import Decimal
from fractions import Fraction
import hashlib
import itertools
import json
import math
from pathlib import Path
import struct
import subprocess
import tomllib

from encode_goshawk_visual import encode_model
from import_goshawk_envelope_source import components

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets/fleet/sh015"
LEGACY_REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
PINS = {
    "resource.toml": ("Sources/L337Core/Resources/Ships/SH015.toml", "2a1173b5437c16144bf46b921e8888b229685bc7d046b8fc8e4a14eb1702c412"),
    "contract.toml": ("docs/fleet-design-v2/ships/SH015.toml", "9996fa6457ac946c7f01ed8dcab21275b17f260fd4f4cf106bd9d0fa1df71c66"),
    "native-source.json": ("Sources/L337Core/Resources/OriginalFleetShowroomModels/SH015.json", "65e64ab37093999801a7267c7054ee3208b9b515a44169f26c46bfb02e419b66"),
    "native-export-manifest.json": ("Sources/L337Core/Resources/OriginalFleetShowroomModels/manifest.json", "118b05757dc54096faa15dd79801f38999b87fe9424f1b3513133ead94f71971"),
    "physical.json": ("Sources/L337Core/Resources/OriginalFleetPhysical/SH015.json", "1de6745b7b55bdd273ed863e5e714691fc78eb6a449af1a4af2007094d919952"),
    "ownership.json": ("docs/milestones/fleet-physical/creator-ownership-records/SH015.json", "1649cdd7aa44135bc7dce9516da4b9d8112b2c9bf1953ff33806177ad2836846"),
    "LICENSE": ("LICENSE", "033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766"),
}
DESIGN_SHA = "e91491dc97b465dea7c09c06f0d0da96a50b8ab0155d3e2af22ed1f99a14e1ca"
MANIFEST_SHA = "b66cba3e414994f0c11317948ad23b31125a7d27c6fe10cf98395ab1e5f7f8f3"
SCALE = Fraction(30000, 2521)
COUNTS = ((2080, 3496, 166), (544, 976, 28))
PIECE_COUNT = 70
MAGIC = b"MSH015V1"
VISUAL_MAGIC = b"SH015VZ1"
TEXTURE_SOURCE_SHA = "96c4c9699b81f50be5a8aefeeca74a6e4870e0bb981d1cd33a9ff2921cd2bd3c"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def pin(raw):
    return {"bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def json_bytes(value):
    return (json.dumps(value, indent=2, allow_nan=False) + "\n").encode()


def authenticated(inputs):
    for name, (_, expected) in PINS.items():
        require(pin(inputs[name])["sha256"] == expected, f"Changed SH015 source: {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode(), parse_float=Decimal)
    contract = tomllib.loads(inputs["contract.toml"].decode(), parse_float=Decimal)
    export = json.loads(inputs["native-source.json"], parse_float=Decimal)
    engineering = json.loads(inputs["physical.json"])
    ownership = json.loads(inputs["ownership.json"])
    index = json.loads(inputs["native-export-manifest.json"])
    require(all(x["id"] == "SH015" for x in (resource, contract, export, engineering, ownership)), "Wrong SH015 identity")
    require(resource["name"] == contract["name"] == "Raven", "Changed hull name")
    info = contract["source"]
    require(info["geometryFile"] == export["sourcePath"] == "replacement-hulls/SH015-design.toml", "Changed original design path")
    require(info["geometrySHA256"] == export["sourceSHA256"] == engineering["detailedSourceSHA256"] == ownership["geometrySHA256"] == DESIGN_SHA, "Changed original design pin")
    require(engineering["contractSHA256"] == ownership["contractSHA256"] == pin(inputs["contract.toml"])["sha256"], "Changed engineering design binding")
    require(engineering["componentSourceSHA256"] == pin(inputs["ownership.json"])["sha256"], "Changed creator component binding")
    require(Fraction(engineering["metresPerSourceUnitExact"]) == SCALE and engineering["sourceOriginUnchanged"], "Changed adopted calibration")
    require(resource["nativeDesign"] == export["nativeDesign"], "Original native display and resource disagree")
    expected = {"id": "SH015", "file": "SH015.json", "sourcePath": info["geometryFile"], "sourceSHA256": DESIGN_SHA, **pin(inputs["native-source.json"])}
    require([r for r in index["models"] if r["id"] == "SH015"] == [expected], "SH015 native exporter does not bind the original")
    require(contract["geometry"]["nativeFaceLimit"] == 512 and contract["performance"]["maxSpeed"] == Decimal("46.0"), "Changed authored limit or speed")
    return resource, ownership, engineering


def local_piece_names(parts, variant):
    """Use creator ownership; keep original detailed/candidate wells separate."""
    ordinary = Counter()
    for part in parts:
        name = part["name"]
        if "/service-well-" in name:
            yield f'{variant}: {name.split("/recess-service-strut-",1)[0]}'
        elif name == "broken spar raised shoulder":
            # The two identical authored labels occur in negative-X, then positive-X order
            # in both R3-owned source lists; do not merge across the spar.
            side = ("negative X","positive X")[ordinary[name]]
            ordinary[name] += 1
            yield f"{name} ({side})"
        else:
            require(ordinary[name] == 0, f"Unresolved repeated creator name: {name}")
            ordinary[name] += 1
            yield name
    require(ordinary["broken spar raised shoulder"] == 2, "Changed original shoulder ownership")


def partition(resource, ownership):
    slots = {}
    tables = []
    for model, variant in (("design","detailed"),("nativeDesign","nativeCandidate")):
        parts = ownership["models"][model]["parts"]
        memberships = {}
        for part, name in zip(parts, local_piece_names(parts,variant), strict=True):
            piece = slots.setdefault(name,len(slots))
            start, count = part["vertexStart"], part["vertexCount"]
            require(all(v not in memberships for v in range(start,start+count)), "Overlapping original component")
            memberships.update((v,piece) for v in range(start,start+count))
            if name.startswith("broken spar raised shoulder"):
                xs = [p[0] for p in resource[model]["vertices"][start:start+count]]
                require((max(xs)<0 if "negative" in name else min(xs)>0), "Shoulder side changed")
        tables.append(memberships)
    require(len(slots) == PIECE_COUNT, "Changed Raven local support count")
    return *tables,list(slots)


def stage_variant(model, owner, expected, pieces):
    vertex_count, face_count, component_count = expected
    vertices, faces = model["vertices"], model["faces"]
    require((len(vertices),len(faces),len(owner["parts"])) == expected, "Changed source topology")
    require(set(pieces) == set(range(vertex_count)), "Unassigned source vertices")
    require(model["paletteName"] == "rust-ivory" and model["license"] == model["appearanceLicense"] == "MIT", "Changed original appearance")
    calibrated = [tuple(float(Fraction(x)*SCALE) for x in point) for point in vertices]
    require(all(len(p)==3 and all(math.isfinite(x) for x in p) for p in calibrated), "Invalid calibrated source")
    require(all(len(f)==3 and len(set(f))==3 and all(type(i) is int and 0<=i<vertex_count for i in f) for f in faces), "Invalid face")
    require(all(len({pieces[i] for i in f})==1 for f in faces), "A triangle crosses local supports")
    groups = components(model)
    require(len(groups)==component_count, "Changed original connected components")
    records = []
    cursor = 0
    for index,(group,original) in enumerate(zip(groups,owner["parts"],strict=True)):
        start,count = original["vertexStart"],original["vertexCount"]
        first_face,face_len = original["faceStart"],original["faceCount"]
        require(original["index"]==index and group==list(range(start,start+count)), "Creator vertex partition mismatch")
        require(first_face==cursor and all(start<=v<start+count for f in faces[first_face:first_face+face_len] for v in f), "Creator face partition mismatch")
        cursor += face_len
        require(len({pieces[i] for i in group})==1, "Creator component split")
        runs = [{"material": key,"faces":sum(1 for _ in run)} for key,run in itertools.groupby(model["faceMaterials"][first_face:cursor])]
        require(runs==original["exactMaterialRuns"], "Creator material run mismatch")
        records.append({"component":index,"name":original["name"],"startVertex":start,"vertexCount":count,
                        "startFace":first_face,"faceCount":face_len,"piece":pieces[start]})
    require(cursor==face_count,"Unassigned source faces")
    return calibrated,faces,records


def textures(resource):
    raw = (ROOT/"assets/fleet/sh001/TEXTURE-SOURCE.json").read_bytes()
    require(pin(raw)["sha256"]==TEXTURE_SOURCE_SHA,"Changed shared texture manifest")
    by_id = {asset["id"]:asset for asset in json.loads(raw)["assets"]}
    result = {}
    for texture in resource["design"]["textures"]:
        row = by_id[texture["asset"]]
        name = row["rgbaFile"]
        path = f"assets/fleet/sh001/{name}"
        expected = {"bytes":row["width"]*row["height"]*4,"sha256":row["rgbaSHA256"]}
        require(pin((ROOT/path).read_bytes())==expected,f"Changed original texture: {path}")
        result[path] = {**expected,"source":f"Sources/L337Core/Resources/OriginalShipTextures/{name}","license":row["license"]}
    return result


def build(inputs):
    resource,owner,engineering = authenticated(inputs)
    detail_pieces,native_pieces,names = partition(resource,owner)
    detailed,native = (stage_variant(resource[key],owner["models"][key],expected,table)
                       for key,expected,table in (("design",COUNTS[0],detail_pieces),("nativeDesign",COUNTS[1],native_pieces)))
    vertices = bytearray(struct.pack("<8sII",MAGIC,COUNTS[0][0],COUNTS[1][0]))
    faces = bytearray()
    memberships = bytearray()
    for (points,triangles,_),table,offset in ((detailed,detail_pieces,0),(native,native_pieces,COUNTS[0][0])):
        vertices.extend(b"".join(struct.pack("<3d",*p) for p in points))
        faces.extend(b"".join(struct.pack("<3I",*(i+offset for i in f)) for f in triangles))
        memberships.extend(table[i] for i in range(len(points)))
    encoded = {
        "source.vertices":bytes(vertices),
        "source-triangles.bin":bytes(faces),
        "source-piece-membership.bin":bytes(memberships),
        "COMPONENT-PIECES.json":json_bytes({"pieces":names,"detailed":detailed[2],"nativeCandidate":native[2]}),
    }
    encoded["visual.bin"] = encode_model(dict(resource["design"],vertices=detailed[0]),vertices,faces,MAGIC,
                                        (COUNTS[0][0],COUNTS[1][0],COUNTS[0][1]),VISUAL_MAGIC)
    bounds = [[fn(axis) for axis in zip(*v)] for v in (detailed[0],native[0]) for fn in (min,max)]
    require(abs(bounds[1][2]-bounds[0][2]-30)<1e-12,"Changed adopted engineering geometry span")
    manifest = {
        "id":"SH015","legacyRevision":LEGACY_REVISION,"originalDesignSHA256":DESIGN_SHA,
        "metresPerSourceUnit":str(SCALE),"axes":"+Z forward, +Y up; original origin",
        "detailed":{"vertices":COUNTS[0][0],"faces":COUNTS[0][1],"components":COUNTS[0][2],"boundsMetres":bounds[:2]},
        "nativeCandidate":{"vertices":COUNTS[1][0],"faces":COUNTS[1][1],"components":COUNTS[1][2],"boundsMetres":bounds[2:],"exactFaceLimit":512},
        "physicalAuthority":{"status":engineering["status"],"dryMassKg":engineering["dryMassKilograms"],"sha256":pin(inputs["physical.json"])["sha256"]},
        "collision":{"pieces":len(names),"roundingMetres":0.02,"scope":"Source-local rounded convex supports; corresponding authored structural parts share a support while separate original well mouths keep separate local supports. Native and ordinary admission pending."},
        "legacyInputs":{n:{"path":p,**pin(inputs[n])} for n,(p,_) in PINS.items()},
        "encoded":{name:pin(data) for name,data in encoded.items()},"sharedTextures":textures(resource),
    }
    encoded["MANIFEST.json"] = json_bytes(manifest)
    return encoded


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--legacy-repo",type=Path)
    parser.add_argument("--check",action="store_true")
    args=parser.parse_args()
    if args.check:
        inputs={name:(ASSETS/name).read_bytes() for name in PINS}
    else:
        if args.legacy_repo is None:
            parser.error("--legacy-repo is required to stage immutable originals")
        inputs={name:subprocess.check_output(["git","-C",str(args.legacy_repo),"show",f"{LEGACY_REVISION}:{path}"]) for name,(path,_) in PINS.items()}
    outputs={**inputs,**build(inputs)}
    require(pin(outputs["MANIFEST.json"])["sha256"] == MANIFEST_SHA, "Changed SH015 manifest")
    if args.check:
        for name,blob in outputs.items():
            require((ASSETS/name).read_bytes()==blob,f"Changed SH015 encoded asset: {name}")
    else:
        ASSETS.mkdir(parents=True,exist_ok=True)
        for name,blob in outputs.items():
            (ASSETS/name).write_bytes(blob)
    print(f"SH015 own original geometry and {PIECE_COUNT} local supports verified ({len(outputs)} assets)")


if __name__=="__main__":
    main()
