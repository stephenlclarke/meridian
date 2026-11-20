"""Independent, positive separating-plane and SH028 contact evidence for SH021."""

import argparse
import hashlib
import json
import math
from pathlib import Path
import struct

from prepare_zebra import ASSETS, ROOT, COUNTS, MANIFEST_SHA, json_bytes, require

GUARDED_PADDING_METRES = 0.021
STATION_SHA = "cd54d30cab638f6b33feacf1fa78c9136f8ce346e8cc6133259bfc0472c259d5"
PORT_SHA = "31a7ed33803e78ebbe7bb87c0658e47a786e8bba9107d513b2e534ccd30cf71f"
DESTINATION = ROOT / "integration/sh021-zebra/FIT-WITNESS.json"


def pinned(path, fingerprint):
    raw = path.read_bytes()
    require(hashlib.sha256(raw).hexdigest() == fingerprint, f"Changed fixture: {path}")
    return raw


def bounds(points):
    return tuple((min(col), max(col)) for col in zip(*points))


def axis_gap(a, b):
    return max(max(lo2 - hi1, lo1 - hi2) for (lo1, hi1), (lo2, hi2) in zip(a, b)) - GUARDED_PADDING_METRES


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def normal(triangle):
    a, b, c = triangle
    u, v = sub(b, a), sub(c, a)
    value = (u[1]*v[2]-u[2]*v[1], u[2]*v[0]-u[0]*v[2], u[0]*v[1]-u[1]*v[0])
    length = math.hypot(*value)
    return tuple(x/length for x in value) if length else None


def projection_gap(points, other, axis):
    a = [math.fsum(x*y for x, y in zip(p, axis)) for p in points]
    b = [math.fsum(x*y for x, y in zip(p, axis)) for p in other]
    return max(min(b)-max(a), min(a)-max(b)) - GUARDED_PADDING_METRES


def swept(points, x, before, after, inward):
    sign = -1 if inward else 1
    return [(x+sign*a, b, z+sign*c) for a,b,c in points for z in (before,after)]


def station_clearance(groups, triangles, x, before, after):
    target = [bounds(triangle) for triangle in triangles]
    return min((axis_gap(bounds(swept(group, x, before, after, True)), triangle), i, j)
               for i, group in enumerate(groups) for j, triangle in enumerate(target))


