"""Verify the independent archived Seriema Courier collision and visible sources."""

import hashlib
import json
import struct
import tomllib
import unittest
from unittest.mock import patch

import prepare_seriema_courier_geometry as prep
import verify_seriema_courier_geometry as proof


class SeriemaCourierSources(unittest.TestCase):
    def test_exact_archives_and_collision_lineage(self):
        originals = prep.load_frozen()
        stamps = [
            json.loads((prep.INTEGRATION / name).read_bytes())
            for name in ("FROZEN-SOURCES.json", "CREATOR-PINS.json")
        ]
        self.assertTrue(all(record["revision"] == "a11fe16924d87d06021816f9037952afcbac08db" for record in stamps))
        self.assertEqual(stamps[0]["tree"], "68fec574df12ddf74acb14747e0dd6839f478642")
        inventory = stamps[0]["paths"] | stamps[1]["paths"]
        for name, data in originals.items():
            with self.subTest(name=name):
                record = inventory[name]
                self.assertEqual(prep.pin(data), {key: record[key] for key in ("bytes", "sha256")})
                self.assertEqual(hashlib.sha1(f"blob {len(data)}\0".encode() + data).hexdigest(), record["blob"])
        source = tomllib.loads(originals["resource.toml"].decode())
        visible_native = json.loads(originals["native-export.json"])
        collision = json.loads(originals["CREATOR-CANDIDATE.json"])
        self.assertEqual(visible_native["nativeDesign"], source["nativeDesign"])
        self.assertEqual(collision["originalSourceSHA256"], visible_native["sourceSHA256"])
        self.assertEqual(collision["calibration"], "75/11")
        self.assertEqual(collision["coordinateUnits"], "metres")
        self.assertEqual(collision["status"], "SOURCE_GEOMETRY_ONLY")
        self.assertEqual([len(mesh["faces"]) for mesh in (source["design"], source["nativeDesign"], collision["nativeDesign"])],
                         [1656, 608, 512])
        self.assertEqual(len(collision["parts"]), 20)
        self.assertFalse(source["catalogue"]["playerFlyable"])
        self.assertFalse(source["catalogue"]["normalPurchaseEligible"])

    def test_recipe_mismatch_and_separate_collision_source(self):
        receipt = json.loads((prep.INTEGRATION / "CREATOR-REPLAY.json").read_bytes())
        self.assertEqual(receipt["matchStatus"], "MISMATCH")
        self.assertEqual([(receipt[key]["vertices"], receipt[key]["faces"], receipt[key]["parts"])
                          for key in ("pinnedRecipeOutput", "frozenCollisionArtifact")],
                         [(272, 464, 20), (296, 512, 20)])
        self.assertEqual(receipt["legacyRecipeTests"]["passed"], 2)
        self.assertFalse(receipt["countMatchedVariant"]["streamMatch"])
        comparison = (prep.INTEGRATION / "CREATOR-ARCHIVE-COMPARISON.json").read_bytes()
        self.assertEqual(prep.pin(comparison)["sha256"], receipt["countMatchedVariant"]["sha256"])
        self.assertTrue(any(not part["facesExact"] or not part["uvExact"] or part["maxVertexDelta"] != 0
                            for part in json.loads(comparison)))

    def test_visible_rendering_and_inspection_keep_separate_meshes(self):
        originals = prep.load_frozen()
        source = tomllib.loads(originals["resource.toml"].decode())
        outputs = prep.build()
        studio = json.loads(outputs[prep.STUDIO_NAME])
        collision = json.loads(originals["CREATOR-CANDIDATE.json"])["nativeDesign"]
        detailed = prep.metre_space(source["design"])
        self.assertEqual(studio["detail"]["vertices"], detailed["vertices"])
        self.assertEqual(studio["detail"]["triangles"], detailed["faces"])
        self.assertEqual(studio["native"]["vertices"], collision["vertices"])
        self.assertEqual(studio["native"]["triangles"], collision["faces"])
        self.assertEqual(studio["faceMaterials"], detailed["faceMaterials"])
        self.assertEqual([mapping["uv"] for mapping in studio["faceTextures"]],
                         [mapping["uv"] for mapping in detailed["faceTextures"]])
        self.assertEqual(studio["provenance"]["originalScaleRatio"], [75, 11])
        self.assertEqual(studio["metresPerUnit"], 1)
        self.assertEqual(studio["engineering"]["massKg"], 14539)
        self.assertEqual(struct.unpack_from("<8sII", outputs["native-visual.bin"]), (b"SERIVIZ1", 608 * 3, 56))

    def test_bundled_collision_keeps_seven_routes_and_four_joints(self):
        result = proof.verify()
        self.assertEqual(prep.encode(result), (prep.INTEGRATION / "CONTACT-RESULT.json").read_bytes())
        self.assertEqual(result["exportedFacesCovered"], {"detailed": 1656, "collision": 512})
        self.assertEqual(result["archivedVisibleNativeRoutesMillimetres"], result["exportedCollisionRoutesMillimetres"])
        self.assertEqual(sorted(result["exportedCollisionRoutesMillimetres"].values()), [448, 448, 542, 542, 618, 618, 629])
        self.assertEqual(len(result["sourceJoints"]), 4)
        self.assertIn("blade-to-main-body", result["sourceJoints"])
        for value in result["sourceJoints"].values():
            self.assertEqual(set(value["hitCountsAboveBelow"]), {"detailed", "archivedVisibleNative", "archivedCollision"})

    def test_fidelity_report_and_scope(self):
        fidelity = json.loads((prep.INTEGRATION / "candidate-fidelity.json").read_bytes())
        self.assertEqual((fidelity["exactParts"], fidelity["allParts"]), (16, 20))
        self.assertEqual(fidelity["fullUpperMillimetres"],
                         {"candidateToArchive": 934, "archiveToCandidate": 900,
                          "candidateToDetailed": 984, "detailedToCandidate": 1235})
        self.assertEqual(fidelity["silhouette"]["againstArchivedNative"]["axisExtremaAbsoluteDeltaMillimetres"],
                         [[0, 0]] * 3)
        self.assertEqual(fidelity["silhouette"]["againstArchivedNative"]["maxSampledSilhouetteOffsetMillimetres"],
                         {"xy": 0, "xz": 0, "yz": 0})
        criteria = json.loads((prep.INTEGRATION / "CRITERIA.json").read_bytes())
        for command in criteria["byteReplay"]["commands"]:
            _, separator, artifact = command.partition(" | cmp - ")
            if separator:
                self.assertIn(artifact, criteria["replayPins"])
        for path, expected in criteria["replayPins"].items():
            self.assertEqual(prep.pin((prep.ROOT / path).read_bytes())["sha256"], expected)

    def test_fit_rejects_an_unverified_triangulation_before_measurement(self):
        import measure_seriema_courier_geometry as measure

        source = measure.load()
        for candidate in (False, True):
            with self.subTest(candidate=candidate), patch.object(measure, "REFERENCE_FANS_SHA", "0" * 64):
                with patch.object(measure, "adaptive_upper") as upper:
                    with self.assertRaisesRegex(ValueError, "Changed Seriema Courier reference triangulation"):
                        measure.support_fit(source, candidate)
                    upper.assert_not_called()

    def test_check_replays_outputs_without_mutation(self):
        with patch.object(prep, "replace_output", side_effect=AssertionError("check rewrote output")):
            with patch("sys.argv", ["prepare_seriema_courier_geometry", "--check"]):
                prep.main()


if __name__ == "__main__":
    unittest.main()
