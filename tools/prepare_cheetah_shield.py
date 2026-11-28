"""Rebuild Cheetah's two original source domains and its capped inspection shield."""
import argparse
from collections import defaultdict
from fractions import Fraction
from hashlib import sha256
import json
from pathlib import Path
import tomllib

from repair_cheetah_casings import repair

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets/fleet/sh082"
OUTPUT = ROOT / "integration/sh082-cheetah"
PACKAGE = OUTPUT / "CHEETAH-STUDIO.json"


def encode(value):
    return (json.dumps(value, separators=(",", ":"), allow_nan=False) + "\n").encode()


def inputs():
    source = tomllib.loads((ASSETS/"resource.toml").read_text())
    frozen = json.loads((OUTPUT/"FROZEN-SOURCES.json").read_bytes())
    for name in ("resource.toml", "native-export.json", "contract.toml", "finite-dry-source.json", "LICENSE"):
        if sha256((ASSETS/name).read_bytes()).hexdigest() != frozen["paths"][name]["sha256"]:
            raise ValueError(f"Changed original Cheetah {name}")
    budget = json.loads((ASSETS/"finite-dry-source.json").read_bytes())
    if sha256((ASSETS/"ownership.json").read_bytes()).hexdigest() != budget["ownershipSHA256"]:
        raise ValueError("Changed Cheetah ownership")
    owners = json.loads((ASSETS/"ownership.json").read_bytes())["models"]
    return source, owners, budget, frozen


def grouped(model, parts, first):
    """Keep each solid local. Split every recessed well into four walls and a base."""
    vertices, triangles, groups = [], [], []
    table = {}
    for part in parts:
        start, n = part["faceStart"], part["faceCount"]
        well = "/service-well-" in part["name"] and n == 20 and part["vertexCount"] == 12
        if start != len(triangles):
            raise ValueError("Noncontiguous creator face range")
        for offset, face in enumerate(model["faces"][start:start+n]):
            group = first + (offset//4 if well else 0)
            indices = []
            for i in face:
                if not part["vertexStart"] <= i < part["vertexStart"]+part["vertexCount"]:
                    raise ValueError("Triangle crosses a creator part")
                key = group, i
                if key not in table:
                    table[key] = len(vertices)
                    vertices.append(model["vertices"][i])
                    groups.append(group)
                indices.append(table[key])
            triangles.append(indices)
        first += 5 if well else 1
    if len(triangles) != len(model["faces"]):
        raise ValueError("Lost creator triangles")
    return {"vertices": vertices, "triangles": triangles, "groups": groups}, first


def coverage(detail, native, source):
    # Three original corners in one local convex support certify the entire triangle.
    memberships = defaultdict(set)
    for mesh in (detail, native):
        for p, group in zip(mesh["vertices"], mesh["groups"], strict=True):
            memberships[tuple(p)].add(group)
    for name in ("design", "nativeDesign"):
        mesh = source[name]
        for i, face in enumerate(mesh["faces"]):
            corners = [memberships[tuple(mesh["vertices"][v])] for v in face]
            if not set.intersection(*corners):
                raise ValueError(f"Uncovered original {name} face {i}")


def definition():
    source, owners, budget, frozen = inputs()
    repaired, parts = repair(ASSETS)
    detail = source["design"]
    high, next_group = grouped(detail, owners["design"]["parts"], 0)
    low, _ = grouped(repaired, parts, next_group)
    coverage(high, low, source)
    routes = [("dorsal", 0, 4.2, -15.5, 15.5), ("ventral", 0, -4.1, -15.5, 15.5)]
    for side, sign in (("port",-1), ("starboard",1)):
        routes.extend([(side+"-outboard", sign*8.2, 0, -15.5, 15.5),
                       (side+"-forward-inner", sign*.7, -2.25, 6, 14.9)])
    dry = budget["emptyDryReference"]
    tensor = dry["inertiaAboutCentreOfMassKgM2"]
    return {
        "schema": 1, "id": "SH082", "name": "Cheetah",
        "metresPerUnit": float(Fraction(budget["metresPerSourceUnitExact"])), "roundingMetres": .02,
        "detail": high, "native": low,
        "materials": [{"name": m["name"], "rgb": [(m["colour"] >> i) & 255 for i in (24,16,8)],
                       "alpha": m["colour"] & 255, "emissive": m["emissive"]} for m in detail["materials"]],
        "faceMaterials": detail["faceMaterials"], "nativeFaceMaterials": repaired["faceMaterials"],
        "faceTextures": [{"texture": detail["textures"][t["texture"]]["asset"] if t["texture"] >= 0 else None,
                          "uv": t["uv"]} for t in detail["faceTextures"]],
        "engineering": {"massKg": dry["massKg"], "centerOfMass": dry["centreMetres"],
                        "inertia": [tensor[i:i+3] for i in (0,3,6)]},
        "flight": {"maxSpeed": source["game"]["speed"]},
        "clearances": [{"name": n, "start": [x,y,a], "end": [x,y,b], "radiusMetres": .03}
                       for n,x,y,a,b in routes],
        "provenance": {"sourceId": "SH082", "revision": frozen["revision"],
                       "originalGeometrySHA256": "1588cff096e99efafd72688aeb3f764080c36517658a682c296f0b872ca33f49",
                       "nativeVariant": "512-face casing repair of the 504-face historical candidate",
                       "license": (ASSETS/"LICENSE").read_text()},
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    raw = encode(definition())
    if args.check:
        if raw != PACKAGE.read_bytes():
            raise ValueError("Cheetah package differs from checked-in geometry")
    else:
        PACKAGE.write_bytes(raw)
