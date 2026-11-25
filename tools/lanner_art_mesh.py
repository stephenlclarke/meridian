"""Partition the archived SH048 bow into overlapping convex support sectors."""

from collections import Counter

ARC_SECTORS = 12


def named_parts(parts):
    count = Counter()
    for part in parts:
        name = part["name"]
        count[name] += 1
        yield (name, count[name]), part


def bow_faces(faces, vertices_per_ring, ring_count):
    for face in faces:
        rings = sorted({index // vertices_per_ring for index in face})
        if len(rings) == 1:
            if rings[0] == 0:
                yield 0
            elif rings[0] == ring_count - 1:
                yield ARC_SECTORS - 1
            else:
                raise ValueError("Interior section in open Lanner bow")
        elif len(rings) == 2 and rings[1] == rings[0] + 1:
            yield ((2 * rings[0] + 1) * ARC_SECTORS) // (2 * (ring_count - 1))
        else:
            raise ValueError("A Lanner bow face spans nonadjacent rings")


def face_groups(mesh, parts, vertices_per_ring):
    names = []
    last_vertex = last_face = 0
    for key, part in named_parts(parts):
        starts = part["vertexStart"], part["faceStart"]
        if starts != (last_vertex, last_face):
            raise ValueError("Changed original part ranges")
        last_vertex += part["vertexCount"]
        last_face += part["faceCount"]
        triangles = mesh["faces"][starts[1]:last_face]
        if not all(starts[0] <= index < last_vertex for face in triangles for index in face):
            raise ValueError("Source face crosses a creator boundary")
        label, occurrence = key
        if label == "open curved loop" and occurrence == 1:
            rings, remainder = divmod(part["vertexCount"], vertices_per_ring)
            if remainder or rings < ARC_SECTORS + 1:
                raise ValueError("Unexpected Lanner bow sampling")
            names.extend((f"open-bow-sector-{index:02d}", 1)
                         for index in bow_faces(triangles, vertices_per_ring, rings))
        else:
            names.extend([key] * len(triangles))
    if last_vertex != len(mesh["vertices"]) or last_face != len(mesh["faces"]):
        raise ValueError("Incomplete creator ranges")
    return names


def partition(mesh, labels, slots):
    vertices, groups, triangles = [], [], []
    old_index_slots = {}
    for face, name in zip(mesh["faces"], labels, strict=True):
        slot = slots[name]
        mapped = []
        for original in face:
            key = original, slot
            derived = old_index_slots.get(key)
            if derived is None:
                derived = len(vertices)
                old_index_slots[key] = derived
                vertices.append(mesh["vertices"][original])
                groups.append(slot)
            mapped.append(derived)
        triangles.append(mapped)
    if {index for index, _ in old_index_slots} != set(range(len(mesh["vertices"]))):
        raise ValueError("Unreferenced original Lanner point")
    for original, derived in zip(mesh["faces"], triangles, strict=True):
        if [mesh["vertices"][index] for index in original] != [vertices[index] for index in derived]:
            raise ValueError("A derived face changed calibrated source order")
        if len({groups[index] for index in derived}) != 1:
            raise ValueError("A derived face spans local supports")
    return {"vertices": vertices, "triangles": triangles, "groups": groups}


def assemble(first, second):
    labels = [face_groups(mesh, parts, perimeter) for mesh, parts, perimeter in (first, second)]
    slots = {name: index for index, name in enumerate(dict.fromkeys(labels[0] + labels[1]))}
    streams = [partition(source[0], names, slots) for source, names in zip((first, second), labels, strict=True)]
    return slots, streams
