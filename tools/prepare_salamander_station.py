"""Preserve the selected Salamander geometry and its full axial docking aperture."""
import argparse
from fractions import Fraction
import math

from catalogue_artwork import ROOT, checked_inputs, component_mesh, persist, visual_definition

FOLDER = ROOT/"integration/sh141-salamander-station"
PLANES = ((0,-1,32), (0,1,32), (1,-1,18), (1,1,18))


def separation(points, plane):
    axis, sign, limit = plane
    return min(sign*p[axis] for p in points) - limit


def split_rings(model, start):
    components, _ = component_mesh(model, 0)
    labels = components["groups"]
    clouds = {}
    for label, point in zip(labels, model["vertices"], strict=True):
        clouds.setdefault(label, []).append(point)
    wraps_port = {g for g,p in clouds.items() if max(separation(p,h) for h in PLANES) < 1}
    vertices, faces, groups = [], [], []
    sectors, corner_lookup = {}, {}
    for face in model["faces"]:
        points = [model["vertices"][v] for v in face]
        label = labels[face[0]]
        key = (label,)
        if label in wraps_port:
            plane = max(range(4), key=lambda i: separation(points, PLANES[i]))
            if separation(points, PLANES[plane]) < 1:
                raise ValueError("Salamander face enters the reserved aperture margin")
            x,y = (sum(p[i] for p in points) for i in (0,1))
            sector = math.floor(math.atan2(y, x) * 8 / math.pi)
            key = (label, plane, sector)
        group = sectors.setdefault(key, start+len(sectors))
        indices = []
        for v in face:
            corner = group, v
            if corner not in corner_lookup:
                corner_lookup[corner] = len(vertices)
                vertices.append(model["vertices"][v])
                groups.append(group)
            indices.append(corner_lookup[corner])
        faces.append(indices)
    return dict(vertices=vertices, triangles=faces, groups=groups), start+len(sectors)


def aperture_margin(package):
    """A separating half-space bounds every convex piece for the entire axial sweep."""
    clouds = {}
    for level in ("detail","native"):
        mesh=package[level]
        for group, point in zip(mesh["groups"],mesh["vertices"],strict=True):
            clouds.setdefault(group,[]).append(point)
    gap = min(max(separation(points,h) for h in PLANES) for points in clouds.values())
    return Fraction(gap) - Fraction(1,50)


def definition():
    receipt, source, archive, scale, notice = checked_inputs(FOLDER/"SOURCE.json")
    docking = source["nativeDocking"]
    corners = {(c["x"],c["y"],c["z"]) for c in docking["aperture"]}
    if corners != {(x,y,0) for x in (-32,32) for y in (-18,18)} or docking["center"]["z"] != 39 or docking["tunnelDepth"] != 119:
        raise ValueError("Changed source station entrance")
    current = source["nativeDesign"]
    archived = archive["nativeDesign"]
    if (len(current["faces"]),len(archived["faces"])) != (416,392) or current["vertices"][:224] != archived["vertices"]:
        raise ValueError("Changed original native revisions")
    high, first = split_rings(source["design"],0)
    low, _ = split_rings(current,first)
    package = visual_definition(receipt,source,archive,scale,notice,high,low)
    package["schema"] = 1
    package["provenance"]["selectedNative"] = "Committed station geometry with two gate struts"
    package["clearances"] = [
        dict(name=f"axial-{i}", start=[x,y,-80], end=[x,y,160], radiusMetres=0.5)
        for i,(x,y) in enumerate([(0,0),(-32,-18),(-32,18),(32,-18),(32,18)])]
    if aperture_margin(package) < Fraction(98,100):
        raise ValueError("Rounded station supports reduce the source passage")
    return package


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    persist(definition(), FOLDER/"SALAMANDER-STUDIO.json", args.check)
