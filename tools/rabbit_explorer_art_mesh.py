"""Use the archived Rabbit artwork and its separate reproducible candidate."""

from fractions import Fraction
import hashlib
import json
from pathlib import Path
import tomllib

from banteng_art_mesh import assemble

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / "assets/fleet/sh066"
INTEGRATION = ROOT / "integration/sh066-rabbit-explorer"
SCALE = Fraction(30_000_000, 2_547_773)
ARC = "open curved loop"
ARC_SPANS = 12
ABSENT_SECONDARY = 255
CANDIDATE_SHA = "c4351fb3719db64e2f64cc77edb50531dca3b486c990dd1b792354bd5e107c0c"


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def load():
    pins = json.loads((INTEGRATION / "FROZEN-SOURCES.json").read_bytes())
    inputs = {}
    for name, expected in pins["paths"].items():
        directory = INTEGRATION if name.startswith("LEGACY-") else ORIGINAL
        if name.endswith(".rgba"):
            directory = ROOT / "assets/fleet/sh001"
        data = (directory / name).read_bytes()
        require(len(data) == expected["bytes"] and hashlib.sha256(data).hexdigest() == expected["sha256"],
                f"changed frozen Rabbit Explorer source: {name}")
        inputs[name] = data

    resource = tomllib.loads(inputs["resource.toml"].decode())
    archive = json.loads(inputs["native-export.json"])
    finite = json.loads(inputs["finite-dry-source.json"])
    candidate_data = (ORIGINAL / "SH066-candidate.json").read_bytes()
    require(hashlib.sha256(candidate_data).hexdigest() == CANDIDATE_SHA, "changed Rabbit Explorer candidate bytes")
    candidate = json.loads(candidate_data)
    require(resource["nativeDesign"] == archive["nativeDesign"], "changed Rabbit Explorer native archive")
    require(candidate["originalSourceSHA256"] == archive["sourceSHA256"], "changed Rabbit Explorer generation input")
    require(Fraction(finite["metresPerSourceUnitExact"]) == SCALE, "changed Rabbit Explorer calibration")
    mesh = candidate["nativeCandidate"]
    models = resource["design"], resource["nativeDesign"], mesh
    require([len(model["vertices"]) for model in models] == [1444, 472, 268], "changed Rabbit Explorer vertex counts")
    require([len(model["faces"]) for model in models] == [2648, 888, 480], "changed Rabbit Explorer face counts")
    ownership = (INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes()
    require(hashlib.sha256(ownership).hexdigest() == "cd19199e65b8cebca7003694865deeb378e9146a76cfcf4f6bed1615930654d7",
            "changed Rabbit Explorer ownership source")
    parts = json.loads(ownership)["models"]
    source = {
        "detailed": (models[0], parts["design"]["parts"]),
        "archiveNative": (models[1], parts["nativeDesign"]["parts"]),
        "candidateNative": (mesh, candidate["parts"]),
    }
    return inputs, resource, finite["emptyDryReference"], source, candidate_data


def metre_space(model):
    return dict(model, vertices=[[coordinate * float(SCALE) for coordinate in point] for point in model["vertices"]])


def collision_groups(detailed, native, *, candidate):
    artwork_slots, meshes = assemble(detailed, native)
    membership = meshes[0]["groups"] + meshes[1]["groups"]
    groups = {
        f"{name}[{ordinal}]": [i for i, member in enumerate(membership) if member == slot]
        for (name, ordinal), slot in artwork_slots.items() if name != ARC
    }
    native_sampling = (12, 4) if candidate else (22, 8)
    for (model, parts), (steps, width), offset in (
        (detailed, (44, 12), 0), (native, native_sampling, len(meshes[0]["vertices"])),
    ):
        arc, = [part for part in parts if part["name"] == ARC]
        require(arc["vertexCount"] == (steps + 1) * width, "changed Rabbit Explorer arc sampling")
        start = arc["vertexStart"] + offset
        spans = []
        for span in range(ARC_SPANS):
            first_ring = span * steps // ARC_SPANS
            last_ring = ((span + 1) * steps + ARC_SPANS - 1) // ARC_SPANS
            indices = list(range(start + first_ring * width, start + (last_ring + 1) * width))
            groups.setdefault(f"{ARC}/span-{span + 1:02d}", []).extend(indices)
            spans.append(set(indices))
        faces = model["faces"][arc["faceStart"]:arc["faceStart"] + arc["faceCount"]]
        require(all(any({offset + i for i in face} <= span for span in spans) for face in faces),
                "uncovered Rabbit Explorer arc triangle")
    for mesh, offset in ((meshes[0], 0), (meshes[1], len(meshes[0]["vertices"]))):
        covered = [set() for _ in mesh["vertices"]]
        for slot, indices in enumerate(groups.values()):
            for index in indices:
                if offset <= index < offset + len(covered):
                    covered[index - offset].add(slot)
        require(all(set.intersection(*(covered[index] for index in face)) for face in mesh["triangles"]),
                "uncovered Rabbit Explorer triangle")
    return artwork_slots, meshes, groups


def collision_membership(vertex_count, groups):
    entries = [[] for _ in range(vertex_count)]
    for slot, vertices in enumerate(groups.values()):
        for index in vertices:
            entries[index].append(slot)
    require(all(1 <= len(entry) <= 2 for entry in entries), "invalid Rabbit Explorer collision membership")
    return bytes(slot for entry in entries for slot in (entry if len(entry) == 2 else [entry[0], ABSENT_SECONDARY]))