def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def verify():
    source = json.loads(pinned(ASSETS / "MANIFEST.json", MANIFEST_SHA))
    enc = source["encoded"]
    points_raw = pinned(ASSETS / "source.vertices", enc["source.vertices"]["sha256"])
    faces_raw = pinned(ASSETS / "source-triangles.bin", enc["source-triangles.bin"]["sha256"])
    membership = pinned(ASSETS / "source-piece-membership.bin", enc["source-piece-membership.bin"]["sha256"])
    require(struct.unpack_from("<8sII", points_raw) == (b"MSH021V1", COUNTS[0][0], COUNTS[1][0]), "Wrong hull fixture")
    vertices = list(struct.iter_unpack("<3d", points_raw[16:]))
    faces = list(struct.iter_unpack("<3I", faces_raw))
    groups = [[] for _ in range(48)]
    face_groups = [[] for _ in groups]
    for point, group in zip(vertices, membership, strict=True):
        groups[group].append(point)
    for face in faces:
        group = membership[face[0]]
        require(all(membership[i] == group for i in face), "Source triangle not locally enclosed")
        face_groups[group].append([vertices[i] for i in face])
    require(all(groups) and len(faces) == 3272, "Incomplete union")
    station = pinned(ROOT/"assets/legacy/sh028.physics", STATION_SHA)
    magic, count, face_count = struct.unpack_from("<8sII", station)
    require((magic,count,face_count) == (b"MPHYS001",272,488), "Changed SH028 dimensions")
    station_vertices = list(struct.iter_unpack("<3d", station[16:16+count*24]))
    station_faces = list(struct.iter_unpack("<3I", station[16+count*24:]))
    triangles = [[station_vertices[i] for i in face] for face in station_faces]
    port = pinned(ROOT/"assets/legacy/sh028.port", PORT_SHA)
    header = struct.unpack_from("<8s10dI",port)
    require(header[0] == b"MPORT001" and header[1:10] == (0.0,0.0,136.32,0.0,0.0,1.0,0.0,1.0,0.0), "Changed aligned port frame")
    aperture = list(struct.iter_unpack("<3d",port[92:]))
    require(len(aperture) == header[-1] == 4, "Changed SH028 aperture")
    port_z = header[3]

    def aperture_slack(x):
        point_xy = [(x-a,b) for a,b,_ in vertices]
        gaps = []
        for a,b in zip(aperture, aperture[1:]+aperture[:1]):
            dx,dy = b[0]-a[0],b[1]-a[1]
            gaps.append(min((dx*(y-a[1])-dy*(px-a[0]))/math.hypot(dx,dy) for px,y in point_xy)-GUARDED_PADDING_METRES)
        return min(gaps)

    aligned = station_clearance(groups, triangles, 0.0, port_z+0.5, port_z-0.5)
    off = station_clearance(groups, triangles, 100.0, port_z+0.5, port_z-0.5)
    full_aligned = station_clearance(groups, triangles, 0.0, 220.0, 135.0)
    full_off = station_clearance(groups, triangles, 100.0, 220.0, 135.0)
    require(aperture_slack(0.0) > 39.6 and aperture_slack(100.0) < 0, "SH028 aperture controls changed")
    require(aligned[0] > 39.6 and off[0] > 3.79, "SH028 swept clear controls changed")
    require(full_aligned[0] > 39.6 and full_off[0] > 2.97, "Full controlled route clearance changed")

    channel=[(-0.15,-2.1,20.0),(0.15,-2.1,20.0),(0.0,-1.8,20.0)]
    candidates=[(1.0,0,0),(0,1.0,0),(0,0,1.0)]
    proof=[]
    for i,group in enumerate(groups):
        path=swept(group,0,0,40,False)
        normals=candidates+[n for triangle in face_groups[i] if (n:=normal(triangle)) is not None]
        gap,axis=max((projection_gap(path,channel,n),n) for n in normals)
        proof.append((gap,i,axis))
    minimum=min(proof)
    require(minimum[0]>1.37,f"Actual full freight trench loses clearance: {minimum}")
    channel_witness={"triangleMetres":channel,"localTranslationMetres":[0,0,40],
                     "minimumSeparatingGapMetres":minimum[0],"limitingPiece":minimum[1],"axis":minimum[2]}

    face = triangles[482]
    rail = vertices[96]
    require(station_faces[482] == (265,266,270) and all(p[2] == port_z for p in face), "Changed station lip triangle")
    x = 69.4 + rail[0]
    contact_z = port_z + rail[2]
    q = (69.4,rail[1],port_z)
    # Solve barycentric weights in the pinned coplanar triangle's transverse frame.
    a,b,c = face
    w3 = (q[1]-a[1])/(c[1]-a[1])
    w1 = (b[0]-q[0])/(b[0]-a[0])
    w2 = 1-w1-w3
    require(min(w1,w2,w3) > 0 and contact_z-1 > port_z, "Lip witness misses strict triangle interior")
    clear_before = station_clearance(groups, triangles, x, contact_z+1, contact_z+1)
    require(clear_before[0] > 0.9, "Initial lip pose is obstructed")
    aft = min(range(len(vertices)), key=lambda i: vertices[i][2])
    launch = max(p[2] for p in station_vertices)-vertices[aft][2]+0.02+1000.0
    bits = struct.unpack("<I", struct.pack("<f",launch))[0]
    if f32(launch) < launch:
        bits += 1
    outward = struct.unpack("<f",struct.pack("<I",bits))[0]
    require(outward >= launch > struct.unpack("<f",struct.pack("<I",bits-1))[0], "Outward f32 launch witness")
    return {
        "hull":"SH021", "sourceVerticesSha256":enc["source.vertices"]["sha256"], "stationSha256":STATION_SHA,
        "portSha256":PORT_SHA, "guardedPaddingMetres":GUARDED_PADDING_METRES, "sourceTriangles":len(faces),
        "alignedApertureSlackMetres":aperture_slack(0), "offApertureSlackMetres":aperture_slack(100),
        "alignedStationSweepGapMetres":aligned[0], "offApertureStationSweepGapMetres":off[0],
        "completeControlledSweep":{"centreZSweepMetres":[220.0,135.0],
                                   "alignedGapMetres":full_aligned[0],"offApertureGapMetres":full_off[0]},
        "freightTrenchChannel":channel_witness,
        "lip":{"sourceVertex":96,"sourceVertexMetres":rail,"stationFace":482,"stationTriangleMetres":face,
               "centreXMetres":x,"centreZAtContactMetres":contact_z,"centreZSweepMetres":[contact_z+1,contact_z-1],
               "contactMetres":q,"barycentric":[w1,w2,w3],"initialSeparatingGapMetres":clear_before[0]},
        "launch":{"rearSourceVertex":aft,"unroundedZMetres":launch,"outwardF32ZMetres":outward,"outwardF32Bits":f"0x{bits:08x}"},
        "scope":"Original metre vertices and strict projection bounds; all 3,272 triangles are enclosed by local convexity. No claim of continuous whole-surface tightness, native execution, or fleet admission.",
    }


def same_witness(expected, actual):
    if isinstance(expected, dict):
        return isinstance(actual, dict) and expected.keys() == actual.keys() and all(
            same_witness(expected[k], actual[k]) for k in expected
        )
    if isinstance(expected, (list, tuple)):
        return isinstance(actual, (list, tuple)) and len(expected) == len(actual) and all(
            same_witness(a, b) for a, b in zip(expected, actual)
        )
    if isinstance(expected, float):
        return isinstance(actual, (float, int)) and math.isclose(expected, actual, rel_tol=0, abs_tol=1e-10)
    return expected == actual


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    result = verify()
    if args.write:
        DESTINATION.parent.mkdir(parents=True, exist_ok=True)
        DESTINATION.write_bytes(json_bytes(result))
    else:
        require(same_witness(json.loads(DESTINATION.read_bytes()), result), "SH021 independent fit witness changed")
    print("SH021 source fit, opening, lip contact and launch independently verified")


if __name__ == "__main__":
    main()
