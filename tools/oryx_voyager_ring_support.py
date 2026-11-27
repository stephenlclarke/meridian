"""Cover the original ring-wing faces with local overlapping spans."""

from dzo_collar_support import ABSENT, SPAN_COUNT, collar_spans


def ring_supports(names, artwork_membership, triangles, detailed_count):
    groups = {}
    for slot, name in enumerate(names):
        vertices = [i for i, member in enumerate(artwork_membership) if member == slot]
        if name == "ring-wing":
            spans = collar_spans(vertices, detailed_count)
            for span, indices in enumerate(spans, 1):
                groups[f"ring-wing-span-{span}"] = indices
        else:
            groups[name] = vertices

    memberships = [set() for _ in artwork_membership]
    for group, indices in enumerate(groups.values()):
        for index in indices:
            memberships[index].add(group)
    if len(groups) != len(names) + SPAN_COUNT - 1 or not all(1 <= len(row) <= 2 for row in memberships):
        raise ValueError("Unexpected Oryx Voyager support inventory")
    if sum(len(row) == 2 for row in memberships) != 128:
        raise ValueError("Unexpected Oryx Voyager ring seam inventory")
    if not all(set.intersection(*(memberships[i] for i in face)) for face in triangles):
        raise ValueError("An original Oryx Voyager triangle crosses the selected ring supports")
    packed = bytes(slot for members in memberships for slot in (
        sorted(members) if len(members) == 2 else [next(iter(members)), ABSENT]
    ))
    return list(groups), packed
