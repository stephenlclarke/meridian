"""Allocate Tapir Mk II's inspection budget using original vertices and overlap constraints."""

from collections import defaultdict
from fractions import Fraction
import hashlib
import json
import tomllib

import numpy as np
from scipy.spatial import ConvexHull, QhullError
import trimesh

from import_goshawk_envelope_source import components
from prepare_tapir_mk_ii import INTEGRATION, ORIGINAL

from measure_banteng_geometry import section, solid
from verify_eagle_cruiser_geometry import inside_y

SCALE = float(Fraction(7600000, 252127))
BUDGET = 512
SOURCE_SHA256 = "280c0ad039985f509ccb4c4f3778c3c2f4ca2e81ae9db60af69c623e4f9ac16c"
OWNER_SHA256 = "3720ad2f6ffc46ff1673f5649f0980101a33054dc87a8e3b271d5d0a1efc753a"


def model_parts(model):
    parted = []
    for vertices in components(model):
        remap = {index: i for i, index in enumerate(vertices)}
        faces = [(i, [remap[v] for v in row]) for i, row in enumerate(model["faces"]) if row[0] in remap]
        mesh = trimesh.Trimesh([model["vertices"][v] for v in vertices], [f for _, f in faces], process=False)
        if parted:
            solid(mesh)
        elif mesh.euler_number != 0 or not mesh.is_watertight or not mesh.is_winding_consistent or mesh.volume <= 0:
            raise ValueError("Tapir Mk II loop is not a closed torus")
        parted.append((mesh, [i for i, _ in faces]))
    return parted


def hull_of(points, indices):
    result = ConvexHull(points[indices])
    assert len(result.vertices) == len(indices)
    return result


def plane_distance(points, hull):
    return np.max(points @ hull.equations[:, :3].T + hull.equations[:, 3], axis=1)


def start_indices(points, options):
    indices = [min(options, key=lambda i: points[i, 2]),
               max(options, key=lambda i: points[i, 2])]
    a, b = points[indices]
    indices.append(max(options, key=lambda i: np.linalg.norm(np.cross(points[i]-a,b-a))))
    c = points[indices[-1]]
    normal = np.cross(b-a,c-a)
    indices.append(max(options, key=lambda i: abs((points[i]-a) @ normal)))
    return sorted(set(indices))


def minimize(points, witnesses, ids):
    while len(ids) > 4:
        choices = []
        for remove in ids:
            test = [i for i in ids if i != remove]
            subset = points[test,2]
            if subset.min()!=points[:,2].min() or subset.max()!=points[:,2].max():
                continue
            try:
                hull = hull_of(points,test)
            except QhullError:
                continue
            if len(witnesses) and max(plane_distance(witnesses,hull)) >= -.0001:
                continue
            choices.append((max(plane_distance(points,hull)), test))
        if not choices:
            break
        _, ids = min(choices)
    return ids


def choose_addition(points, indices, options, required):
    remaining = [i for i in options if i not in indices]
    if not remaining:
        return None
    best = None
    for index in remaining:
        ids = sorted([*indices, index])
        hull = hull_of(points, ids)
        error = np.maximum(plane_distance(points,hull),0)
        missing = np.maximum(plane_distance(required,hull)+.0001,0) if len(required) else []
        score = (max(missing,default=0.), float(max(error)), float(np.mean(error**2)), index)
        if best is None or score < best[0]:
            best = (score, ids)
    return best


COLLARS = (0,)
MAJOR = tuple(range(24))
MINOR = tuple(range(0, 8, 2))


def collar_section(row):
    vertices, faces, materials, mappings = row
    chosen = [vertices[major*8+minor] for major in MAJOR for minor in MINOR]
    points = np.array(chosen)
    faces = []
    width, height = len(MINOR), len(MAJOR)
    for major in range(height):
        after = (major+1) % height
        for minor in range(width):
            following = (minor+1) % width
            a, b = major*width+minor, after*width+minor
            c, d = after*width+following, major*width+following
            faces.extend([[a,b,c],[a,c,d]])
    mesh = trimesh.Trimesh(points, faces, process=False)
    if mesh.volume < 0:
        faces = [face[::-1] for face in faces]
        mesh = trimesh.Trimesh(points, faces, process=False)
    assert mesh.euler_number == 0 and mesh.is_watertight and mesh.is_winding_consistent and mesh.volume > 0
    mat = set(materials).pop()
    uv = {}
    for old_face, mapping in zip(row[1], mappings, strict=True):
        for original, uv_value in zip(old_face, mapping["uv"], strict=True):
            uv.setdefault(original, uv_value)
    ids = [major*8+minor for major in MAJOR for minor in MINOR]
    mapped = [{"texture": mappings[0]["texture"], "uv": [uv[ids[i]] for i in face]} for face in faces]
    return chosen, faces, [mat]*len(faces), mapped


