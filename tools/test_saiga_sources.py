"""Check the exact Saiga archives, authored appearance and bundled collision geometry."""

import hashlib
import json
import struct
import tomllib
import unittest
from unittest.mock import patch

import prepare_saiga_candidate as recipe
import prepare_saiga_geometry as prep
import verify_saiga_geometry as proof


class SaigaSources(unittest.TestCase):
    def test_original_archive_and_calibration(self):
        frozen = json.loads((prep.INTEGRATION / "FROZEN-SOURCES.json").read_bytes())
        self.assertEqual(frozen["revision"], "a11fe16924d87d06021816f9037952afcbac08db")
        self.assertEqual(frozen["tree"], "68fec574df12ddf74acb14747e0dd6839f478642")
        originals = prep.load_frozen()
        for name, data in originals.items():
            with self.subTest(path=name):
                record = frozen["paths"][name]
                self.assertEqual(prep.pin(data), {key: record[key] for key in ("bytes", "sha256")})
                self.assertEqual(hashlib.sha1(f"blob {len(data)}\0".encode() + data).hexdigest(), record["blob"])
        resource = tomllib.loads(originals["resource.toml"].decode())
        original_native = json.loads(originals["native-export.json"])["nativeDesign"]
        contract = tomllib.loads(originals["contract.toml"].decode())
        archived = json.loads(originals["engineering.json"])
        self.assertEqual(original_native, resource["nativeDesign"])
        self.assertEqual((len(resource["design"]["faces"]), len(original_native["faces"])), (5776, 1720))
        self.assertEqual(archived["metresPerSourceUnitExact"], "20")
        self.assertEqual(archived["dryMassKilograms"], 284291)
        self.assertTrue(archived["nativeQualification"].startswith("UNRUN"))
        self.assertEqual(originals["engineering.json"], originals["decimal80-record.json"])
        self.assertEqual(contract["geometry"]["nativeFaceLimit"], 512)
        self.assertTrue(resource["catalogue"]["playerFlyable"])
        self.assertTrue(resource["catalogue"]["normalPurchaseEligible"])
        self.assertTrue(contract["access"]["playerFlyable"])
        self.assertEqual(contract["access"]["purchasePolicy"], "blueprintFabrication")

    def test_recipe_preserves_original_collar_extrema_and_through_holes(self):
        original, ownership, _ = recipe.source()
        candidate = recipe.build()
        self.assertEqual(recipe.encode_candidate(), recipe.PATH.read_bytes())
        self.assertEqual(set(candidate["omittedBlindWellParts"]), {3, 4, 7, 10, 14, 15})
        self.assertEqual((len(candidate["parts"]), len(candidate["nativeCandidate"]["faces"])), (22, 512))
        surfaces = candidate["nativeCandidate"]
        collars = [part for part in candidate["parts"] if part["mode"] == "resampled original toroidal rings"]
        self.assertEqual(len(collars), 2)
        for part in collars:
            index = part["originalPartIndex"]
            source = ownership[index]
            first, count = part["vertexStart"], part["vertexCount"]
            point_set = surfaces["vertices"][first:first + count]
            old = original["vertices"][source["vertexStart"]:source["vertexStart"] + source["vertexCount"]]
            self.assertEqual(point_set, [old[m * 8 + s] for m in range(0, 24, 3) for s in range(0, 8, 2)])
            self.assertEqual([(min(row), max(row)) for row in zip(*point_set)],
                             [(min(row), max(row)) for row in zip(*old)])
            faces = surfaces["faces"][part["faceStart"]:part["faceStart"] + part["faceCount"]]
            edges = {tuple(sorted(edge)) for a, b, c in faces for edge in ((a, b), (b, c), (c, a))}
            self.assertEqual(len(point_set) - len(edges) + len(faces), 0)

    def test_artwork_has_exact_original_detail_and_distinct_probe_scope(self):
        originals = prep.load_frozen()
        resource = tomllib.loads(originals["resource.toml"].decode())
        original_detail = resource["design"]
        generated = prep.build()
        studio = json.loads(generated[prep.STUDIO_NAME])
        self.assertEqual(studio["detail"]["vertices"], original_detail["vertices"])
        self.assertEqual(studio["detail"]["triangles"], original_detail["faces"])
        self.assertEqual(studio["faceMaterials"], original_detail["faceMaterials"])
        self.assertEqual([item["uv"] for item in studio["faceTextures"]],
                         [item["uv"] for item in original_detail["faceTextures"]])
        self.assertEqual({item["name"] for item in studio["clearances"]},
                         {"port-outboard-upper", "starboard-outboard-upper", "ventral-outside-collar"})
        self.assertEqual(len(set(studio["detail"]["groups"] + studio["native"]["groups"])), 58)
        self.assertEqual(studio["metresPerUnit"], 20)
        self.assertEqual(studio["engineering"]["massKg"], 284291)
        self.assertEqual(struct.unpack_from("<8sII", generated["archive-native-visual.bin"]),
                         (b"SAIGVIZ1", 1720 * 3, 56))
        self.assertEqual(struct.unpack_from("<8sII", generated["candidate-native-visual.bin"]),
                         (b"SAIGVIZ1", 512 * 3, 56))

    def test_exported_collision_keeps_faces_and_all_seven_routes(self):
        result = proof.verify()
        recorded = prep.INTEGRATION / "CONTACT-RESULT.json"
        self.assertEqual(prep.encode(result), recorded.read_bytes())
        self.assertEqual(result["collisionSupportGroups"], 72)
        self.assertEqual(result["overlappingBoundaryVertices"], 256)
        self.assertEqual(result["exportedFacesCovered"], {"detailed": 5776, "candidateNative": 512})
        self.assertEqual(result["exportedCollarFacesCovered"], {"detailed": 2304, "candidateNative": 128})
        actual = result["exportedCandidateRoutesMillimetres"]
        self.assertEqual(actual, result["archivedOriginalRoutesMillimetres"])
        self.assertEqual(sorted(actual.values()), [511, 515, 1499, 2245, 2245, 2256, 3129])
        self.assertEqual(result["centralWebBlockedControl"]["hitCountsAboveBelow"],
                         {"detailed": (1, 1), "archivedNative": (1, 1), "candidateNative": (1, 1)})

    def test_fidelity_evidence_covers_candidate_and_both_originals(self):
        fit = json.loads((prep.INTEGRATION / "candidate-fidelity.json").read_bytes())
        self.assertEqual(fit["fullUpperMillimetres"], {
            "candidateToArchive": 2750, "archiveToCandidate": 4259,
            "candidateToDetailed": 2844, "detailedToCandidate": 4748,
        })
        self.assertEqual(fit["archivedOriginalComparisonUpperMillimetres"],
                         {"archivedNativeToDetailed": 2222, "detailedToArchivedNative": 2598})
        self.assertEqual((fit["fixedParts"], fit["resampledParts"], fit["resampledCollars"]), (12, 8, 2))
        self.assertEqual(fit["silhouette"]["againstArchivedNative"]["axisExtremaAbsoluteDeltaMillimetres"],
                         [[0, 0]] * 3)
        self.assertEqual(max(value for row in fit["omittedWellDepths"].values()
                             for value in row["mouthToFloorUpperMillimetres"]), 1041)
        criteria = json.loads((prep.INTEGRATION / "CRITERIA.json").read_bytes())
        for command in criteria["byteReplay"]["commands"]:
            _, separator, target = command.partition(" | cmp - ")
            if separator:
                self.assertIn(target, criteria["replayPins"])
        for path, expected in criteria["replayPins"].items():
            self.assertEqual(prep.pin((prep.ROOT / path).read_bytes())["sha256"], expected)

    def test_fit_rejects_an_unverified_triangulation_before_measurement(self):
        import measure_saiga_geometry as measure

        for candidate in (False, True):
            with self.subTest(candidate=candidate), patch.object(measure, "REFERENCE_FANS_SHA", "0" * 64):
                with patch.object(measure, "adaptive_upper") as upper:
                    with self.assertRaisesRegex(ValueError, "Changed Saiga reference triangulation"):
                        measure.measure_fit(candidate)
                    upper.assert_not_called()

    def test_generated_outputs_replay_without_writing(self):
        with patch.object(prep, "replace_output", side_effect=AssertionError("check rewrote a Saiga stream")):
            with patch("sys.argv", ["prepare_saiga_geometry", "--check"]):
                prep.main()


if __name__ == "__main__":
    unittest.main()
