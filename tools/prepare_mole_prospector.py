"""Authenticate and pack the original Mole Prospector with source-owned local supports."""

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
ASSETS = ROOT / "assets/fleet/sh017"
LEGACY_REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
PINS = {
    "resource.toml": ("Sources/L337Core/Resources/Ships/SH017.toml", "1a31ae1c3827227f79d7fa826d69c6970d34806400d9ab642d0db0f1ddcd2fb6"),
    "contract.toml": ("docs/fleet-design-v2/ships/SH017.toml", "c31408f9cbf0e76af47c583fcf084080addb1d8e84970cc0168d7c065bae385a"),
    "native-source.json": ("Sources/L337Core/Resources/OriginalFleetShowroomModels/SH017.json", "fc85a8cf974366ce38285485feda72e6399eed36506e019e7341f8905188f622"),
    "native-export-manifest.json": ("Sources/L337Core/Resources/OriginalFleetShowroomModels/manifest.json", "118b05757dc54096faa15dd79801f38999b87fe9424f1b3513133ead94f71971"),
    "physical.json": ("Sources/L337Core/Resources/OriginalFleetPhysical/SH017.json", "1e63f301befca35e4b3a118075a247f6c9a228a623fb8b8a04b2043351a70e64"),
    "ownership.json": ("docs/milestones/fleet-physical/creator-ownership-records/SH017.json", "f1f7dde5ea9df700c825b1ac9f1405eee63b02327225a12f0afab2350927d48b"),
    "LICENSE": ("LICENSE", "033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766"),
}
DESIGN_SHA = "2a5efee25247b09a861d5112c7d070685b1f9aa2892143505fca28332fb2d4f3"
MANIFEST_SHA = "72ee0f8388143e63e8cec7d6ddaffae4cd4296240856cf891ecd2a3b115949c2"
SCALE = Fraction(920000, 56367)
COUNTS = ((6760, 12836, 179), (616, 1100, 33))
PIECE_COUNT = 83
MAGIC = b"MSH017V1"
VISUAL_MAGIC = b"SH017VZ1"
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
        require(pin(inputs[name])["sha256"] == expected, f"Changed SH017 source: {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode(), parse_float=Decimal)
    contract = tomllib.loads(inputs["contract.toml"].decode(), parse_float=Decimal)
    export = json.loads(inputs["native-source.json"], parse_float=Decimal)
    engineering = json.loads(inputs["physical.json"])
    ownership = json.loads(inputs["ownership.json"])
    index = json.loads(inputs["native-export-manifest.json"])
    require(all(x["id"] == "SH017" for x in (resource, contract, export, engineering, ownership)), "Wrong SH017 identity")
    require(resource["name"] == contract["name"] == "Mole Prospector", "Changed hull name")
    info = contract["source"]
    require(info["geometryFile"] == export["sourcePath"] == "replacement-hulls/SH017-design.toml", "Changed original design path")
    require(info["geometrySHA256"] == export["sourceSHA256"] == engineering["detailedSourceSHA256"] == ownership["geometrySHA256"] == DESIGN_SHA, "Changed original design pin")
    require(engineering["contractSHA256"] == ownership["contractSHA256"] == pin(inputs["contract.toml"])["sha256"], "Changed engineering design binding")
    require(engineering["componentSourceSHA256"] == pin(inputs["ownership.json"])["sha256"], "Changed creator component binding")
    require(Fraction(engineering["metresPerSourceUnitExact"]) == SCALE and engineering["sourceOriginUnchanged"], "Changed adopted calibration")
    require(resource["nativeDesign"] == export["nativeDesign"], "Original native display and resource disagree")
    expected = {"id": "SH017", "file": "SH017.json", "sourcePath": info["geometryFile"], "sourceSHA256": DESIGN_SHA, **pin(inputs["native-source.json"])}
    require([r for r in index["models"] if r["id"] == "SH017"] == [expected], "SH017 native exporter does not bind the original")
    require(contract["geometry"]["nativeFaceLimit"] == 512 and contract["performance"]["maxSpeed"] == Decimal("29.0"), "Changed authored limit or speed")
    return resource, ownership, engineering


def local_piece_names(parts, variant, model):
    """Keep the two boring heads and each axial collar independently owned."""
    ordinary = Counter()
    last_collar_z = {}
    for part in parts:
        name = part["name"]
        if "/service-well-" in name:
            yield f'{variant}: {name.split("/recess-service-strut-",1)[0]}'
        elif name in {"boring-head structural support", "boring head", "boring head collar"}:
            pts = model["vertices"][part["vertexStart"]:part["vertexStart"]+part["vertexCount"]]
            xs = [p[0] for p in pts]
            require(max(xs)<0 or min(xs)>0, "Boring-head side crosses the original center")
            side = "negative X" if max(xs)<0 else "positive X"
            label = f"{name} ({side})"
            ordinal = ordinary[label]
            ordinary[label] += 1
            if name == "boring head collar":
                z = min(p[2] for p in pts)
                require(side not in last_collar_z or last_collar_z[side] < z, "Changed axial collar order")
                last_collar_z[side] = z
                yield f"{label} axial {ordinal}"
            else:
                require(ordinal == 0, f"Duplicate side-local structure: {label}")
                yield label
        else:
            require(ordinary[name] == 0, f"Unresolved repeated creator name: {name}")
            ordinary[name] += 1
            yield name
    for side in ("negative X","positive X"):
        require(ordinary[f"boring-head structural support ({side})"] == ordinary[f"boring head ({side})"] == 1, "Missing source-side head")
        require(ordinary[f"boring head collar ({side})"] == (4 if variant == "detailed" else 0), "Changed source collar count")


def partition(resource, ownership):
    slots = {}
    tables = []
    for model, variant in (("design","detailed"),("nativeDesign","nativeCandidate")):
        parts = ownership["models"][model]["parts"]
        memberships = {}
        for part, name in zip(parts, local_piece_names(parts,variant,resource[model]), strict=True):
            piece = slots.setdefault(name,len(slots))
            start, count = part["vertexStart"], part["vertexCount"]
            require(all(v not in memberships for v in range(start,start+count)), "Overlapping original component")
            memberships.update((v,piece) for v in range(start,start+count))
        tables.append(memberships)
    require(len(slots) == PIECE_COUNT, "Changed Mole Prospector local support count")
    return *tables,list(slots)


def stage_variant(model, owner, expected, pieces):
    vertex_count, face_count, component_count = expected
    vertices, faces = model["vertices"], model["faces"]
    require((len(vertices),len(faces),len(owner["parts"])) == expected, "Changed source topology")
    require(set(pieces) == set(range(vertex_count)), "Unassigned source vertices")
    require(model["paletteName"] == "slate-orange" and model["license"] == model["appearanceLicense"] == "MIT", "Changed original appearance")
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
    require(abs(bounds[1][2]-bounds[0][2]-46)<1e-12,"Changed adopted engineering geometry span")
    manifest = {
        "id":"SH017","legacyRevision":LEGACY_REVISION,"originalDesignSHA256":DESIGN_SHA,
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
    require(pin(outputs["MANIFEST.json"])["sha256"] == MANIFEST_SHA, "Changed SH017 manifest")
    if args.check:
        for name,blob in outputs.items():
            require((ASSETS/name).read_bytes()==blob,f"Changed SH017 encoded asset: {name}")
    else:
        ASSETS.mkdir(parents=True,exist_ok=True)
        for name,blob in outputs.items():
            (ASSETS/name).write_bytes(blob)
    print(f"SH017 own original geometry and {PIECE_COUNT} local supports verified ({len(outputs)} assets)")


if __name__=="__main__":
    main()
