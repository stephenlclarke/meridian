"""Resolve local authoring inputs and prepare a self-contained hull package."""
import hashlib
import json
import struct
import tomllib
from functools import lru_cache
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def reference_fit():
    packed = HERE / "reference-fit.json"
    if packed.is_file():
        return parse(packed.read_bytes())
    return parse((ROOT / "integration/sh023-peregrine-mk3/SURFACE-FIT.json").read_bytes())["summary"]["detailed"]


def parse(raw):
    def unique(pairs):
        obj = {}
        for key, value in pairs:
            if key in obj:
                raise ValueError(f"Duplicate field: {key}")
            obj[key] = value
        return obj

    def invalid(value):
        raise ValueError(f"Invalid JSON number: {value}")

    return json.loads(raw, object_pairs_hook=unique, parse_constant=invalid)


def encode(value):
    return json.dumps(value, allow_nan=False, ensure_ascii=False, separators=(",", ":")).encode()


def local_definition(path):
    """CLI-only relative JSON references; browser uploads are self-contained."""
    definition = parse(path.read_bytes())
    if not isinstance(definition, dict):
        raise ValueError("A definition must be an object")
    base = path.resolve().parent
    for key in ("detail", "native", "materials", "faceMaterials", "faceTextures"):
        ref = definition.get(key)
        if isinstance(ref, dict) and set(ref) == {"file"}:
            if not isinstance(ref["file"], str) or Path(ref["file"]).is_absolute():
                raise ValueError(f"{key}.file must be a relative path")
            target = (base / ref["file"]).resolve()
            if not target.is_relative_to(base):
                raise ValueError(f"{key}.file must be within the definition directory")
            definition[key] = parse(target.read_bytes())
    return definition


def components(mesh):
    points, faces = mesh.get("vertices"), mesh.get("triangles")
    if not isinstance(points, list) or not isinstance(faces, list):
        raise ValueError("Automatic grouping requires vertices and triangles")
    parents = list(range(len(points)))

    def root(i):
        while i != parents[i]:
            parents[i] = parents[parents[i]]
            i = parents[i]
        return i

    for face in faces:
        if not isinstance(face, list) or len(face) != 3 or any(
            type(i) is not int or not 0 <= i < len(points) for i in face
        ):
            raise ValueError("Automatic grouping encountered an invalid triangle")
        a, b, c = map(root, face)
        parents[b] = parents[c] = a
    return [root(i) for i in range(len(points))]


def prepare(definition):
    if not isinstance(definition, dict):
        raise ValueError("A ship definition must be a JSON object")
    package = dict(definition)
    used = set()
    for key in ("detail", "native"):
        mesh = package.get(key)
        if not isinstance(mesh, dict):
            raise ValueError(f"{key} must contain a mesh")
        package[key] = dict(mesh)
        if "groups" in mesh:
            groups = mesh["groups"]
            if not isinstance(groups, list) or any(type(g) is not int or g < 0 for g in groups):
                raise ValueError(f"{key}.groups must contain nonnegative integers")
            used.update(groups)
    next_id = max(used, default=-1) + 1
    for key in ("detail", "native"):
        mesh = package[key]
        if "groups" not in mesh:
            labels = {}
            mesh["groups"] = [labels.setdefault(component, next_id + len(labels))
                              for component in components(mesh)]
            next_id += len(labels)
    return package


@lru_cache(maxsize=1)
def reference_bytes():
    packaged = HERE / "reference.json"
    if packaged.is_file():
        return packaged.read_bytes()
    folder = ROOT / "assets/fleet/sh023"
    geometry = parse((folder / "SOURCE-GEOMETRY.json").read_bytes())

    def original(name):
        data = (folder / name).read_bytes()
        digest = geometry["pins"].get(name) or geometry["outputs"][name]["sha256"]
        if hashlib.sha256(data).hexdigest() != digest:
            raise ValueError(f"Reference asset differs from the recorded source: {name}")
        return data

    model = tomllib.loads(original("resource.toml").decode())
    physics = parse(original("engineering.json"))
    blob = original("source.vertices")
    _, nd, nn = struct.unpack_from("<8sII", blob)
    points = list(struct.iter_unpack("<3d", blob[16:]))
    faces = list(struct.iter_unpack("<3I", original("source-triangles.bin")))
    memberships = list(original("source-piece-membership.bin"))
    nf = geometry["detail"]["faces"]
    if len(points) != nd + nn or len(memberships) != len(points):
        raise ValueError("Incomplete reference vertices")
    tensor = physics["inertiaAboutCentreOfMassKgM2"]
    routes = [
        ("Axial bow gap", [0,-1.45,18.5], [0,-1.45,8.5]),
        ("Lateral bow gap", [2.35,-1.4,12], [-2.35,-1.4,12]),
        ("Port shoulder", [-2.25,-1.45,17.5], [-2.25,-1.45,8.75]),
        ("Starboard shoulder", [2.25,-1.45,17.5], [2.25,-1.45,8.75]),
    ]
    return encode({
        "schema": 1, "id": model["id"], "name": model["name"],
        "metresPerUnit": 1, "roundingMetres": parse(original("PIECES.json"))["roundingMetres"],
        "detail": {"vertices": points[:nd], "triangles": faces[:nf], "groups": memberships[:nd]},
        "native": {"vertices": points[nd:], "triangles": [[i-nd for i in f] for f in faces[nf:]],
                   "groups": memberships[nd:]},
        "materials": [{"name": m["name"], "rgb": [(m["colour"] >> s) & 255 for s in (24,16,8)],
                       "alpha": m["colour"] & 255, "emissive": m["emissive"]}
                      for m in model["design"]["materials"]],
        "faceMaterials": model["design"]["faceMaterials"],
        "faceTextures": [{"uv": f["uv"], "texture": model["design"]["textures"][f["texture"]]["asset"]
                          if f["texture"] >= 0 else None} for f in model["design"]["faceTextures"]],
        "engineering": {"massKg": physics["dryMassKilograms"], "centerOfMass": physics["centreOfMassMetres"],
                        "inertia": [tensor[i:i+3] for i in (0,3,6)]},
        "flight": {"maxSpeed": model["game"]["speed"]},
        "clearances": [{"name": n, "start": a, "end": b, "radiusMetres": 0.10} for n,a,b in routes],
        "provenance": {"sourceId":"SH023", "revision":geometry["legacyRevision"],
                       "sourceHashes":geometry["pins"], "derivedHashes":geometry["outputs"],
                       "license":original("LICENSE").decode(), "originalMetresPerUnit":geometry["metresPerSourceUnit"]},
    })
