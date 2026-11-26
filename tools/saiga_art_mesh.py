"""Keep the signed Saiga mesh groups and sectorize the actual collar supports."""

from fractions import Fraction

from banteng_art_mesh import assemble

SCALE = Fraction(20)
COLLAR = "radial structural collar"
ABSENT_SECONDARY = 255


def collision_groups(detailed, native, artwork_slots, meshes, *, archived=False):
    old_membership = meshes[0]["groups"] + meshes[1]["groups"]
    groups = {
        f"{name}[{ordinal}]": [i for i, slot in enumerate(old_membership) if slot == identifier]
        for (name, ordinal), identifier in artwork_slots.items() if name != COLLAR
    }
    native_offset = len(meshes[0]["vertices"])
    for (model, parts), width, offset in ((detailed, 12, 0), (native, 8 if archived else 4, native_offset)):
        collars = [part for part in parts if part["name"] == COLLAR]
        if len(collars) != 2:
            raise ValueError("wrong Saiga collar count")
        for ordinal, collar in enumerate(collars, 1):
            major, remainder = divmod(collar["vertexCount"], width)
            if major != (48 if offset == 0 else 24 if archived else 8) or remainder:
                raise ValueError("changed Saiga collar major stations")
            step = major // 8
            start = collar["vertexStart"] + offset
            sectors = []
            for sector in range(8):
                first_ring = sector * step
                end_ring = (sector + 1) * step + 1
                indices = {
                    start + (ring % major) * width + side
                    for ring in range(first_ring, end_ring) for side in range(width)
                }
                groups.setdefault(f"{COLLAR}[{ordinal}]-sector-{sector+1}", []).extend(sorted(indices))
                sectors.append(indices)
            span = slice(collar["faceStart"], collar["faceStart"] + collar["faceCount"])
            if not all(any({offset + corner for corner in face} <= sector for sector in sectors)
                       for face in model["faces"][span]):
                raise ValueError("Saiga collar triangle crosses sector supports")

    if len(groups) != 72:
        raise ValueError("wrong Saiga collision support count")
    return groups


def collision_membership(vertex_count, groups):
    slots = [[] for _ in range(vertex_count)]
    for slot, indices in enumerate(groups.values()):
        for index in indices:
            slots[index].append(slot)
    if not all(1 <= len(row) <= 2 for row in slots):
        raise ValueError("incomplete Saiga collision membership")
    return bytes(slot for row in slots for slot in (row if len(row) == 2 else [row[0], ABSENT_SECONDARY]))
