"""Authenticate and pack the original Peregrine Mk I with source-owned local supports."""

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
ASSETS = ROOT / "assets/fleet/sh022"
LEGACY_REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
PINS = {
    "resource.toml": ("Sources/L337Core/Resources/Ships/SH022.toml", "a4e2830603cded5e8c2435819dcd554b1475096f0b519d2bdcdfd5da3473034d"),
    "contract.toml": ("docs/fleet-design-v2/ships/SH022.toml", "f8502a924dca78b9274ae9ad282cb3d9636ce094087a2ab4365aae7f733a736d"),
    "native-source.json": ("Sources/L337Core/Resources/OriginalFleetShowroomModels/SH022.json", "86a27d80227e5f6552439a0fa4956dd60b3322d6d8c84984dab3fd42d6de7d73"),
    "native-export-manifest.json": ("Sources/L337Core/Resources/OriginalFleetShowroomModels/manifest.json", "118b05757dc54096faa15dd79801f38999b87fe9424f1b3513133ead94f71971"),
    "physical.json": ("Sources/L337Core/Resources/OriginalFleetPhysical/SH022.json", "85b41b972c2c274bdcd2c822d5da04ff611c159b2576986196a6d1ff9e01f51e"),
    "ownership.json": ("docs/milestones/fleet-physical/creator-ownership-records/SH022.json", "a538a93634755431ca49b780f8e7d5f6485c9cfb13786130aaf82cf19a1ed5c6"),
    "LICENSE": ("LICENSE", "033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766"),
}
CANDIDATE_INPUT_PINS = {
    "peerBlueprint": ("docs/milestones/fleet-physical/peregrine-peer-native-candidate-blueprint.json","cb7fbb763eb56da8c7651b11b705759baea74eb4b94e1560d70fbaff86e0cc70"),
    "qualificationInputs": ("docs/milestones/fleet-physical/actual55-candidate-qualification-inputs.json","2847c68d4328956b776cf89217b8b42c35990b4b0fea87bac39247fafe3dc01f"),
}
CANDIDATE_STATUS_SHA = "08e50790a1e2673d536025f94fa129c7a769375f73b1b73604ae2ea2498c1615"
CANDIDATE_RECEIPT = "NATIVE-CANDIDATE-STATUS.json"
DESIGN_SHA = "60cd55b2c752e53200a950ad9481c6f49bd6e9eed228071abb2d8794ed0a6979"
MANIFEST_SHA = "b7c4ed44b129ec709ca79001f5b425daf1c98ddee0c13859a3a38f47bb791f6e"
SCALE = Fraction(800, 59)
COUNTS = ((2072, 3484, 165), (536, 964, 27))
PIECE_COUNT = 69
MAGIC = b"MSH022V1"
VISUAL_MAGIC = b"SH022VZ1"
TEXTURE_SOURCE_SHA = "96c4c9699b81f50be5a8aefeeca74a6e4870e0bb981d1cd33a9ff2921cd2bd3c"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def pin(raw):
    return {"bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def json_bytes(value):
    return (json.dumps(value, indent=2, allow_nan=False) + "\n").encode()


def candidate_status(raw):
    """Retain only SH022's evidence from the shared frozen peer and 55-input records."""
    for name,(_,digest) in CANDIDATE_INPUT_PINS.items():
        require(pin(raw[name])["sha256"]==digest,f"Changed SH022 candidate input: {name}")
    blueprint=json.loads(raw["peerBlueprint"])
    qualification=json.loads(raw["qualificationInputs"])
    peers=[row for row in blueprint["entries"] if row["id"]=="SH022"]
    rows=[row for row in qualification["rows"] if row["id"]=="SH022"]
    require(len(peers)==len(rows)==1,"Missing exact SH022 candidate record")
    peer,row=peers[0],rows[0]
    require(peer["r3ContractSHA256"]==row["r3SHA256"]==PINS["contract.toml"][1] and peer["originalSource"]["sha256"]==row["originalSource"]["sha256"]==DESIGN_SHA,"Changed separate SH022 candidate identity")
    require(peer["currentResource"]["sha256"]==row["currentResource"]["sha256"]==PINS["resource.toml"][1] and peer["componentOwnership"]["sha256"]==PINS["ownership.json"][1],"Changed separate original binding")
    require(peer["declaredOriginalNativeFaces"]==964 and peer["originalComponents"]==27 and blueprint["limit"]==qualification["limits"]["nativeTriangleLimit"]==512,"Changed reported native limit")
    require(blueprint["status"]=="IMPLEMENTED_SOURCE_GENERATOR_UNEXECUTED_NOT_NATIVE_QUALIFIED" and qualification["status"]=="INPUTS_FOR_G1_ONLY_NOT_FIDELITY_NATIVE_PHYSICAL_OR_RUNTIME_QUALIFICATION","Changed candidate qualification boundary")
    require(row["physicalRecord"]["sha256"]==PINS["physical.json"][1] and row["blueprint"]["sha256"]==CANDIDATE_INPUT_PINS["peerBlueprint"][1] and row["criteria"]["continuousFidelityStatus"]=="UNISSUED_GENERAL_METRIC_AND_TOLERANCE_NO_AUTOMATIC_PASS","Changed candidate physical/fidelity scope")
    return json_bytes({
        "schema":"meridian-original-candidate-status/1","hull":"SH022","legacyRevision":LEGACY_REVISION,
        "parentInputs":{name:{"path":path,**pin(raw[name])} for name,(path,_) in CANDIDATE_INPUT_PINS.items()},
        "peerProposal":{"id":blueprint["id"],"status":blueprint["status"],"entryStatus":peer["status"],"reportedOriginalNativeFaces":peer["declaredOriginalNativeFaces"],"originalComponents":peer["originalComponents"],"exactFaceLimit":blueprint["limit"],"recipe":row["recipe"],"tests":row["tests"]},
        "laterQualificationInputs":{"status":qualification["status"],"sourceCommit":qualification["sourceCommit"],"actualRunHead":qualification["actualRunHead"],"actualRunSession":qualification["actualRunSession"],"cache":row["cache"],"sampledPartIndices":row["parts"]["sampledIndices"],"fixedPartIndices":row["parts"]["exactFixedIndices"],"continuousFidelityStatus":row["criteria"]["continuousFidelityStatus"],"adoptionRoute":row["adoptionRoute"],"candidateFaceCount":None},
        "scope":"The later record identifies a cached input; the candidate geometry bytes and actual count are not in this frozen Git tree. Neither proposal nor cached input was adopted or qualified here. Only the original 964-face packaged source is used."
    })


def authenticated(inputs):
    for name, (_, expected) in PINS.items():
        require(pin(inputs[name])["sha256"] == expected, f"Changed SH022 source: {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode(), parse_float=Decimal)
    contract = tomllib.loads(inputs["contract.toml"].decode(), parse_float=Decimal)
    export = json.loads(inputs["native-source.json"], parse_float=Decimal)
    engineering = json.loads(inputs["physical.json"])
    ownership = json.loads(inputs["ownership.json"])
    index = json.loads(inputs["native-export-manifest.json"])
    require(all(x["id"] == "SH022" for x in (resource, contract, export, engineering, ownership)), "Wrong SH022 identity")
    require(resource["name"] == contract["name"] == "Peregrine Mk I", "Changed hull name")
    info = contract["source"]
    require(info["geometryFile"] == export["sourcePath"] == "replacement-hulls/SH022-design.toml", "Changed original design path")
    require(info["geometrySHA256"] == export["sourceSHA256"] == engineering["detailedSourceSHA256"] == ownership["geometrySHA256"] == DESIGN_SHA, "Changed original design pin")
    require(engineering["contractSHA256"] == ownership["contractSHA256"] == pin(inputs["contract.toml"])["sha256"], "Changed engineering design binding")
    require(engineering["componentSourceSHA256"] == pin(inputs["ownership.json"])["sha256"], "Changed creator component binding")
    require(Fraction(engineering["metresPerSourceUnitExact"]) == SCALE and engineering["sourceOriginUnchanged"], "Changed adopted calibration")
    require(resource["nativeDesign"] == export["nativeDesign"], "Original native display and resource disagree")
    expected = {"id": "SH022", "file": "SH022.json", "sourcePath": info["geometryFile"], "sourceSHA256": DESIGN_SHA, **pin(inputs["native-source.json"])}
    require([r for r in index["models"] if r["id"] == "SH022"] == [expected], "SH022 native exporter does not bind the original")
    require(contract["geometry"]["nativeFaceLimit"] == 512 and contract["performance"]["maxSpeed"] == Decimal("37.0"), "Changed authored limit or speed")
    require(pin(inputs[CANDIDATE_RECEIPT])["sha256"]==CANDIDATE_STATUS_SHA,"Changed retained SH022 candidate receipt")
    return resource, ownership, engineering


def local_piece_names(parts,variant,model):
    """Keep both hooked longitudinal members independent and wells source-local."""
    ordinary=Counter()
    for part in parts:
        name=part["name"]
        if "/service-well-" in name:
            yield f'{variant}: {name.split("/recess-service-strut-",1)[0]}'
        elif name=="fork longitudinal envelope":
            pts=model["vertices"][part["vertexStart"]:part["vertexStart"]+part["vertexCount"]]
            xs=[p[0] for p in pts]
            require(max(xs)<0 or min(xs)>0,"Fork member crosses the original centre")
            label=f"{name} ({'negative X' if max(xs)<0 else 'positive X'})"
            require(ordinary[label]==0,"Duplicate source-local fork")
            ordinary[label]+=1
            yield label
        else:
            require(ordinary[name]==0,f"Unresolved repeated creator name: {name}")
            ordinary[name]+=1
            yield name
    require(ordinary["fork longitudinal envelope (negative X)"]==ordinary["fork longitudinal envelope (positive X)"]==1,"Missing original fork")


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
    require(len(slots) == PIECE_COUNT, "Changed Peregrine Mk I local support count")
    return *tables,list(slots)


def stage_variant(model, owner, expected, pieces):
    vertex_count, face_count, component_count = expected
    vertices, faces = model["vertices"], model["faces"]
    require((len(vertices),len(faces),len(owner["parts"])) == expected, "Changed source topology")
    require(set(pieces) == set(range(vertex_count)), "Unassigned source vertices")
    require(model["paletteName"] == "cobalt-cream" and model["license"] == model["appearanceLicense"] == "MIT", "Changed original appearance")
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
        "id":"SH022","legacyRevision":LEGACY_REVISION,"originalDesignSHA256":DESIGN_SHA,
        "metresPerSourceUnit":str(SCALE),"axes":"+Z forward, +Y up; original origin",
        "detailed":{"vertices":COUNTS[0][0],"faces":COUNTS[0][1],"components":COUNTS[0][2],"boundsMetres":bounds[:2]},
        "nativeCandidate":{"vertices":COUNTS[1][0],"faces":COUNTS[1][1],"components":COUNTS[1][2],"boundsMetres":bounds[2:],"exactFaceLimit":512},
        "physicalAuthority":{"status":engineering["status"],"dryMassKg":engineering["dryMassKilograms"],"sha256":pin(inputs["physical.json"])["sha256"]},
        "collision":{"pieces":len(names),"roundingMetres":0.02,"scope":"Source-local rounded convex supports; corresponding authored structural parts share a support while separate original well mouths keep separate local supports. Native and ordinary admission pending."},
        "legacyInputs":{n:{"path":p,**pin(inputs[n])} for n,(p,_) in PINS.items()},
        "encoded":{name:pin(data) for name,data in encoded.items()},"sharedTextures":textures(resource),
    }
    manifest["separateNativeCandidate"]={"receipt":CANDIDATE_RECEIPT,**pin(inputs[CANDIDATE_RECEIPT]),"status":"CACHED_INPUT_UNQUALIFIED_NOT_SELECTED"}
    encoded["MANIFEST.json"] = json_bytes(manifest)
    return encoded


def read_pinned_blobs(repository: Path, pins) -> dict[str, bytes]:
    repo=repository.resolve(strict=True)
    return {name:subprocess.check_output(["git","cat-file","blob",f"{LEGACY_REVISION}:{path}"],cwd=repo) for name,(path,_) in pins.items()}


def read_legacy_inputs(repository: Path) -> dict[str, bytes]:
    return read_pinned_blobs(repository,PINS)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--legacy-repo",type=Path)
    parser.add_argument("--check",action="store_true")
    args=parser.parse_args()
    if args.check:
        inputs={name:(ASSETS/name).read_bytes() for name in (*PINS,CANDIDATE_RECEIPT)}
    else:
        if args.legacy_repo is None:
            parser.error("--legacy-repo is required to stage immutable originals")
        inputs=read_legacy_inputs(args.legacy_repo)
        inputs[CANDIDATE_RECEIPT]=candidate_status(read_pinned_blobs(args.legacy_repo,CANDIDATE_INPUT_PINS))
    outputs={**inputs,**build(inputs)}
    require(pin(outputs["MANIFEST.json"])["sha256"]==MANIFEST_SHA,"Changed SH022 manifest")
    if args.check:
        for name,blob in outputs.items():
            require((ASSETS/name).read_bytes()==blob,f"Changed SH022 encoded asset: {name}")
    else:
        ASSETS.mkdir(parents=True,exist_ok=True)
        for name,blob in outputs.items():
            (ASSETS/name).write_bytes(blob)
    print(f"SH022 own original geometry and {PIECE_COUNT} local supports verified ({len(outputs)} assets)")


if __name__=="__main__":
    main()
