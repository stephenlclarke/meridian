"""Preserve each original Dzo collar triangle with overlapping local supports."""

COLLARS = tuple(f"{side}-{end}-collar" for side in ("port", "starboard") for end in ("aft", "forward"))
SPAN_COUNT = 8
ABSENT = 255


def collar_spans(indices, detailed_count, count=SPAN_COUNT):
    all_indices = [int(index) for index in indices]
    vertices = ([i for i in all_indices if i < detailed_count], [i for i in all_indices if i >= detailed_count])
    spans = [[] for _ in range(count)]
    for rings, width, major in zip(vertices, (12, 4), (64, 8), strict=True):
        if len(rings) != width * major or major % count:
            raise ValueError("Unexpected original Dzo collar sampling")
        step = major // count
        for span, vertex_ids in enumerate(spans):
            for ring in range(span * step, (span + 1) * step + 1):
                first = (ring % major) * width
                vertex_ids.extend(rings[first:first + width])
    return [sorted(indices) for indices in spans]


def collision_geometry(names, membership, triangles, detailed_count):
    groups = {}
    for slot, name in enumerate(names):
        indices = [i for i, value in enumerate(membership) if value == slot]
        if name in COLLARS:
            for span, vertices in enumerate(collar_spans(indices, detailed_count), 1):
                groups[f"{name}-span-{span}"] = vertices
        else:
            groups[name] = indices

    slots = [[] for _ in membership]
    for slot, indices in enumerate(groups.values()):
        for index in indices:
            slots[index].append(slot)
    if len(groups) != len(names) + 4 * (SPAN_COUNT - 1) or not all(1 <= len(row) <= 2 for row in slots):
        raise ValueError("Unexpected Dzo collision support inventory")
    if sum(len(row) == 2 for row in slots) != 512:
        raise ValueError("Unexpected Dzo collar seam inventory")
    if not all(set.intersection(*(set(slots[i]) for i in face)) for face in triangles):
        raise ValueError("An original Dzo triangle crosses the selected supports")
    packed = bytes(slot for row in slots for slot in (row if len(row) == 2 else [row[0], ABSENT]))
    return list(groups), packed
