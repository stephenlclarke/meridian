"""Check preserved artwork and the independent Medusa inspection gates."""

from hashlib import sha256
import json
import unittest

import prepare_medusa_inspection as medusa


class MedusaInspection(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source, cls.parts, cls.engineering, _, cls.owners, cls.points, cls.groups = medusa.shield_inputs()

    def test_original_artwork_and_record_are_bound(self):
        detailed, archived = (self.source[name] for name in medusa.MESH_NAMES)
        self.assertEqual(tuple(len(mesh["faces"]) for mesh in (detailed, archived)), (162548, 76228))
        self.assertEqual(detailed["materials"], archived["materials"])
        self.assertEqual([m["name"] for m in detailed["materials"] if m["emissive"]], ["bioluminescent gland"])
        for kind in medusa.MESH_NAMES:
            dots = [part for part in self.parts[kind]["parts"]
                    if part["name"].startswith("appended luminous projection dot")]
            self.assertEqual(len(dots), 510)
            self.assertTrue(all(part["allowedSourceMaterials"] == [6] for part in dots))
        self.assertEqual(self.engineering["dryMassKilograms"], 40712)
        self.assertTrue(self.engineering["nativeQualification"].startswith("UNRUN"))
        self.assertFalse(self.source["catalogue"]["playerFlyable"])
        self.assertFalse(self.source["catalogue"]["normalPurchaseEligible"])

    def test_generated_visual_and_points_replay_exactly(self):
        for name, actual in medusa.prepare().items():
            self.assertEqual(sha256(actual).hexdigest(), sha256((medusa.ASSETS / name).read_bytes()).hexdigest(), name)

    def test_shield_and_fit_receipt_bind_to_exact_source(self):
        shield = medusa.source_fans(self.points, self.groups, author=False)
        fans = list(medusa.read_fans(shield, self.groups))
        self.assertEqual(len(fans), 729)
        self.assertEqual(sum("/span-" in name and "living crown" in name for name, _, _ in fans), 16)
        fit = json.loads((medusa.EVIDENCE / "SHIELD-FIT.json").read_bytes())
        self.assertEqual(fit["shieldSha256"], sha256(shield).hexdigest())
        self.assertEqual([r["support"] for r in fit["supports"]], [name for name, _, _ in fans])
        self.assertEqual(fit["roundingMillimetres"], 20)
        self.assertEqual(fit["maximumOuterFitUpperMillimetres"], 1394)
        self.assertGreaterEqual(fit["crownAxisClearanceLowerMillimetres"], 5800)

    def test_studio_keeps_both_archived_triangle_sequences_and_materials(self):
        package = medusa.studio_definition()
        self.assertEqual(package["schema"], 2)
        self.assertIsNone(package["engineering"])
        self.assertEqual(package["flight"], {})
        for key, mesh in zip(("detail", "native"), (self.source[n] for n in medusa.MESH_NAMES), strict=True):
            inspected = package[key]
            for initial, final in zip(mesh["faces"], inspected["triangles"], strict=True):
                self.assertEqual([mesh["vertices"][i] for i in initial], [inspected["vertices"][i] for i in final])
            materials = "faceMaterials" if key == "detail" else "nativeFaceMaterials"
            self.assertEqual(package[materials], mesh["faceMaterials"])
        self.assertGreater(len(package["native"]["triangles"]), package["provenance"]["flightNativeFaceLimit"])


if __name__ == "__main__":
    unittest.main()
