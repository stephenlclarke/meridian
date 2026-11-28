"""Build an inspection model from one immutable catalogue registration and its original artwork."""
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import tomllib

from atomic_asset_output import replace_output
from import_goshawk_envelope_source import components

ROOT = Path(__file__).resolve().parents[1]


def checked_inputs(manifest):
    receipt = json.loads(manifest.read_bytes())
    identity = receipt["id"]
    if len(identity) != 5 or not identity.startswith("SH") or not identity[2:].isascii() or not identity[2:].isdigit():
        raise ValueError("Invalid original catalogue identity")
    folder = ROOT / "assets/fleet" / identity.lower()
    raw = {}
    for name in ("resource.toml", "native-export.json", "contract.toml", "LICENSE", "engineering.json"):
        if name == "engineering.json" and name not in receipt["inputs"]:
            continue
        raw[name] = (folder/name).read_bytes()
        if hashlib.sha256(raw[name]).hexdigest() != receipt["inputs"][name]["sha256"]:
            raise ValueError(f"Changed {identity} input {name}")
    source = tomllib.loads(raw["resource.toml"].decode())
    contract = tomllib.loads(raw["contract.toml"].decode())
    native = json.loads(raw["native-export.json"])
    if not (identity == source["id"] == contract["id"] == native["id"]
            and source["name"] == contract["name"] == receipt["name"]
            and native["sourceSHA256"] == contract["source"]["geometrySHA256"]
            and native["sourcePath"] == contract["source"]["geometryFile"]):
        raise ValueError("Original registration differs from the selected catalogue entry")
    if native.get("coordinateUnits") == "metres":
        scale = 1
    else:
        physical = json.loads(raw["engineering.json"])
        scale = float(Fraction(physical["metresPerSourceUnitExact"]))
        if not (physical["id"] == identity and physical["metresPerSourceUnit"] == scale
                and physical["contractSHA256"] == receipt["inputs"]["contract.toml"]["sha256"]
                and physical["detailedSourceSHA256"] == native["sourceSHA256"]):
            raise ValueError("Calibration does not match this geometry")
    return receipt, source, native, scale, raw["LICENSE"].decode()


def component_mesh(model, start):
    parts = components(model)
    labels = [0] * len(model["vertices"])
    for local, vertices in enumerate(parts, start):
        for index in vertices:
            labels[index] = local
    return dict(vertices=model["vertices"], triangles=model["faces"], groups=labels), start + len(parts)


def visual_definition(receipt, source, archive, scale, notice, detail_mesh, native_mesh):
    detail, native = source["design"], source["nativeDesign"]
    if detail["materials"] != native["materials"]:
        raise ValueError("Detailed and native palettes disagree")
    for level in (detail, native):
        if level["license"] != "MIT" or level["appearanceLicense"] != "MIT":
            raise ValueError("Missing original artwork licence")
    palette = []
    for material in detail["materials"]:
        r,g,b,alpha = material["colour"].to_bytes(4, "big")
        palette.append(dict(name=material["name"], rgb=[r,g,b], alpha=alpha, emissive=material["emissive"]))
    package = dict(id=receipt["id"], name=receipt["name"], schema=2, metresPerUnit=scale, roundingMetres=.02,
                   detail=detail_mesh, native=native_mesh, materials=palette, faceMaterials=detail["faceMaterials"],
                   nativeFaceMaterials=native["faceMaterials"], engineering=None, flight={"maxSpeed":None}, clearances=[],
                   provenance=dict(sourceId=receipt["id"], revision=receipt["revision"], sourceInputs=receipt["inputs"],
                                   geometrySHA256=archive["sourceSHA256"], license=notice))
    if "faceTextures" in detail:
        assets = [t["asset"] for t in detail.get("textures", [])]
        package["faceTextures"] = [dict(uv=m["uv"], texture=assets[m["texture"]] if m["texture"] >= 0 else None)
                                   for m in detail["faceTextures"]]
    return package


def archive_definition(manifest):
    receipt, source, archive, scale, notice = checked_inputs(manifest)
    if source["nativeDesign"] != archive["nativeDesign"]:
        raise ValueError("Selected native geometry differs from the archived source")
    high, start = component_mesh(source["design"], 0)
    low, _ = component_mesh(source["nativeDesign"], start)
    return visual_definition(receipt, source, archive, scale, notice, high, low)


def persist(package, path, check):
    raw = (json.dumps(package, separators=(",", ":"), allow_nan=False) + "\n").encode()
    if check:
        if path.read_bytes() != raw:
            raise ValueError(f"Changed stock package {path.name}")
    else:
        replace_output(path, raw)


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    persist(archive_definition(args.manifest), args.manifest.with_name("ORIGINAL-INSPECTION.json"), args.check)
