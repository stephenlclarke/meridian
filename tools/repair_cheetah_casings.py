"""Resample only Cheetah's two propulsion casings from their original twelve-point rings."""
import hashlib
import json
import tomllib


HISTORICAL_SHA = "360a51940b58c726301ba4398166405b5c2d78291f8d653f68dd3ab170f452bf"
SECTORS = (1, 4, 7, 10)


def casing_texture(vertices, face):
    # The original creator projects onto the dominant face plane at 0.85 repeat units.
    a, b, c = (vertices[i] for i in face)
    u, v = ([x-y for x, y in zip(p, a)] for p in (b, c))
    normal = (u[1]*v[2]-u[2]*v[1], u[2]*v[0]-u[0]*v[2], u[0]*v[1]-u[1]*v[0])
    axis = max(range(3), key=lambda i: abs(normal[i]))
    plane = ((2, 1), (0, 2), (0, 1))[axis]
    return {"texture": 2, "uv": [[round(p[i]*0.85, 6) for i in plane] for p in (a, b, c)]}


def repair(folder):
    prior = (folder / "historical-504.json").read_bytes()
    if hashlib.sha256(prior).hexdigest() != HISTORICAL_SHA:
        raise ValueError("Changed historical candidate")
    source = tomllib.loads((folder/"resource.toml").read_text())["nativeDesign"]
    archive = json.loads((folder/"native-export.json").read_bytes())
    if source != archive["nativeDesign"] or archive["id"] != "SH082":
        raise ValueError("Changed original native mesh")
    ownership = json.loads((folder/"ownership.json").read_bytes())["models"]["nativeDesign"]["parts"]
    plan = json.loads(prior)
    old = plan["nativeCandidate"]
    model = dict(old, vertices=[], faces=[], faceMaterials=[], faceTextures=[])
    parts = []
    for part, original in zip(plan["parts"], ownership, strict=True):
        v, f = part["vertexStart"], part["faceStart"]
        stop = f + part["faceCount"]
        points = old["vertices"][v:v+part["vertexCount"]]
        faces = [[i-v for i in tri] for tri in old["faces"][f:stop]]
        mats, uvs = old["faceMaterials"][f:stop], old["faceTextures"][f:stop]
        if part["name"].endswith("/propulsion casing"):
            if original["vertexCount"] != 24 or original["name"] != part["name"]:
                raise ValueError("Changed original casing layout")
            start = original["vertexStart"]
            points = [source["vertices"][start + ring + i] for ring in (0, 12) for i in SECTORS]
            faces = [[3, 2, 1], [3, 1, 0]]
            for i in range(4):
                j = (i+1)%4
                faces.extend([[i, j, j+4], [i, j+4, i+4]])
            faces.extend([[4, 5, 6], [4, 6, 7]])
            mats = [1]*len(faces)
            uvs = [casing_texture(points, tri) for tri in faces]
        first = len(model["vertices"])
        parts.append(dict(part, vertexStart=first, vertexCount=len(points),
                          faceStart=len(model["faces"]), faceCount=len(faces)))
        model["vertices"].extend(points)
        model["faces"].extend([[first+i for i in tri] for tri in faces])
        model["faceMaterials"].extend(mats)
        model["faceTextures"].extend(uvs)
    if len(model["faces"]) != 512 or len(parts) != 29:
        raise ValueError("Changed Cheetah face or part count")
    return model, parts
