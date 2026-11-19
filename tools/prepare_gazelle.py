"""Authenticate SH019 authored-metre geometry and sector-local rounded supports."""

import argparse
from collections import defaultdict
from decimal import Decimal
import hashlib
import json
import math
from pathlib import Path
import struct
import subprocess
import tomllib

from encode_goshawk_visual import encode_model
from import_goshawk_envelope_source import components

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets/fleet/sh019"
LEGACY_REVISION = "a11fe16924d87d06021816f9037952afcbac08db"
PINS = {
    "resource.toml": ("Sources/L337Core/Resources/Ships/SH019.toml", "b37924110f28d2b0d541af5b7de2feb7c386254350afe6f4a8c162af471169ac"),
    "contract.toml": ("docs/fleet-design-v2/ships/SH019.toml", "9eb70cd7503964f5613f86cd6ceec2cf34f64598e8018779380fc6eca5b08a04"),
    "native-source.json": ("Sources/L337Core/Resources/OriginalFleetShowroomModels/SH019.json", "664c9a5ad747c60982d4f0bf460a3dbd69e286ce47c733a25d8a335cfec6fa6f"),
    "native-export-manifest.json": ("Sources/L337Core/Resources/OriginalFleetShowroomModels/manifest.json", "118b05757dc54096faa15dd79801f38999b87fe9424f1b3513133ead94f71971"),
    "legacy-activation.swift": ("Sources/L337Core/OriginalGazelleExplorerActivation.swift", "ed92011aebc8482c578a67e316da4aded15828b102e51bfd135a97b1ecd08389"),
    "legacy-physical-contract.swift": ("Sources/L337Core/OriginalHullPhysicalContract.swift", "2c68d8d51acdd469faa25124bcc19027aec1a8f747eb267da2e84e5a98e3c027"),
    "legacy-source-tests.swift": ("Tests/L337CoreTests/OriginalGazelleExplorerUnitTests.swift", "8fe0010a4985eff0c862b6aa974ec3d9b219bb89f30f106ff0f7c1642c88e7e6"),
    "legacy-appearance.swift": ("Sources/L337Core/ShipAppearance.swift", "629e75651da678609f1092fb0f287f48d1718331066c29a81626c1e660fadbb5"),
    "legacy-registrations.swift": ("Sources/L337Core/OriginalFleetPhysicalRegistrations.swift", "5ec9cd96763e0cf96783052127a0d0ec05a2a9155fd0289d3f8e25682510c0fa"),
    "legacy-baseline-registrations.swift": ("Sources/L337Core/OriginalFleetBaselinePhysicalRegistrations.swift", "7264c4af506629f5559fda999267d78376aaaaed6d27cb7325551a45e25da8e3"),
    "legacy-relative-registrations.swift": ("Sources/L337Core/OriginalFleetRelativePhysicalRegistrations.swift", "f27bb348cd77faa18eb2c01c71afbf503f2e240ce974fec59036447eef70017d"),
    "legacy-larvaal-registrations.swift": ("Sources/L337Core/OriginalFleetLarvaalPhysicalRegistrations.swift", "560232d01d8954477df164531b0ed3595caca3657d77c0e99cc6bd65353d8001"),
    "LICENSE": ("LICENSE", "033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766"),
}
DESIGN_SHA = "56172839f14cc1aca09814de34d181b98832e87e071130d0c4c930a760d66944"
MANIFEST_SHA = "23cc2f3e72c5ae4142046c0b64cc357ec46a4924dc7f5dd4899b71cad2fe67c5"
SCALE = 1
COUNTS = ((3008, 5900, 30), (172, 304, 11))
SUPPORT_COUNTS = (3104, 204)
PIECE_COUNT = 37
MAGIC = b"MSH019V1"
SUPPORT_MAGIC = b"MSH019S1"
VISUAL_MAGIC = b"SH019VZ1"
TEXTURE_SOURCE_SHA = "96c4c9699b81f50be5a8aefeeca74a6e4870e0bb981d1cd33a9ff2921cd2bd3c"

DETAILED_NAMES = [
    "pressure body", "ventral telescope keel", "elliptical service loop",
    "negative loop support", "negative survey float", "negative engine casing",
    "negative engine detail", "negative engine recess", "positive loop support",
    "positive survey float", "positive engine casing", "positive engine detail",
    "positive engine recess", "upper glazing envelope", "upper glazing structural member",
    "forward telescope head",
] + [f"{side} upper local feature {index}" for side in ("negative", "positive") for index in range(7)]
NATIVE_COMPONENT_TO_DETAILED = (0,1,2,3,4,5,8,9,10,13,15)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def pin(raw):
    return {"bytes":len(raw),"sha256":hashlib.sha256(raw).hexdigest()}


