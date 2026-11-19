"""Authenticate and pack the original Zebra with source-owned local supports."""

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
ASSETS = ROOT / "assets/fleet/sh021"
LEGACY_REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
PINS = {
    "resource.toml": ("Sources/L337Core/Resources/Ships/SH021.toml", "78b8ba0570e4869297e8878c98df5f76503d20136da9e9636309de894ec2394b"),
    "contract.toml": ("docs/fleet-design-v2/ships/SH021.toml", "49908a275fb7e5af7c0715b384e8dcc0a9072e453c94c73826d177fccf3a20ea"),
    "native-source.json": ("Sources/L337Core/Resources/OriginalFleetShowroomModels/SH021.json", "200d313cb6b0a2c603ce9e51cb778117808f6ab514945cd94a40350c2e657199"),
    "native-export-manifest.json": ("Sources/L337Core/Resources/OriginalFleetShowroomModels/manifest.json", "118b05757dc54096faa15dd79801f38999b87fe9424f1b3513133ead94f71971"),
    "physical.json": ("Sources/L337Core/Resources/OriginalFleetPhysical/SH021.json", "8b912a525dc3c5545fed1d7d47caf40a98183e56388b237174fcbfa62db9b63a"),
    "ownership.json": ("docs/milestones/fleet-physical/creator-ownership-records/SH021.json", "7cfd7a7ded55e219aecadf68c4f80118fd87ede55cccce35e3b3a0cbfedd73e6"),
    "native-candidate-blueprint.json": ("docs/milestones/fleet-physical/SH021-native-candidate-blueprint.json", "7d5c74bb63417ef99bb5ac658d04b817f9d721ff3c04c875a0afaf895cd29b90"),
    "native-candidate-recipe.py": ("docs/milestones/fleet-physical/SH021-native-candidate-recipe.py", "3904dab1b2a6c13c761591da04330248ee36bade478330ff5866e1f62e1c9e72"),
    "native-candidate-tests.py": ("docs/milestones/fleet-physical/SH021-native-candidate-tests.py", "ab874d413bb815c09d93975abdc87c1755955802a53e9b02821a656e748895bf"),
    "LICENSE": ("LICENSE", "033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766"),
}
DESIGN_SHA = "90942461424c81049c241869ac934058a53ae9f75cd494e9c50bebf10f6313e9"
MANIFEST_SHA = "4477d260f98b0e8eb8b584ae0d30698b795e21331a86a0898c5c9bcdc233b3d2"
SCALE = Fraction(40, 3)
COUNTS = ((1488, 2528, 112), (412, 744, 20))
PIECE_COUNT = 48
MAGIC = b"MSH021V1"
VISUAL_MAGIC = b"SH021VZ1"
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
        require(pin(inputs[name])["sha256"] == expected, f"Changed SH021 source: {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode(), parse_float=Decimal)
    contract = tomllib.loads(inputs["contract.toml"].decode(), parse_float=Decimal)
    export = json.loads(inputs["native-source.json"], parse_float=Decimal)
    engineering = json.loads(inputs["physical.json"])
    ownership = json.loads(inputs["ownership.json"])
    index = json.loads(inputs["native-export-manifest.json"])
    require(all(x["id"] == "SH021" for x in (resource, contract, export, engineering, ownership)), "Wrong SH021 identity")
    require(resource["name"] == contract["name"] == "Zebra", "Changed hull name")
    info = contract["source"]
    require(info["geometryFile"] == export["sourcePath"] == "replacement-hulls/SH021-design.toml", "Changed original design path")
    require(info["geometrySHA256"] == export["sourceSHA256"] == engineering["detailedSourceSHA256"] == ownership["geometrySHA256"] == DESIGN_SHA, "Changed original design pin")
    require(engineering["contractSHA256"] == ownership["contractSHA256"] == pin(inputs["contract.toml"])["sha256"], "Changed engineering design binding")
    require(engineering["componentSourceSHA256"] == pin(inputs["ownership.json"])["sha256"], "Changed creator component binding")
    require(Fraction(engineering["metresPerSourceUnitExact"]) == SCALE and engineering["sourceOriginUnchanged"], "Changed adopted calibration")
    require(resource["nativeDesign"] == export["nativeDesign"], "Original native display and resource disagree")
    expected = {"id": "SH021", "file": "SH021.json", "sourcePath": info["geometryFile"], "sourceSHA256": DESIGN_SHA, **pin(inputs["native-source.json"])}
    require([r for r in index["models"] if r["id"] == "SH021"] == [expected], "SH021 native exporter does not bind the original")
    require(contract["geometry"]["nativeFaceLimit"] == 512 and contract["performance"]["maxSpeed"] == Decimal("34.0"), "Changed authored limit or speed")
    proposal=json.loads(inputs["native-candidate-blueprint.json"])
    require(proposal["ship"]=="SH021" and proposal["status"]=="IMPLEMENTED_SOURCE_GENERATOR_UNEXECUTED_NOT_NATIVE_QUALIFIED","Changed separate native proposal status")
    require(proposal["r3ContractSHA256"]==pin(inputs["contract.toml"])["sha256"] and proposal["originalSource"]["sha256"]==DESIGN_SHA, "Changed separate native proposal binding")
    for key,name in (("currentResource","resource.toml"),("componentOwnership","ownership.json"),("recipe","native-candidate-recipe.py"),("tests","native-candidate-tests.py")):
        require(proposal[key]["sha256"]==pin(inputs[name])["sha256"],f"Changed proposal source binding: {name}")
    require(proposal["baseline"]["declaredNativeFaces"]==744 and proposal["baseline"]["limit"]==512 and proposal["baseline"]["installedResourceUnchanged"],"Changed proposed native cap")
    return resource, ownership, engineering


def local_piece_names(parts, variant, model):
    """Keep both freight keels and the two longitudinally distinct cross bridges."""
    ordinary=Counter()
    for part in parts:
        name=part["name"]
        if "/service-well-" in name:
            yield f'{variant}: {name.split("/recess-service-strut-",1)[0]}'
        elif name=="keel cross bridge":
            pts=model["vertices"][part["vertexStart"]:part["vertexStart"]+part["vertexCount"]]
            zs=[p[2] for p in pts]
            require(max(zs)<0 or min(zs)>0,"Keel cross bridge crosses the original centre")
            label=f"{name} ({'aft' if max(zs)<0 else 'forward'})"
            require(ordinary[label]==0,"Duplicate source-local bridge")
            ordinary[label]+=1
            yield label
        else:
            require(ordinary[name]==0,f"Unresolved repeated creator name: {name}")
            ordinary[name]+=1
            yield name
    require(ordinary["keel cross bridge (aft)"]==ordinary["keel cross bridge (forward)"]==1,"Missing original bridge")


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
    require(len(slots) == PIECE_COUNT, "Changed Zebra local support count")
    return *tables,list(slots)


def stage_variant(model, owner, expected, pieces):
    vertex_count, face_count, component_count = expected
    vertices, faces = model["vertices"], model["faces"]
    require((len(vertices),len(faces),len(owner["parts"])) == expected, "Changed source topology")
    require(set(pieces) == set(range(vertex_count)), "Unassigned source vertices")
    require(model["paletteName"] == "ice-blue" and model["license"] == model["appearanceLicense"] == "MIT", "Changed original appearance")
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
    require(abs(bounds[1][2]-bounds[0][2]-40)<1e-12,"Changed adopted engineering geometry span")
    manifest = {
        "id":"SH021","legacyRevision":LEGACY_REVISION,"originalDesignSHA256":DESIGN_SHA,
        "metresPerSourceUnit":str(SCALE),"axes":"+Z forward, +Y up; original origin",
        "detailed":{"vertices":COUNTS[0][0],"faces":COUNTS[0][1],"components":COUNTS[0][2],"boundsMetres":bounds[:2]},
        "nativeCandidate":{"vertices":COUNTS[1][0],"faces":COUNTS[1][1],"components":COUNTS[1][2],"boundsMetres":bounds[2:],"exactFaceLimit":512},
        "physicalAuthority":{"status":engineering["status"],"dryMassKg":engineering["dryMassKilograms"],"sha256":pin(inputs["physical.json"])["sha256"]},
        "collision":{"pieces":len(names),"roundingMetres":0.02,"scope":"Source-local rounded convex supports; corresponding authored structural parts share a support while separate original well mouths keep separate local supports. Native and ordinary admission pending."},
        "legacyInputs":{n:{"path":p,**pin(inputs[n])} for n,(p,_) in PINS.items()},
        "encoded":{name:pin(data) for name,data in encoded.items()},"sharedTextures":textures(resource),
    }
    manifest["nativeAlternative"]={"status":"IMPLEMENTED_SOURCE_GENERATOR_UNEXECUTED_NOT_NATIVE_QUALIFIED","installedResourceUnchanged":True,"scope":"Separate original 20-part source generator proposal retained; not executed or selected by this port."}
    encoded["MANIFEST.json"] = json_bytes(manifest)
    return encoded


def read_legacy_inputs(repository: Path) -> dict[str, bytes]:
    repo=repository.resolve(strict=True)
    return {
        name:subprocess.check_output(["git","cat-file","blob",f"{LEGACY_REVISION}:{path}"],cwd=repo)
        for name,(path,_) in PINS.items()
    }


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
        inputs=read_legacy_inputs(args.legacy_repo)
    outputs={**inputs,**build(inputs)}
    require(pin(outputs["MANIFEST.json"])["sha256"] == MANIFEST_SHA, "Changed SH021 manifest")
    if args.check:
        for name,blob in outputs.items():
            require((ASSETS/name).read_bytes()==blob,f"Changed SH021 encoded asset: {name}")
    else:
        ASSETS.mkdir(parents=True,exist_ok=True)
        for name,blob in outputs.items():
            (ASSETS/name).write_bytes(blob)
    print(f"SH021 own original geometry and {PIECE_COUNT} local supports verified ({len(outputs)} assets)")


if __name__=="__main__":
    main()
