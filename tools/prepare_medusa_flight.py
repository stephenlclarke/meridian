"""Pack the qualified SH171 collision candidate in the station physics format."""
import argparse
from collections import Counter
from hashlib import sha256
import json
from math import isfinite
from pathlib import Path
import struct

from atomic_asset_output import replace_output


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "integration/sh171-medusa-flight/NATIVE-STRUCTURE-SPANS.json"
OUTPUT = ROOT / "assets/fleet/sh171/flight.physics"
SUPPORT_SOURCE = SOURCE.with_name("adapter-trial.bin")
SUPPORT_OUTPUT = OUTPUT.with_name("flight.supports")
SUPPORT_SOURCE_SHA256 = "21ae0457d3d18e4c21eff22b6db48cad88127c81416e72f3e873592f2a00dfc9"
SOURCE_SHA256 = "432e703c435b25ad13a6430a647c8c0d7995935d23c0cc469d202af579db2261"


def collision_bytes():
    raw = SOURCE.read_bytes()
    if sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError("Medusa collision source differs from the verified trial")
    report = json.loads(raw)
    vertices = report["candidate"]["verticesMetres"]
    faces = report["candidate"]["faces"]
    if (len(vertices), len(faces), report["populatedFaces"]) != (372, 512, 512):
        raise ValueError("Medusa collision source exceeds its authenticated geometry budget")
    if any(len(point) != 3 or not all(isfinite(value) for value in point) for point in vertices):
        raise ValueError("invalid Medusa collision vertex")
    if any(len(set(face)) != 3 or len(face) != 3 or
           any(type(index) is not int or not 0 <= index < len(vertices) for index in face)
           for face in faces):
        raise ValueError("invalid Medusa collision triangle")
    edges = Counter((a, b) for a, b, c in faces for a, b in ((a, b), (b, c), (c, a)))
    if any(count != 1 or edges[b, a] != 1 for (a, b), count in edges.items()):
        raise ValueError("Medusa collision source contains an open or misoriented edge")
    if {index for face in faces for index in face} != set(range(len(vertices))):
        raise ValueError("Medusa collision source contains an unused vertex")
    return (struct.pack("<8sII", b"MPHYS001", len(vertices), len(faces))
            + b"".join(struct.pack("<3d", *point) for point in vertices)
            + b"".join(struct.pack("<3I", *face) for face in faces))


def support_bytes():
    raw = SUPPORT_SOURCE.read_bytes()
    if sha256(raw).hexdigest() != SUPPORT_SOURCE_SHA256 or raw[:8] != b"MDU512T1":
        raise ValueError("Medusa supports differ from the verified trial")
    if raw[8:40] != bytes.fromhex(SOURCE_SHA256) or struct.unpack_from("<I", raw, 40)[0] != 70:
        raise ValueError("Medusa support fixture has an unexpected geometry identity")
    end = 44
    for _ in range(70):
        count, = struct.unpack_from("<I", raw, end)
        if not 4 <= count <= 8:
            raise ValueError("invalid Medusa support size")
        end += 4 + count * 24
    # The immutable trial ends with four crown origins and four range probes.
    if len(raw) - end != 4 * 24 + 4 * 56:
        raise ValueError("invalid Medusa trial suffix")
    return b"MDU512S1" + raw[40:end]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for output, data in ((OUTPUT, collision_bytes()), (SUPPORT_OUTPUT, support_bytes())):
        if args.check:
            if output.read_bytes() != data:
                raise ValueError(f"checked-in Medusa flight bytes differ: {output.name}")
        else:
            replace_output(output, data)
        print(f"SH171 {output.name}: SHA-256 {sha256(data).hexdigest()}")


if __name__ == "__main__":
    main()