def json_bytes(value):
    return (json.dumps(value,indent=2,allow_nan=False)+"\n").encode()


def authenticated(inputs):
    for name,(_,digest) in PINS.items():
        require(pin(inputs[name])["sha256"]==digest,f"Changed SH019 source: {name}")
    resource = tomllib.loads(inputs["resource.toml"].decode(),parse_float=Decimal)
    contract = tomllib.loads(inputs["contract.toml"].decode(),parse_float=Decimal)
    native = json.loads(inputs["native-source.json"],parse_float=Decimal)
    index = json.loads(inputs["native-export-manifest.json"])
    require(resource["id"]==contract["id"]==native["id"]=="SH019" and resource["name"]==contract["name"]=="Gazelle Explorer","Wrong SH019 identity")
    source=contract["source"]
    require(source["geometryFile"]==native["sourcePath"]=="fleet-completion-20261019/batch-01-native-lod/SH019-design.toml","Changed original native path")
    require(source["geometrySHA256"]==native["sourceSHA256"]==DESIGN_SHA and native["coordinateUnits"]=="metres","Changed original metre calibration")
    require(resource["nativeDesign"]==native["nativeDesign"],"Original native export and resource disagree")
    expected={"id":"SH019","file":"SH019.json","sourcePath":source["geometryFile"],"sourceSHA256":DESIGN_SHA,**pin(inputs["native-source.json"])}
    require([r for r in index["models"] if r["id"]=="SH019"]==[expected],"SH019 exporter identity changed")
    require(contract["geometry"]["nativeFaceLimit"]==512 and contract["performance"]["maxSpeed"]==Decimal("43.0"),"Changed source limit or speed")
    activation=inputs["legacy-activation.swift"].decode()
    physical=inputs["legacy-physical-contract.swift"].decode()
    calibration=physical.split("private static func calibrationCase6(",1)[1].split("private static func calibrationCase7(",1)[0]
    require('id = "SH019"' in activation and 'physical.metresPerSourceUnit == 1' in activation and 'physical.model.coordinateUnits == "metres"' in activation,"Changed approved Gazelle activation")
    require('metresPerSourceUnit = 1' in calibration and DESIGN_SHA in calibration and PINS["contract.toml"][1] in calibration,"Changed approved independent calibration")
    require(b'guard mapping.texture != -1 else { return nil }' in inputs["legacy-appearance.swift"],"Changed original no-map appearance semantics")
    require(all(b'"SH019"' not in raw for name,raw in inputs.items() if name.endswith("registrations.swift")),"Unexpected registered SH019 engineering")
    require(all(x.encode() in inputs["legacy-registrations.swift"] for x in ("declaredMetreRegistrations","relativeRegistrations","larvaalMetreRegistrations","baselineRegistrations")),"Changed physical registration authorities")
    return resource


def piece_labels():
    return DETAILED_NAMES[:2]+[f"elliptical service loop sector {i}" for i in range(8)]+DETAILED_NAMES[3:]


def component_piece(detailed_index):
    require(detailed_index!=2,"The torus needs a sector, not a full convex hull")
    return detailed_index if detailed_index<2 else detailed_index+7


def loop_faces(start,major,tube):
    """Reconstruct the authenticated ring adjacency, including the wrapped seam."""
    for ring in range(major):
        following=(ring+1)%major
        for i in range(tube):
            j=(i+1)%tube
            a,b,c,d=(start+ring*tube+i,start+following*tube+i,start+following*tube+j,start+ring*tube+j)
            yield [a,b,c]
            yield [a,c,d]


