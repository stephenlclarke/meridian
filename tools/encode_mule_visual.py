"""Encode SH001 source-order face attributes; no enclosure/actor/native execution."""
from pathlib import Path
import hashlib
import json
import math
import struct
import tomllib

ROOT = Path(__file__).resolve().parents[1]
RESOURCE_SHA = "1c8cce7fb8130c12780fb522592249805cb06e6b22fcffb169ae651556f47c82"

def encode(source):
    if hashlib.sha256(source).hexdigest() != RESOURCE_SHA:
        raise ValueError("Changed selected SH001 resource")
    model = tomllib.loads(source.decode())["design"]
    if (len(model["vertices"]), len(model["faces"])) != (4360, 7236):
        raise ValueError("Changed detailed face set")
    output = bytearray(struct.pack("<8sII", b"MULEVIS1", 7236 * 3, 56))
    for face, material_index, texture in zip(model["faces"], model["faceMaterials"], model["faceTextures"], strict=True):
        points = [[v * (40000 / 3411) for v in model["vertices"][i]] for i in face]
        a = [points[1][i] - points[0][i] for i in range(3)]
        b = [points[2][i] - points[0][i] for i in range(3)]
        n = [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]
        size = math.sqrt(sum(v*v for v in n))
        if size <= 0 or not math.isfinite(size):
            raise ValueError("Degenerate source face")
        normal = [v / size for v in n]
        material = model["materials"][material_index]
        rgba = [(material["colour"] >> shift & 255) / 255 for shift in (24, 16, 8, 0)]
        if not 0 <= texture["texture"] < 3:
            raise ValueError("Unexpected texture index")
        for point, uv in zip(points, texture["uv"], strict=True):
            output.extend(struct.pack("<12fII", *point, *normal, *rgba, *uv, texture["texture"], int(material["emissive"])))
    return bytes(output)

def main():
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    folder = ROOT / "assets/fleet/sh001"
    encoded = encode((folder / "resource.toml").read_bytes())
    if args.check:
        if (folder / "visual.bin").read_bytes() != encoded:
            raise ValueError("Visual encoding differs from exact source fields")
    else:
        (folder / "visual.bin").write_bytes(encoded)
    print(json.dumps({"faces":7236,"vertices":21708,"bytes":len(encoded),"sha256":hashlib.sha256(encoded).hexdigest(),"scope":"source attribute encoding only"}))

if __name__ == "__main__":
    main()
