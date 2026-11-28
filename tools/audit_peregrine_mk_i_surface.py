"""Independent rounded-union bounds for the two SH022 source surfaces."""

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
import trimesh

from measure_goshawk_surface_fit import (
    audit_fan, certified_upper, certify_exposed_sample, exposed_samples,
    integer_source, nearest, verify_lower, worst_sample,
)
from prepare_peregrine_mk_i import ASSETS, COUNTS, MANIFEST_SHA, ROOT, pin, require

OUTPUT = ROOT / "integration/sh022-peregrine-mk-i/SURFACE-FIT.json"
TARGET_MM = {"detailed": 900, "nativeCandidate": 1740}


def read_inputs(folder):
    manifest_raw = (folder/"MANIFEST.json").read_bytes()
    manifest = json.loads(manifest_raw)
    require(manifest["id"] == "SH022", "Wrong surface audit identity")
    require(pin(manifest_raw)["sha256"] == MANIFEST_SHA, "Changed surface audit manifest")
    names = ["source.vertices","source-triangles.bin","source-piece-membership.bin","COMPONENT-PIECES.json"]
    data = {name:(folder/name).read_bytes() for name in names}
    require(all(pin(blob) == manifest["encoded"][name] for name,blob in data.items()), "Changed surface audit input")
    vertices = np.frombuffer(data[names[0]],dtype="<f8",offset=16).reshape(-1,3)
    faces = np.frombuffer(data[names[1]],dtype="<u4").reshape(-1,3)
    membership = np.frombuffer(data[names[2]],dtype="u1")
    labels = json.loads(data[names[3]])["pieces"]
    n,f,_ = COUNTS[0]
    nn,ff,_ = COUNTS[1]
    require(data[names[0]][:8] == b"MSH022V1" and vertices.shape == (n+nn,3)
            and faces.shape == (f+ff,3), "Changed audit geometry dimensions")
    require(len(labels) == 69 and set(membership) == set(range(len(labels))) and np.all(membership[faces] == membership[faces[:,:1]]), "Invalid local partition")
    require(np.all(faces[:f] < n) and np.all(faces[f:] >= n) and np.isfinite(vertices).all(), "Invalid original face domain")
    return manifest, labels, vertices, faces, membership


def measure(folder):
    manifest, labels, xyz, faces, membership = read_inputs(folder)
    n,f,_ = COUNTS[0]
    models = {
        "detailed": (0,trimesh.Trimesh(xyz[:n],faces[:f],process=False)),
        "nativeCandidate": (n,trimesh.Trimesh(xyz[n:],faces[f:]-n,process=False)),
    }
    pieces = []
    for i in range(len(labels)):
        ids = np.flatnonzero(membership == i)
        points, first = np.unique(xyz[ids], axis=0, return_index=True)
        piece = audit_fan(points)
        piece["sourceIds"] = ids[first]
        pieces.append(piece)
    exact, scale, exact_faces = integer_source(xyz,models)
    mandatory = []
    for source, other, subset in (("detailed","nativeCandidate",slice(n)),("nativeCandidate","detailed",slice(n,None))):
        _, distances, nearest_faces = nearest(models[other][1],xyz[subset])
        index = int(np.argmax(distances))
        global_index = models[source][0]+index
        limit = int(distances[index]*1000)-1
        verify_lower(exact[global_index],exact_faces[other],scale,limit)
        mandatory.append({"source":source,"comparedTo":other,"vertex":index,"piece":labels[membership[global_index]],
                          "nearestFace":int(nearest_faces[index]),"sampleMetres":float(distances[index]),"certifiedLowerMillimetres":limit})
    per_piece = []
    peak = {key:[] for key in models}
    for name,piece in zip(labels,pieces,strict=True):
        samples = exposed_samples(piece,pieces)
        comparisons = {}
        for source,(_,mesh) in models.items():
            record,index = worst_sample(mesh,samples[0])
            comparisons[source] = {"largestExposedSample":record,"fullRoundedPieceUpperMillimetres":certified_upper(mesh,piece,TARGET_MM[source])}
            if record is not None:
                peak[source].append((record,index,samples,piece))
        per_piece.append({"piece":name,"exposedSamples":len(samples[0]),"comparisons":comparisons})
    for source,results in peak.items():
        for record,index,samples,piece in sorted(results,key=lambda item:-item[0]["metres"])[:2]:
            certify_exposed_sample(record,index,samples,piece,pieces,(exact_faces[source],scale))
    summary = {}
    for source in models:
        bound = max(r["comparisons"][source]["fullRoundedPieceUpperMillimetres"] for r in per_piece)
        samples = [r["comparisons"][source]["largestExposedSample"] for r in per_piece if r["comparisons"][source]["largestExposedSample"]]
        summary[source] = {"certifiedUpperMillimetres":bound,"largestExposedSample":max(samples,key=lambda r:r["metres"])}
    return {
        "hull":"SH022","legacyRevision":manifest["legacyRevision"],"manifestSha256":hashlib.sha256((folder/"MANIFEST.json").read_bytes()).hexdigest(),
        "nominalPaddingMetres":0.02,"pieceCount":len(per_piece),"sourceComparisons":summary,"mandatoryCrossSourceVertices":mandatory,
        "perPiece":per_piece,
        "method":"Exact calibrated dyadic hull-fan and outward-rounded patch coverage give a conservative upper bound including occluded piece surfaces. Exact point-to-triangle checks certify the largest nominated source vertices and two largest exposed samples against each distinct source. Sampling alone cannot prove a maximum.",
        "scope":"Source-only directed surface bounds. Actual station/render/native/runtime acceptance remains separate.",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets",type=Path,default=ASSETS)
    parser.add_argument("--write",action="store_true")
    args = parser.parse_args()
    result = measure(args.assets)
    output = json.dumps(result,indent=2,allow_nan=False)+"\n"
    if args.write:
        OUTPUT.write_text(output)
    else:
        print(output,end="")


if __name__ == "__main__":
    main()