def stage_variant(model,variant,expected):
    count,face_count,component_count=expected
    points=[tuple(float(x) for x in p) for p in model["vertices"]]
    faces=model["faces"]
    require(len(points)==count and len(faces)==face_count,"Changed original topology")
    require(model["paletteName"]=="ice-cobalt" and model["license"]==model["appearanceLicense"]=="MIT","Changed original appearance")
    require(all(len(p)==3 and all(math.isfinite(x) for x in p) for p in points),"Invalid source coordinates")
    require(all(len(f)==3 and len(set(f))==3 and all(type(i) is int and 0<=i<count for i in f) for f in faces),"Invalid source triangle")
    untextured=[m for m,t in zip(model["faceMaterials"],model["faceTextures"],strict=True) if t["texture"]==-1]
    require((untextured.count(5),untextured.count(6),len(untextured))==((960,256,1216) if variant=="detailed" else (8,8,16)),"Changed original untextured glazing or emitter faces")
    require(all(t["texture"] in (-1,0,1,2) for t in model["faceTextures"]),"Invalid original material map")
    groups=components(model)
    require(len(groups)==component_count,"Changed connected component count")
    vertex_component={v:i for i,g in enumerate(groups) for v in g}
    by_component=defaultdict(list)
    for i,face in enumerate(faces):
        group=vertex_component[face[0]]
        require(all(vertex_component[v]==group for v in face),"Invalid original component")
        by_component[group].append(i)
    assignments=[None]*face_count
    rows=[]
    for i,group in enumerate(groups):
        detailed_index=i if variant=="detailed" else NATIVE_COMPONENT_TO_DETAILED[i]
        ids=by_component[i]
        require(group==list(range(group[0],group[0]+len(group))) and ids==list(range(ids[0],ids[0]+len(ids))),"Noncontiguous authenticated component")
        if detailed_index==2:
            major,tube=(64,12) if variant=="detailed" else (8,4)
            require(len(group)==major*tube and [faces[j] for j in ids]==list(loop_faces(group[0],major,tube)),"Changed elliptical loop adjacency or winding")
            for local,j in enumerate(ids):
                assignments[j]=2+(local//(2*tube))//(major//8)
        else:
            for j in ids:
                assignments[j]=component_piece(detailed_index)
        rows.append({"component":i,"derivedLabel":DETAILED_NAMES[detailed_index],"startVertex":group[0],"vertexCount":len(group),"startFace":ids[0],"faceCount":len(ids),"pieces":sorted({assignments[j] for j in ids})})
    require(all(x is not None for x in assignments),"Unassigned original faces")
    per_piece=defaultdict(set)
    for face,piece in zip(faces,assignments,strict=True):
        per_piece[piece].update(face)
    require(set().union(*per_piece.values())==set(range(count)),"Unassigned source vertices")
    # Only the shared loop boundaries duplicate source indices in the support stream.
    counts=defaultdict(int)
    for indices in per_piece.values():
        for i in indices:
            counts[i]+=1
    loop=groups[2]; major,tube=(64,12) if variant=="detailed" else (8,4)
    seam={loop[0]+ring*tube+i for ring in range(0,major,major//8) for i in range(tube)}
    require({i for i,c in counts.items() if c!=1}==seam and all(counts[i]==2 for i in seam),"Changed duplicated loop seam")
    support=[(piece,i) for piece,indices in sorted(per_piece.items()) for i in sorted(indices)]
    return points,faces,assignments,support,rows


def textures(resource):
    raw=(ROOT/"assets/fleet/sh001/TEXTURE-SOURCE.json").read_bytes()
    require(pin(raw)["sha256"]==TEXTURE_SOURCE_SHA,"Changed shared texture manifest")
    by_id={asset["id"]:asset for asset in json.loads(raw)["assets"]}
    result={}
    for texture in resource["design"]["textures"]:
        row=by_id[texture["asset"]]; name=row["rgbaFile"]; path=f"assets/fleet/sh001/{name}"
        expect={"bytes":row["width"]*row["height"]*4,"sha256":row["rgbaSHA256"]}
        require(pin((ROOT/path).read_bytes())==expect,f"Changed original texture: {path}")
        result[path]={**expect,"source":f"Sources/L337Core/Resources/OriginalShipTextures/{name}","license":row["license"]}
    return result


def build(inputs):
    resource=authenticated(inputs)
    models=[stage_variant(resource[key],variant,counts) for key,variant,counts in (("design","detailed",COUNTS[0]),("nativeDesign","nativeCandidate",COUNTS[1]))]
    labels=piece_labels(); require(len(labels)==PIECE_COUNT,"Changed piece labels")
    vertices=bytearray(struct.pack("<8sII",MAGIC,COUNTS[0][0],COUNTS[1][0])); triangles=bytearray(); face_members=bytearray()
    support=bytearray(struct.pack("<8sII",SUPPORT_MAGIC,*SUPPORT_COUNTS)); source_ids=bytearray(); support_members=bytearray()
    for variant,(points,faces,assignments,local,_) in enumerate(models):
        offset=0 if variant==0 else COUNTS[0][0]
        require(len(local)==SUPPORT_COUNTS[variant],"Changed support duplication count")
        vertices.extend(b"".join(struct.pack("<3d",*p) for p in points))
        triangles.extend(b"".join(struct.pack("<3I",*(offset+i for i in face)) for face in faces))
        face_members.extend(assignments)
        for piece,index in local:
            support.extend(struct.pack("<3d",*points[index])); source_ids.extend(struct.pack("<I",offset+index)); support_members.append(piece)
    encoded={"source.vertices":bytes(vertices),"source-triangles.bin":bytes(triangles),"source-face-membership.bin":bytes(face_members),"support.vertices":bytes(support),"support-source-indices.bin":bytes(source_ids),"support-piece-membership.bin":bytes(support_members),"COMPONENT-PIECES.json":json_bytes({"labelScope":"Derived geometric labels; no creator-ownership record is present in the frozen source.","pieces":labels,"detailed":models[0][4],"nativeCandidate":models[1][4]})}
    encoded["visual.bin"]=encode_model(dict(resource["design"],vertices=models[0][0]),vertices,triangles,MAGIC,(COUNTS[0][0],COUNTS[1][0],COUNTS[0][1]),VISUAL_MAGIC,allow_untextured=True)
    bounds=[[fn(axis) for axis in zip(*model[0])] for model in models for fn in (min,max)]
    require(bounds==[[-11.3,-6.9,-16.79],[11.3,6.9,21.5]]*2,"Changed authored metre dimensions")
    manifest={"id":"SH019","legacyRevision":LEGACY_REVISION,"originalDesignSHA256":DESIGN_SHA,"metresPerSourceUnit":"1","axes":"+Z forward, +Y up; original origin","detailed":{"vertices":COUNTS[0][0],"faces":COUNTS[0][1],"components":COUNTS[0][2],"boundsMetres":bounds[:2]},"nativeCandidate":{"vertices":COUNTS[1][0],"faces":COUNTS[1][1],"components":COUNTS[1][2],"boundsMetres":bounds[2:],"exactFaceLimit":512,"sourceStatus":"Approved authored metre native LOD; no Meridian native qualification claimed."},"physicalAuthority":{"status":"AUTHORED_METRE_GEOMETRY_ONLY_NO_REGISTERED_ENGINEERING","dryMassKg":None,"centreOfMassMetres":None,"inertiaKgM2":None},"collision":{"pieces":len(labels),"roundingMetres":0.02,"supportPoints":list(SUPPORT_COUNTS),"scope":"Source-local rounded convex supports. Eight separate loop sectors duplicate only canonical seam points in support stream; original geometry/indices unchanged."},"legacyInputs":{name:{"path":path,**pin(inputs[name])} for name,(path,_) in PINS.items()},"encoded":{name:pin(data) for name,data in encoded.items()},"sharedTextures":textures(resource)}
    manifest["appearance"]={"detailedUntexturedFaces":1216,"nativeUntexturedFaces":16,"originalNoMap":-1,"packedNoMapSentinel":3,"scope":"Source glazing and emitter face colour without a diffuse map; source UV fields retained."}
    encoded["MANIFEST.json"]=json_bytes(manifest)
    return encoded


def read_legacy_inputs(repository: Path) -> dict[str, bytes]:
    repo=repository.resolve(strict=True)
    return {
        name:subprocess.check_output(["git","cat-file","blob",f"{LEGACY_REVISION}:{path}"],cwd=repo)
        for name,(path,_) in PINS.items()
    }


def main():
    parser=argparse.ArgumentParser(description=__doc__); parser.add_argument("--legacy-repo",type=Path); parser.add_argument("--check",action="store_true"); args=parser.parse_args()
    if args.check:
        inputs={name:(ASSETS/name).read_bytes() for name in PINS}
    else:
        if args.legacy_repo is None: parser.error("--legacy-repo is required to stage immutable originals")
        inputs=read_legacy_inputs(args.legacy_repo)
    outputs={**inputs,**build(inputs)}
    require(pin(outputs["MANIFEST.json"])["sha256"]==MANIFEST_SHA,"Changed SH019 manifest")
    if args.check:
        for name,blob in outputs.items(): require((ASSETS/name).read_bytes()==blob,f"Changed SH019 encoded asset: {name}")
    else:
        ASSETS.mkdir(parents=True,exist_ok=True)
        for name,blob in outputs.items(): (ASSETS/name).write_bytes(blob)
    print(f"SH019 canonical geometry and {PIECE_COUNT} local supports verified ({len(outputs)} assets)")


if __name__=="__main__":
    main()