def exact_faces(mesh):
    return [[tuple(map(Fraction, vertex)) for vertex in face] for face in mesh.triangles.tolist()]


def common_contact(meshes, original_witness):
    if all(mesh.contains([original_witness])[0] for mesh in meshes):
        candidates = [original_witness]
    else:
        lo = np.maximum.reduce([m.bounds[0] for m in meshes])
        hi = np.minimum.reduce([m.bounds[1] for m in meshes])
        assert np.all(lo < hi)
        rng = np.random.default_rng(98)
        samples = rng.uniform(lo,hi,(24576,3)).round(7)
        for mesh in meshes:
            if len(samples):
                samples = samples[mesh.contains(samples)]
        candidates = samples.tolist()
    surfaces = [exact_faces(mesh) for mesh in meshes]
    for point in candidates:
        try:
            parity = [inside_y(tuple(map(Fraction,point)), faces) for faces in surfaces]
        except ValueError:
            continue
        if all(n % 2 for row in parity for n in row):
            return point
    return None


def constrain_collars(native, collars, archive):
    original = model_parts(dict(native, vertices=[[axis * SCALE for axis in point] for point in native["vertices"]]))
    constraints = []
    for row in archive:
        a,b = row["parts"]
        point = row["pointMetres"]
        torus = next((i for i in (a,b) if i in collars), None)
        meshes = [original[a][0], original[b][0]]
        if torus is not None:
            vertices, triangles = collars[torus][:2]
            reduced = trimesh.Trimesh(np.array(vertices)*SCALE, triangles, process=False)
            shared = common_contact([*meshes,reduced], point)
            if shared is None:
                raise ValueError(f"Collar angular sampling loses contact {(a,b)}")
            point = shared
        parity = [inside_y(tuple(map(Fraction,point)), exact_faces(mesh)) for mesh in meshes]
        constraints.append({"parts":row["parts"], "pointMetres":point, "hitCountsAboveBelow":parity})
    return constraints


def shape_budget(sections, parts, overlaps, collars):
    required = defaultdict(list)
    for row in overlaps:
        for index in row["parts"]:
            required[index].append(row["pointMetres"])
    states = {}
    for index, (part, row) in enumerate(zip(parts,sections,strict=True)):
        if part["creatorMethod"] == "Mesh.strut / Hull.box" or part["creatorMethod"] == "Mesh.well" or index in collars:
            continue
        points = np.array(row[0])*SCALE
        witnesses = np.asarray(required[index]).reshape(-1,3)
        options = sorted(map(int,ConvexHull(points).vertices))
        ids = start_indices(points,options)
        while len(witnesses) and max(plane_distance(witnesses,hull_of(points,ids))) >= -.0001:
            _, ids = choose_addition(points,ids,options,witnesses)
        states[index] = [points, witnesses, options, minimize(points,witnesses,ids)]
    fixed = sum(part["faceCount"] for part in parts if part["index"] not in states and part["index"] not in collars)
    fixed += sum(len(row[1]) for row in collars.values())
    def faces():
        return fixed + sum(2*len(ids)-4 for *_,ids in states.values())
    print("minimum",faces(), {i:len(state[-1]) for i,state in states.items()},flush=True)
    assert faces() <= BUDGET, ("constraints require", faces())
    while faces() < BUDGET:
        def worst(state):
            points, _, _, ids = state
            return max(plane_distance(points,hull_of(points,ids)))
        growable = [i for i,state in states.items() if len(state[2])>len(state[3])]
        index = max(growable, key=lambda i: worst(states[i]))
        points, witnesses, options, ids = states[index]
        _, ids = choose_addition(points, ids, options, witnesses)
        states[index][3] = ids
    print("faces", faces(), "selected sizes", {i:len(row[3]) for i,row in states.items()},flush=True)
    return {i:row[3] for i,row in states.items()}


