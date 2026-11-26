"""Measure the generated Griffon Spectre collision artifact against both original meshes."""
import argparse
import struct
import sys
import json
import tomllib

import numpy as np
import trimesh

from banteng_art_mesh import assemble
from prepare_griffon_spectre_geometry import INTEGRATION, ORIGINAL, build, metre_space, require
from measure_banteng_geometry import adaptive_upper, section, solid, upper
from measure_lanner_geometry import make_fan
from measure_seriema_courier_geometry import silhouette
from vulture_mk_i_exact_fit import grid_index



def mesh(points, faces):
    return trimesh.Trimesh(np.array(points), faces, process=False)


def load():
    raw = tomllib.loads((ORIGINAL/"resource.toml").read_text())
    ownership = json.loads((INTEGRATION/"ORIGINAL-OWNERSHIP.json").read_bytes())["models"]
    candidate = json.loads((ORIGINAL/"GENERATED-COLLISION.json").read_bytes())
    source = {
        "detailed": (metre_space(raw["design"]), ownership["design"]["parts"]),
        "archiveNative": (metre_space(raw["nativeDesign"]), ownership["nativeDesign"]["parts"]),
        "candidateNative": (metre_space(candidate["nativeCandidate"]), candidate["parts"]),
    }
    return source


def fidelity(source):
    detailed, _ = source["detailed"]
    archival, old_parts = source["archiveNative"]
    candidate, new_parts = source["candidateNative"]
    references = {label: mesh(raw["vertices"],raw["faces"]) for label,(raw,_) in source.items()}
    indices = {name:grid_index(surface) for name,surface in references.items()}
    rows=[]
    for i,(old_part,new_part) in enumerate(zip(old_parts,new_parts,strict=True)):
        assert old_part["name"]==new_part["name"]
        old=section(archival,old_part)
        current=section(candidate,new_part)
        solid(mesh(*old[:2]))
        solid(mesh(*current[:2]))
        assert set(old[2])==set(current[2])
        equal=old==current
        measured={"candidateToArchive":0,"archiveToCandidate":0} if equal else {
            "candidateToArchive":upper(mesh(*current[:2]),indices["archiveNative"]),
            "archiveToCandidate":upper(mesh(*old[:2]),indices["candidateNative"]),
        }
        rows.append({"partIndex":i,"name":old_part["name"],"exactVerticesFacesMaterialsAndUV":equal,
                     "boundsMillimetres":measured})
    return {
        "exactParts":sum(row["exactVerticesFacesMaterialsAndUV"] for row in rows),
        "allParts":len(rows),
        "silhouette":{"directionsPerOrthographicView":16,"againstArchivedNative":silhouette(archival,candidate),
                       "againstDetailed":silhouette(detailed,candidate)},
        "fullUpperMillimetres":{
            name: max(row["boundsMillimetres"][name] for row in rows)
            for name in ("candidateToArchive","archiveToCandidate")
        } | {
            "candidateToDetailed":upper(references["candidateNative"],indices["detailed"]),
            "detailedToCandidate":upper(references["detailed"],indices["candidateNative"]),
        },
        "archivedOriginalComparisonUpperMillimetres": {
            "archiveNativeToDetailed":upper(references["archiveNative"],indices["detailed"]),
            "detailedToArchivedNative":upper(references["detailed"],indices["archiveNative"]),
        },
        "parts":rows,
    }


def support_fit(source,use_candidate):
    detailed=source["detailed"]
    low=source["candidateNative" if use_candidate else "archiveNative"]
    slots,meshes=assemble(detailed,low)
    points = np.array(meshes[0]["vertices"] + meshes[1]["vertices"])
    member = np.array(meshes[0]["groups"] + meshes[1]["groups"])
    if use_candidate:
        compiled = build()
        for name in ("source.vertices", "source-piece-membership.bin"):
            require((ORIGINAL / name).read_bytes() == compiled[name], f"changed Griffon Spectre fit input {name}")
        bundled = np.array(list(struct.iter_unpack("<3d", compiled["source.vertices"][16:])))
        membership = np.frombuffer(compiled["source-piece-membership.bin"], dtype="u1")
        require(np.array_equal(points, bundled) and np.array_equal(member, membership),
                "Griffon Spectre fit differs from bundled collision geometry")
        points, member = bundled, membership
    visible=["detailed","archiveNative","candidateNative"] if use_candidate else ["detailed","archiveNative"]
    indices={key:grid_index(mesh(source[key][0]["vertices"],source[key][0]["faces"])) for key in visible}
    rows={}
    for (name,ordinal),index in slots.items():
        fan=make_fan(points,np.flatnonzero(member==index))
        rows[f"{name}[{ordinal}]"]={lod:adaptive_upper(reference,points[fan],1600) for lod,reference in indices.items()}
    return {"roundingMetres":.02,"fullUpperMillimetres":{lod:max(row[lod] for row in rows.values()) for lod in indices},
            "pieces":rows}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", choices=("archive-fit", "candidate-fit", "candidate-fidelity"))
    args = parser.parse_args()
    source = load()
    result = fidelity(source) if args.report == "candidate-fidelity" else support_fit(source, args.report == "candidate-fit")
    sys.stdout.buffer.write((json.dumps(result, indent=2) + "\n").encode("utf-8"))


if __name__ == "__main__":
    main()
