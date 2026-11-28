"""Check the frozen SH078 original meshes and the generated inspection package."""

import hashlib
import json
import struct
import tomllib
import unittest
from unittest.mock import patch

import prepare_oryx_voyager as prep
import verify_oryx_voyager as proof


class OryxVoyagerSources(unittest.TestCase):
    def test_original_archive_pins(self):
        provenance, inputs, resource, detail, native = prep.load_source()
        self.assertEqual(provenance["tree"], "68fec574df12ddf74acb14747e0dd6839f478642")
        for name, data in inputs.items():
            with self.subTest(path=name):
                record = provenance["paths"][name]
                self.assertEqual(record["bytes"], len(data))
                self.assertEqual(record["sha256"], hashlib.sha256(data).hexdigest())
                self.assertEqual(record["blob"], hashlib.sha1(f"blob {len(data)}\0".encode() + data).hexdigest())
        self.assertFalse(resource["catalogue"]["playerFlyable"])
        self.assertFalse(resource["catalogue"]["normalPurchaseEligible"])
        access = tomllib.loads(inputs["contract.toml"].decode())["access"]
        self.assertTrue(access["playerFlyable"])
        self.assertEqual(access["purchasePolicy"], "blueprintFabrication")
        criteria = json.loads((prep.INTEGRATION / "CRITERIA.json").read_bytes())["originalAccess"]
        self.assertEqual(criteria["runtimeCatalogue"], {
            key: resource["catalogue"][key] for key in ("playerFlyable", "normalPurchaseEligible", "purchasePolicy")
        })
        self.assertEqual(criteria["r3DesignContract"], {
            key: access[key] for key in ("playerFlyable", "purchasePolicy")
        })
        self.assertEqual((len(detail["faces"]), len(native["faces"])), (4620, 200))

    def test_presentation_keeps_both_meshes_and_untextured_faces(self):
        _, _, _, detail, native = prep.load_source()
        derived = prep.prepare()
        for model, name in ((detail, "visual.bin"), (native, "native-visual.bin")):
            with self.subTest(path=name):
                stream = derived[name]
                self.assertEqual((prep.ORIGINAL / name).read_bytes(), stream)
                self.assertEqual(struct.unpack_from("<8sII", stream), (b"ORYXVIZ1", len(model["faces"]) * 3, 56))
                corners = iter(struct.iter_unpack("<12fII", stream[16:]))
                for face, material_index, mapping in zip(
                    model["faces"], model["faceMaterials"], model["faceTextures"], strict=True
                ):
                    material = model["materials"][material_index]
                    color = [(material["colour"] >> shift & 255) / 255 for shift in (24, 16, 8, 0)]
                    for vertex, uv in zip(face, mapping["uv"], strict=True):
                        row = next(corners)
                        expected = struct.unpack("<9f", struct.pack("<9f", *model["vertices"][vertex], *color, *uv))
                        self.assertEqual(row[:3] + row[6:12], expected)
                        self.assertEqual(row[12:], (mapping["texture"] & 0xffff_ffff, int(material["emissive"])))
                self.assertIsNone(next(corners, None))

    def test_exact_contact_and_fit_receipts(self):
        result = proof.verify()
        self.assertEqual(prep.encode_json(result), (prep.INTEGRATION / "CONTACT-RESULT.json").read_bytes())
        self.assertEqual(sorted(result["collisionClearanceMillimetres"].values()), [1229, 1343, 1343, 1407, 1811, 1811, 2608])
        self.assertEqual(len(result["sourceJointNames"]), 5)
        self.assertEqual(result["collisionClearanceMillimetres"]["port-inner-ring"], 1811)
        self.assertEqual(result["collisionClearanceMillimetres"]["starboard-inner-ring"], 1811)
        self.assertEqual(result["occupiedCentreHull"]["hitCountsAboveBelow"], {"detailed": [1, 1], "native": [1, 1]})
        self.assertEqual([row["eulerCharacteristic"] for row in result["ringTopology"].values()], [0, 0])
        support_geo = json.loads((prep.INTEGRATION / "RING-OPTIONS.json").read_bytes())
        self.assertEqual(support_geo["ring"]["8SpansUpperMillimetres"], {"detailed": 3286, "native": 3476})
        assignment = (prep.ORIGINAL / "collision-piece-membership.bin").read_bytes()
        self.assertEqual(sum(slot != 255 for slot in assignment[1::2]), 128)
        criteria = json.loads((prep.INTEGRATION / "CRITERIA.json").read_bytes())
        for command in criteria["byteReplay"]["commands"]:
            _, separator, target = command.partition(" | cmp - ")
            if separator:
                self.assertIn(target, criteria["replayPins"])
        for path, expected in criteria["replayPins"].items():
            with self.subTest(path=path):
                self.assertEqual(hashlib.sha256((prep.ROOT / path).read_bytes()).hexdigest(), expected)
        surface = json.loads((prep.INTEGRATION / "SURFACE-FIT.json").read_bytes())
        self.assertEqual(surface["fullUpperMillimetres"], {"detailed": 5000, "native": 4997})
        self.assertEqual(surface["fullUpperMillimetres"]["native"], surface["pieces"]["ring-wing"]["native"])
        difference = json.loads((prep.INTEGRATION / "ORIGINAL-DIFFERENCE.json").read_bytes())
        self.assertEqual(difference["native"]["strictLowerMillimetres"], 1957)

    def test_studio_uses_exact_original_engineering_and_meshes(self):
        _, inputs, _, detail, native = prep.load_source()
        finite = json.loads(inputs["finite-dry-source.json"])["emptyDryReference"]
        package = json.loads(prep.prepare()[prep.STUDIO_NAME])
        for key, model in (("detail", detail), ("native", native)):
            self.assertEqual(package[key]["vertices"], model["vertices"])
            self.assertEqual(package[key]["triangles"], model["faces"])
        self.assertEqual(set(package["detail"]["groups"] + package["native"]["groups"]), set(range(24)))
        self.assertEqual(len(package["clearances"]), 5)
        self.assertEqual(package["engineering"]["massKg"], 50196)
        self.assertEqual(package["engineering"]["centerOfMass"], finite["centreMetres"])
        self.assertEqual(sum(package["engineering"]["inertia"], []), finite["inertiaAboutCentreOfMassKgM2"])
        self.assertEqual(package["provenance"]["engineeringReference"], finite["id"])

    def test_check_replays_all_generated_streams_without_writing(self):
        with patch.object(prep, "replace_output", side_effect=AssertionError("check rewrote stream")):
            with patch("sys.argv", ["prepare_oryx_voyager", "--check"]):
                prep.main()


if __name__ == "__main__":
    unittest.main()