def rebuilt_section(row, ids):
    vertices, faces, materials, mappings = row
    src = np.asarray(vertices)
    chosen = src[ids]
    center = np.mean(chosen, axis=0)
    original = trimesh.Trimesh(src,faces,process=False)
    patch = []
    for a,b,c in ConvexHull(chosen).simplices.tolist():
        if np.dot(np.cross(chosen[b]-chosen[a],chosen[c]-chosen[a]), chosen[a]-center) < 0:
            b,c=c,b
        face = [a,b,c]
        shift = face.index(min(face))
        patch.append(face[shift:] + face[:shift])
    patch.sort()
    mesh = trimesh.Trimesh(chosen,patch,process=False)
    _, _, nearest = original.nearest.on_surface(mesh.triangles_center)
    # A simplified well can collapse several source texture faces into one
    # facet. Assign each missing material to the facet with the nearest
    # original surface carrying that material. Leave the original artwork intact.
    counts = {material: sum(materials[index] == material for index in nearest) for material in set(materials)}
    for material, count in counts.items():
        if count:
            continue
        source_indices = [i for i,value in enumerate(materials) if value == material]
        targets = [i for i,index in enumerate(nearest) if counts[materials[index]] > 1]
        assignment = min(
            ((np.linalg.norm(mesh.triangles_center[i]-original.triangles_center[j]), i, j)
             for i in targets for j in source_indices),
        )
        _, chosen_face, source_index = assignment
        counts[materials[nearest[chosen_face]]] -= 1
        nearest[chosen_face] = source_index
        counts[material] += 1
    selected_materials = [materials[index] for index in nearest]
    selected_mappings = []
    for face, index in zip(patch,nearest,strict=True):
        mapping = mappings[index]
        original_triangle = np.array(faces[index])
        closest = [np.linalg.norm(src[original_triangle]-chosen[i],axis=1).argmin() for i in face]
        selected_mappings.append({"texture": mapping["texture"], "uv": [mapping["uv"][i] for i in closest]})
    solid(mesh)
    return [vertices[i] for i in ids],patch,selected_materials,selected_mappings


def main():
    resource = (ORIGINAL / "resource.toml").read_bytes()
    owner = (INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes()
    assert hashlib.sha256(resource).hexdigest() == SOURCE_SHA256
    assert hashlib.sha256(owner).hexdigest() == OWNER_SHA256
    original = tomllib.loads(resource.decode())["nativeDesign"]
    ownership = json.loads(owner)
    parts = ownership["models"]["nativeDesign"]["parts"]
    archive = json.loads((INTEGRATION / "ARCHIVED-OVERLAPS.json").read_bytes())["interiorOverlaps"]
    assert len(parts) == 14 and len(archive) == 7
    sections = [section(original,part) for part in parts]
    collars = {index:collar_section(sections[index]) for index in COLLARS}
    archive = constrain_collars(original,collars,archive)
    selected = shape_budget(sections,parts,archive,collars)
    keys = ("vertices", "faces", "faceMaterials", "faceTextures")
    output = original | {key:[] for key in keys}
    bindings = []
    detached = []
    exact_parts = []
    for index, (part, row) in enumerate(zip(parts,sections,strict=True)):
        ids = selected.get(index)
        derived = collars[index] if index in collars else rebuilt_section(row,ids) if ids is not None else row
        vertices, faces, materials, mappings = derived
        exact_parts.append([[tuple(Fraction(c*SCALE) for c in vertices[i]) for i in face] for face in faces])
        if set(materials) != set(row[2]):
            detached.append(index)
        start = len(output["vertices"])
        bindings.append({key:part[key] for key in ("index","name","creatorMethod")} | {
            "vertexStart": start, "vertexCount":len(vertices),
            "faceStart": len(output["faces"]), "faceCount":len(faces),
        })
        output["vertices"].extend(vertices)
        output["faces"].extend([[i+start for i in face] for face in faces])
        output["faceMaterials"].extend(materials)
        output["faceTextures"].extend(mappings)
    witnesses = []
    for row in archive:
        point = tuple(map(Fraction,row["pointMetres"]))
        hits = [inside_y(point,exact_parts[i]) for i in row["parts"]]
        assert all(all(n % 2 for n in result) for result in hits), row["parts"]
        witnesses.append(row | {"candidateHitCountsAboveBelow":hits})
    assert len(output["faces"]) == BUDGET
    print("retained",len(witnesses),"material subsets",detached,flush=True)
    result = {
        "id":"SH106", "role":"derived native inspection candidate",
        "originalSourceSHA256":ownership["geometrySHA256"], "r3SHA256":ownership["contractSHA256"],
        "selectedSourceVertexIndices":selected, "collars":{"majorIndices":MAJOR,"minorIndices":MINOR}, "wellMaterialSubsets":detached,
        "nativeCandidate":output, "parts":bindings,
    }
    if ((json.dumps(result, separators=(",", ":")) + "\n").encode()
            != (ORIGINAL / "GENERATED-COLLISION.json").read_bytes()):
        raise ValueError("Tapir Mk II candidate replay differs")
    if ((json.dumps(witnesses, indent=2) + "\n").encode()
            != (INTEGRATION / "RETAINED-OVERLAPS.json").read_bytes()):
        raise ValueError("Tapir Mk II overlap replay differs")


if __name__ == "__main__":
    main()
