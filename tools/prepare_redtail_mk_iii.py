"""Rebuild the two original Redtail Mk III meshes for stock inspection."""
import argparse
import hashlib
import json
from pathlib import Path
import tomllib

from import_goshawk_envelope_source import components

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets/fleet/sh103"
DEST = ROOT / "integration/sh103-redtail-mk-iii/REDTAIL-STUDIO.json"
PINS = {'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
 'contract.toml': 'a867f71cdc9693f245c54a8c63d15a657a00c6ba7db782cfa384b6f6ee41ccdf',
 'native-export.json': '05285e9531db6baecaad91e33c73e5390ed2b2495d1d13ef10e2c6eeadb9289a',
 'resource.toml': '146700ac1cd4d7990ba64a19399d0875af52d36545edd668431d9f81edd9c17e'}
REVISION = "d03f1b4ecfd7588bb6b27763e31c230ebe1fbf5a"


def mesh(model, start):
    groups = [None] * len(model["vertices"])
    partitions = components(model)
    for group, indices in enumerate(partitions, start):
        for index in indices:
            groups[index] = group
    return {"vertices": model["vertices"], "triangles": model["faces"], "groups": groups}, start + len(partitions)


def definition():
    raw = {name: (ASSETS/name).read_bytes() for name in PINS}
    for name, data in raw.items():
        if hashlib.sha256(data).hexdigest() != PINS[name]:
            raise ValueError(f"Changed original Redtail {name}")
    source = tomllib.loads(raw["resource.toml"].decode())
    contract = tomllib.loads(raw["contract.toml"].decode())
    archive = json.loads(raw["native-export.json"])
    if (archive["id"] != "SH103" or archive["coordinateUnits"] != "metres"
            or archive["sourceSHA256"] != contract["source"]["geometrySHA256"]
            or archive["nativeDesign"] != source["nativeDesign"]):
        raise ValueError("Redtail source identity or calibration changed")
    detail = source["design"]
    native = source["nativeDesign"]
    if detail["materials"] != native["materials"] or detail["textures"] != native["textures"]:
        raise ValueError("Redtail appearance tables differ between meshes")
    high, first = mesh(detail, 0)
    low, end = mesh(native, first)
    if (len(high["triangles"]), len(low["triangles"]), first, end-first) != (2328, 140, 26, 7):
        raise ValueError("Changed Redtail geometry")
    return {"id": "SH103", "name": "Redtail Mk III", "schema": 1, "metresPerUnit": 1, "roundingMetres": .02,
            "detail": high, "native": low,
            "materials": [{"name": m["name"], "rgb": [(m["colour"] >> i) & 255 for i in (24,16,8)],
                           "alpha": m["colour"] & 255, "emissive": m["emissive"]} for m in detail["materials"]],
            "faceMaterials": detail["faceMaterials"], "nativeFaceMaterials": native["faceMaterials"],
            "faceTextures": [{"texture": detail["textures"][t["texture"]]["asset"] if t["texture"] >= 0 else None,
                              "uv": t["uv"]} for t in detail["faceTextures"]],
            "engineering": None, "flight": {"maxSpeed": source["game"]["speed"]}, "clearances": [],
            "provenance": {"sourceId": source["id"], "revision": REVISION, "originalSource": archive["sourcePath"],
                           "geometrySHA256": archive["sourceSHA256"], "inputSHA256": PINS,
                           "scaleSource": "OriginalHullPhysicalContract.calibrationCase12; archived native authored in metres",
                           "license": raw["LICENSE"].decode()}}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = (json.dumps(definition(), separators=(",", ":"), allow_nan=False) + "\n").encode()
    if args.check:
        if data != DEST.read_bytes():
            raise ValueError("Redtail Studio source changed")
    else:
        DEST.write_bytes(data)
