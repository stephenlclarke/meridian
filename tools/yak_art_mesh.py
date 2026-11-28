"""Group the exact original or candidate SH050 artwork by frozen creator parts."""

from collections import Counter
from fractions import Fraction

SCALE = Fraction(400, 41)


def group_labels(mesh, parts):
    labels = []
    occurrences = Counter()
    last_face = 0
    for part in parts:
        name, count = part["name"], part["vertexCount"]
        if part["vertexStart"] != len(labels) or part["faceStart"] != last_face:
            raise ValueError("Changed Yak creator part boundaries")
        first_vertex, last_vertex = len(labels), len(labels) + count
        faces = mesh["faces"][last_face:last_face + part["faceCount"]]
        if len(faces) != part["faceCount"] or not all(
            first_vertex <= index < last_vertex for face in faces for index in face
        ):
            raise ValueError("A Yak face crosses its creator part")
        last_face += len(faces)
        occurrences[name] += 1
        labels.extend([(name, occurrences[name])] * count)
    if len(labels) != len(mesh["vertices"]) or last_face != len(mesh["faces"]):
        raise ValueError("Incomplete Yak creator part inventory")
    return labels


def assemble(detailed, native):
    detail_source, detail_parts = detailed
    native_source, native_parts = native
    first, second = group_labels(detail_source, detail_parts), group_labels(native_source, native_parts)
    slots = {name: index for index, name in enumerate(dict.fromkeys(first + second))}
    return slots, [{
        "vertices": mesh["vertices"], "triangles": mesh["faces"], "groups": [slots[group] for group in names],
    } for mesh, names in ((detail_source, first), (native_source, second))]
