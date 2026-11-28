"""Assign the original Banteng artwork to local convex supports."""

from collections import Counter
from fractions import Fraction

SCALE = Fraction(20_000, 1_661)


def group_labels(mesh, records):
    labels = []
    occurrences = Counter()
    last_face = 0
    current_well = None
    for part in records:
        name, count = part["name"], part["vertexCount"]
        if part["vertexStart"] != len(labels) or part["faceStart"] != last_face:
            raise ValueError("Changed Banteng creator part boundaries")
        first_vertex, last_vertex = len(labels), len(labels) + count
        faces = mesh["faces"][last_face:last_face + part["faceCount"]]
        if len(faces) != part["faceCount"] or not all(
            first_vertex <= index < last_vertex for face in faces for index in face
        ):
            raise ValueError("A Banteng face crosses its creator part")
        last_face += len(faces)
        occurrences[name] += 1
        if "/recess-service-strut-" in name:
            parent = name.rsplit("/", 1)[0]
            if current_well is None or current_well[0] != parent:
                raise ValueError("A Banteng recess rim left its local well")
            group = current_well
        else:
            group = (name, part.get("originalOccurrence", occurrences[name]))
            current_well = group if "/service-well-" in name else None
        labels.extend([group] * count)
    if len(labels) != len(mesh["vertices"]) or last_face != len(mesh["faces"]):
        raise ValueError("Incomplete Banteng creator part inventory")
    return labels


def assemble(detailed, native):
    detailed_mesh, detailed_parts = detailed
    native_mesh, native_parts = native
    detail = group_labels(detailed_mesh, detailed_parts)
    low = group_labels(native_mesh, native_parts)
    groups = {name: index for index, name in enumerate(dict.fromkeys(detail + low))}
    return groups, [
        {"vertices": mesh["vertices"], "triangles": mesh["faces"], "groups": [groups[x] for x in labels]}
        for mesh, labels in ((detailed_mesh, detail), (native_mesh, low))
    ]
