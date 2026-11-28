"""Verify the archived Cheetah contacts and the native casing repair."""
from fractions import Fraction
import json
import unittest

from hull_contact_geometry import inside_y
from prepare_cheetah_shield import ASSETS, OUTPUT, PACKAGE, definition, encode, inputs
from repair_cheetah_casings import repair


def crossings(model, parts, pair, point, scale):
    vertices = [tuple(Fraction(x*scale) for x in p) for p in model["vertices"]]
    hits = []
    for index in pair:
        part = parts[index]
        first = part["faceStart"]
        faces = model["faces"][first:first+part["faceCount"]]
        hits.append(list(inside_y(tuple(map(Fraction, point)),
                                  [[vertices[v] for v in face] for face in faces])))
    return hits


class CheetahShieldTests(unittest.TestCase):
    def test_source_package_and_repaired_native_rebuild(self):
        package = definition()
        self.assertEqual(encode(package), PACKAGE.read_bytes())
        model, parts = repair(ASSETS)
        self.assertEqual(encode({"nativeCandidate": model, "parts": parts}), (ASSETS/"repaired-native.json").read_bytes())
        self.assertEqual(len(model["faces"]), 512)
        self.assertEqual(len(package["detail"]["triangles"]), 3508)
        self.assertEqual(len(package["clearances"]), 6)

    def test_all_original_contacts_and_four_previously_lost_points(self):
        source, original_parts, _, _ = inputs()
        archive = source["nativeDesign"]
        self.assertEqual(len(archive["faces"]), 988)
        model, parts = repair(ASSETS)
        contacts = json.loads((OUTPUT/"REPAIRED-CONTACTS.json").read_bytes())["contacts"]
        self.assertEqual((len(contacts["retained"]), contacts["lost"], contacts["unresolved"]), (24, [], []))
        previously_lost = {(8,20), (11,20), (13,23), (16,23)}
        checked = set()
        for row in contacts["retained"]:
            pair = tuple(row["parts"])
            checks = [(archive, original_parts["nativeDesign"]["parts"], "archivedOverlap"),
                      (model, parts, "candidateOverlap")]
            if pair in previously_lost:
                checks.append((model, parts, "archivedOverlap"))
                checked.add(pair)
            for mesh, part_rows, key in checks:
                hits = crossings(mesh, part_rows, pair, row[key]["pointMetres"], 7500/1081)
                self.assertEqual(hits, [[1, 1], [1, 1]], (pair, key))
        self.assertEqual(checked, previously_lost)
