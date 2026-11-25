"""Verify the two original Hyena meshes, separate candidate and grouped artwork."""

from fractions import Fraction
import hashlib
import json
import struct
import tomllib
import unittest
from unittest.mock import patch

import prepare_hyena_geometry as prep
import verify_hyena_geometry as proof


class HyenaSources(unittest.TestCase):
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
        candidate_bytes = (prep.ORIGINAL / "SH049-candidate.json").read_bytes()
        candidate = json.loads(candidate_bytes)
        self.assertEqual(prep.pin(candidate_bytes)["sha256"], prep.CANDIDATE_SHA)
        self.assertEqual(resource["nativeDesign"], archive["nativeDesign"])
        self.assertEqual((len(archive["nativeDesign"]["faces"]), len(candidate["nativeCandidate"]["faces"])), (952, 504))
        self.assertEqual(contract["geometry"]["nativeFaceLimit"], 512)
        self.assertEqual(candidate["status"], "GENERATED_CANDIDATE_ONLY_NOT_INSTALLED_OR_NATIVE_QUALIFIED")
        self.assertFalse(resource["catalogue"]["playerFlyable"])
        self.assertFalse(resource["catalogue"]["normalPurchaseEligible"])
        self.assertTrue(contract["access"]["playerFlyable"])
        self.assertEqual(contract["access"]["purchasePolicy"], "blueprintFabrication")

    def test_grouping_keeps_every_triangle_and_uv(self):
        inputs = prep.load_frozen()
        resource = tomllib.loads(inputs["resource.toml"].decode())
        candidate = json.loads((prep.ORIGINAL / "SH049-candidate.json").read_bytes())["nativeCandidate"]
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
        for name, faces in (("visual.bin", 3472), ("archive-native-visual.bin", 952), ("candidate-native-visual.bin", 504)):
            self.assertEqual(struct.unpack_from("<8sII", outputs[name]), (b"HYENVIZ1", faces * 3, 56))

    def test_candidate_fidelity_and_both_contact_replays(self):
        observed = proof.verify()
        self.assertEqual(prep.encode(observed), (prep.INTEGRATION / "CONTACT-RESULT.json").read_bytes())
        self.assertEqual(observed["detailedWithArchivedNative"], observed["detailedWithCandidateNative"])
        self.assertEqual(sorted(observed["detailedWithCandidateNative"].values()), [834, 834, 1563, 1563, 1659, 1659, 2771])
        fidelity = json.loads((prep.INTEGRATION / "candidate-fidelity.json").read_bytes())
        before = [row["verifiedUpperMillimetres"]["archiveToCandidate"] for row in fidelity["parts"]]
        after = [row["verifiedUpperMillimetres"]["candidateToArchive"] for row in fidelity["parts"]]
        self.assertEqual((max(before), max(after), before.count(0), after.count(0)), (1442, 1113, 18, 18))
        fit = json.loads((prep.INTEGRATION / "candidate-fit.json").read_bytes())
        criteria = json.loads((prep.INTEGRATION / "CRITERIA.json").read_bytes())
        self.assertEqual(fit["fullUpperMillimetres"], criteria["fitUpperMillimetres"])
        for path, expected in criteria["replayPins"].items():
            with self.subTest(path=path):
                self.assertEqual(prep.pin((prep.ROOT / path).read_bytes())["sha256"], expected)

    def test_studio_exact_scale_and_frozen_dry_record(self):
        outputs = prep.build()
        definition = json.loads(outputs[prep.STUDIO_NAME])
        finite = json.loads((prep.ORIGINAL / "finite-dry-source.json").read_bytes())["emptyDryReference"]
        self.assertEqual(definition["metresPerUnit"], float(Fraction(20_000, 1_737)))
        self.assertEqual(definition["provenance"]["frozenNativeArchive"]["faces"], 952)
        self.assertEqual(definition["provenance"]["separateNativeCandidate"]["faces"], 504)
        self.assertEqual(set(definition["detail"]["groups"] + definition["native"]["groups"]), set(range(164)))
        self.assertEqual(len(definition["clearances"]), 7)
        self.assertEqual(definition["engineering"]["massKg"], 58298)
        self.assertEqual(definition["engineering"]["centerOfMass"], finite["centreMetres"])
        self.assertEqual(sum(definition["engineering"]["inertia"], []), finite["inertiaAboutCentreOfMassKgM2"])

    def test_check_replays_outputs_without_writing(self):
        with patch.object(prep, "replace_output", side_effect=AssertionError("check rewrote stream")):
            with patch("sys.argv", ["prepare_hyena_geometry", "--check"]):
                prep.main()


if __name__ == "__main__":
    unittest.main()
