"""Rebuild Besra's original metre-scale station Showroom package."""
import argparse
from hashlib import sha256
import json
from pathlib import Path
import tomllib

from atomic_asset_output import replace_output
from import_goshawk_envelope_source import components

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets/fleet/sh108"
OUTPUT = ROOT / "integration/sh108-besra/BESRA-STUDIO.json"
PINS = {
    "LICENSE": "033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766",
    "contract.toml": "e6dd044e8985abfeb14b7a23296389c5acbb210e901bebe64aa4cbbdc49c4de8",
    "native-export.json": "301a01431e934a51eec8ea7a7d8b48f82452093aebefd12ab46292a79edfc79a",
    "resource.toml": "5975818748faca138454ab9f7d274034391bfbe4159729a9a738927405d26b7d"
}
GEOMETRY = "ef7d78789eed53b721c6f9417abe5f79a0394613540ff901d8af1fd9c76e13f1"
REVISION = "0d8cc9ebe0018bf74b0d9fa10ef23b70339dd6b0"


def original_mesh(source, group_start):
    partitions = components(source)
    groups = [0] * len(source["vertices"])
    for group, vertices in enumerate(partitions, group_start):
        for v in vertices:
            groups[v] = group
    return dict(vertices=source["vertices"], triangles=source["faces"], groups=groups), len(partitions)


def definition(folder=ASSETS):
    raw = {name: (folder/name).read_bytes() for name in PINS}
    for name, expected in PINS.items():
        if sha256(raw[name]).hexdigest() != expected:
            raise ValueError(f"Changed Besra source {name}")
    resource = tomllib.loads(raw["resource.toml"].decode())
    archive = json.loads(raw["native-export.json"])
    contract = tomllib.loads(raw["contract.toml"].decode())
    if not (resource["id"] == archive["id"] == contract["id"] == "SH108"
            and resource["name"] == contract["name"] == "Besra"
            and archive["sourceSHA256"] == contract["source"]["geometrySHA256"] == GEOMETRY
            and archive["sourcePath"] == contract["source"]["geometryFile"]
            and archive["coordinateUnits"] == "metres" and resource["nativeDesign"] == archive["nativeDesign"]):
        raise ValueError("Besra source registration changed")
    detail, native = resource["design"], archive["nativeDesign"]
    high, detailed_groups = original_mesh(detail, 0)
    low, native_groups = original_mesh(native, detailed_groups)
    if detailed_groups != 28 or native_groups != 9 or detail["materials"] != native["materials"]:
        raise ValueError("Besra component or material identity changed")
    palette = []
    for material in detail["materials"]:
        r, g, b, alpha = material["colour"].to_bytes(4, "big")
        palette.append(dict(name=material["name"], rgb=[r, g, b], alpha=alpha, emissive=material["emissive"]))
    textures = [entry["asset"] for entry in detail["textures"]]
    return dict(schema=1, id=resource["id"], name=resource["name"], metresPerUnit=1, roundingMetres=0.02,
                detail=high, native=low, materials=palette, faceMaterials=detail["faceMaterials"],
                nativeFaceMaterials=native["faceMaterials"],
                faceTextures=[dict(uv=m["uv"], texture=textures[m["texture"]] if m["texture"] >= 0 else None)
                              for m in detail["faceTextures"]],
                engineering=None, flight=dict(maxSpeed=resource["game"]["speed"]), clearances=[],
                provenance=dict(sourceId="SH108", revision=REVISION, originalGeometrySHA256=GEOMETRY,
                                sourceFiles=PINS, calibration="OriginalHullPhysicalContract.calibrationCase13",
                                license=raw["LICENSE"].decode()))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    payload = (json.dumps(definition(), separators=(",", ":"), allow_nan=False) + "\n").encode()
    if args.check:
        if OUTPUT.read_bytes() != payload:
            raise ValueError("Besra package differs from the original inputs")
    else:
        replace_output(OUTPUT, payload)
