"""Verify the two original Wombat meshes, separate candidate and grouped artwork."""

from fractions import Fraction
import hashlib
import json
import struct
import tomllib
import unittest
from unittest.mock import patch

import prepare_wombat_candidate as recipe
import prepare_wombat_geometry as prep
import verify_wombat_geometry as proof


class WombatSources(unittest.TestCase):
    def test_exact_original_archive_and_distinct_candidate(self):
        pins = json.loads((prep.INTEGRATION / "FROZEN-SOURCES.json").read_bytes())
        self.assertEqual(pins["revision"], "a11fe16924d87d06021816f9037952afcbac08db")
        self.assertEqual(pins["tree"], "68fec574df12ddf74acb14747e0dd6839f478642")
        inputs = prep.load_frozen()
        for name, raw in inputs.items():
            with self.subTest(path=name):
                expected = pins["paths"][name]
                self.assertEqual(prep.pin(raw), {key: expected[key] for key in ("bytes", "sha256")})
                self.assertEqual(hashlib.sha1(f"blob {len(raw)}\0".encode() + raw).hexdigest(), expected["blob"])
        resource = tomllib.loads(inputs["resource.toml"].decode())
        contract = tomllib.loads(inputs["contract.toml"].decode())
        archive = json.loads(inputs["native-export.json"])
        candidate_bytes = (prep.ORIGINAL / "SH060-project-candidate.json").read_bytes()
        candidate = json.loads(candidate_bytes)
        self.assertEqual(prep.pin(candidate_bytes)["sha256"], prep.CANDIDATE_SHA)
        self.assertEqual(resource["nativeDesign"], archive["nativeDesign"])
        self.assertEqual((len(archive["nativeDesign"]["faces"]), len(candidate["nativeCandidate"]["faces"])), (1024, 496))
        self.assertEqual(contract["geometry"]["nativeFaceLimit"], 512)
        self.assertEqual(candidate["status"], "PROJECT_AUTHORED_ARTWORK_CANDIDATE")
        self.assertFalse(resource["catalogue"]["playerFlyable"])
        self.assertFalse(resource["catalogue"]["normalPurchaseEligible"])
        self.assertTrue(contract["access"]["playerFlyable"])
        self.assertEqual(contract["access"]["purchasePolicy"], "blueprintFabrication")

    def test_grouping_keeps_every_triangle_and_uv(self):
        inputs = prep.load_frozen()
        resource = tomllib.loads(inputs["resource.toml"].decode())
        candidate = json.loads((prep.ORIGINAL / "SH060-project-candidate.json").read_bytes())["nativeCandidate"]
        outputs = prep.build()
        studio = json.loads(outputs[prep.STUDIO_NAME])
        for key, source in (("detail", resource["design"]), ("native", candidate)):
            output = studio[key]
            self.assertEqual(len(output["triangles"]), len(source["faces"]))
            for before, after in zip(source["faces"], output["triangles"], strict=True):
                self.assertEqual([source["vertices"][i] for i in before], [output["vertices"][i] for i in after])
                self.assertEqual(len({output["groups"][i] for i in after}), 1)
        self.assertEqual(studio["faceMaterials"], resource["design"]["faceMaterials"])
        self.assertEqual([m["uv"] for m in studio["faceTextures"]], [m["uv"] for m in resource["design"]["faceTextures"]])
        points = list(struct.iter_unpack("<3d", outputs["source.vertices"][16:]))
        expected = [
            tuple(value * float(prep.SCALE) for value in vertex)
            for key in ("detail", "native")
            for vertex in studio[key]["vertices"]
        ]
        self.assertEqual(points, expected)
        for name, faces in (("visual.bin", 3544), ("archive-native-visual.bin", 1024), ("candidate-native-visual.bin", 496)):
            self.assertEqual(struct.unpack_from("<8sII", outputs[name]), (b"WOMBVIZ1", faces * 3, 56))

    def test_candidate_fidelity_and_both_contact_replays(self):
        observed = proof.verify()
        self.assertEqual(prep.encode(observed), (prep.INTEGRATION / "CONTACT-RESULT.json").read_bytes())
        self.assertEqual(observed["detailedWithArchivedNative"], observed["detailedWithCandidateNative"])
        self.assertEqual(sorted(observed["detailedWithCandidateNative"].values()), [459, 459, 513, 858, 858, 2188, 2188])
        self.assertEqual(observed["bundledFaceCoverage"], {"detailed": 3544, "candidate": 496})
        joints = observed["sourceJoints"]
        self.assertEqual(sum(name.endswith("-frame-corner") for name in joints), 4)
        self.assertIn("cockpit-crossmember-to-glazing", joints)
        for joint in joints.values():
            self.assertEqual(set(joint["hitCountsAboveBelow"]), {"detailed", "archivedNative", "candidateNative"})
        fidelity = json.loads((prep.INTEGRATION / "candidate-fidelity.json").read_bytes())
        self.assertEqual(fidelity["fullUpperMillimetres"], {
            "candidateToArchive": 2483, "archiveToCandidate": 2564,
            "candidateToDetailed": 2746, "detailedToCandidate": 3957,
        })
        self.assertEqual(fidelity["archivedOriginalComparisonUpperMillimetres"], {
            "archivedNativeToDetailed": 1475, "detailedToArchivedNative": 3157,
        })
        self.assertEqual(set(fidelity["omittedWellDepths"]), {"12", "15", "18", "19"})
        for depth in fidelity["omittedWellDepths"].values():
            self.assertEqual(depth["mouthToFloorUpperMillimetres"], [988] * 4)
        silhouette = fidelity["silhouette"]["againstArchivedNative"]
        self.assertEqual(silhouette["axisExtremaAbsoluteDeltaMillimetres"], [[0, 0]] * 3)
        self.assertEqual(silhouette["maxSampledSilhouetteOffsetMillimetres"], {"xy": 535, "xz": 0, "yz": 0})
        before = [row["verifiedUpperMillimetres"]["archiveToCandidate"] for row in fidelity["parts"]]
        after = [row["verifiedUpperMillimetres"].get("candidateToArchive", 0) for row in fidelity["parts"]]
        self.assertEqual((max(before), max(after)), (2564, 2483))
        self.assertEqual((fidelity["fixedParts"], fidelity["resampledParts"], fidelity["omittedBlindWellParts"]),
                         (20, 8, 4))
        fit = json.loads((prep.INTEGRATION / "candidate-fit.json").read_bytes())
        criteria = json.loads((prep.INTEGRATION / "CRITERIA.json").read_bytes())
        self.assertEqual(fit["fullUpperMillimetres"], criteria["fitUpperMillimetres"])
        for command in criteria["byteReplay"]["commands"]:
            _, separator, artifact = command.partition(" | cmp - ")
            if separator:
                self.assertIn(artifact, criteria["replayPins"])
        for path, expected in criteria["replayPins"].items():
            with self.subTest(path=path):
                self.assertEqual(prep.pin((prep.ROOT / path).read_bytes())["sha256"], expected)

    def test_studio_exact_scale_and_frozen_dry_record(self):
        outputs = prep.build()
        definition = json.loads(outputs[prep.STUDIO_NAME])
        finite = json.loads((prep.ORIGINAL / "finite-dry-source.json").read_bytes())["emptyDryReference"]
        self.assertEqual(definition["metresPerUnit"], float(Fraction(23000, 1211)))
        self.assertEqual(definition["provenance"]["frozenNativeArchive"]["faces"], 1024)
        self.assertEqual(definition["provenance"]["separateNativeCandidate"]["faces"], 496)
        self.assertEqual(set(definition["detail"]["groups"] + definition["native"]["groups"]), set(range(62)))
        self.assertEqual(len(definition["clearances"]), 7)
        self.assertEqual(definition["engineering"]["massKg"], 167216)
        self.assertEqual(definition["engineering"]["centerOfMass"], finite["centreMetres"])
        self.assertEqual(sum(definition["engineering"]["inertia"], []), finite["inertiaAboutCentreOfMassKgM2"])

    def test_project_recipe_replays_without_changing_archived_parts(self):
        native, ownership, _ = recipe.source()
        candidate = recipe.build()
        self.assertEqual(recipe.encode_candidate(), recipe.PATH.read_bytes())
        self.assertEqual((len(candidate["parts"]), len(candidate["nativeCandidate"]["faces"])), (28, 496))
        omitted = set(candidate["omittedBlindWellParts"])
        self.assertEqual(omitted, {12, 15, 18, 19})
        self.assertTrue(all(ownership[index]["creatorMethod"] == "Mesh.well" for index in omitted))
        self.assertEqual(len(native["faces"]), 1024)
        creator = json.loads((prep.INTEGRATION / "CREATOR-REPRODUCTION.json").read_bytes())
        self.assertTrue(creator["design"]["exactArchivedDesignAndPackagedResource"])
        self.assertTrue(creator["nativeDesign"]["exactArchivedDesignAndPackagedResource"])
        self.assertEqual(creator["angleOnlyLowerBound"]["minimumFaces"], 520)
        self.assertEqual({part["originalPartIndex"] for part in candidate["parts"]} | omitted, set(range(32)))

    def test_check_replays_outputs_without_writing(self):
        with patch.object(prep, "replace_output", side_effect=AssertionError("check rewrote stream")):
            with patch("sys.argv", ["prepare_wombat_geometry", "--check"]):
                prep.main()


if __name__ == "__main__":
    unittest.main()
