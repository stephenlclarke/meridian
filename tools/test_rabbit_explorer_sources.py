"""Check the original Rabbit meshes and the isolated inspection candidate."""

import hashlib
import json
import struct
import unittest
from unittest.mock import patch

import prepare_rabbit_explorer as prep
import rabbit_explorer_art_mesh as binding
import verify_rabbit_explorer as proof


class RabbitExplorerSources(unittest.TestCase):
    def test_frozen_original_catalogue_and_calibration(self):
        inputs, resource, finite, source, _ = binding.load()
        self.assertEqual(str(binding.SCALE), "30000000/2547773")
        self.assertEqual((finite["id"], finite["massKg"]), ("finite-intrinsic-empty-rabbit-explorer", 33268))
        self.assertFalse(resource["catalogue"]["playerFlyable"])
        self.assertFalse(resource["catalogue"]["normalPurchaseEligible"])
        self.assertEqual([len(mesh["faces"]) for mesh, _ in source.values()], [2648, 888, 480])
        pins = json.loads((prep.INTEGRATION / "FROZEN-SOURCES.json").read_bytes())
        for name, data in inputs.items():
            with self.subTest(path=name):
                self.assertEqual(pins["paths"][name]["blob"], hashlib.sha1(f"blob {len(data)}\0".encode() + data).hexdigest())

    def test_original_visuals_and_separate_candidate_stream(self):
        source = binding.load()[3]
        for label, filename in (("detailed", "visual.bin"), ("archiveNative", "archive-native-visual.bin"),
                                ("candidateNative", "candidate-native-visual.bin")):
            with self.subTest(stream=filename):
                mesh = source[label][0]
                encoded = (prep.ORIGINAL / filename).read_bytes()
                self.assertEqual(encoded, prep.visual(mesh))
                self.assertEqual(struct.unpack_from("<8sII", encoded), (b"RABBVIZ1", len(mesh["faces"]) * 3, 56))
                self.assertEqual(len(encoded), 16 + 56 * 3 * len(mesh["faces"]))

    def test_sweeps_separate_the_real_bow_and_inner_arc(self):
        result = proof.verify()
        self.assertEqual(prep.encode(result), (prep.INTEGRATION / "CONTACT-RESULT.json").read_bytes())
        candidate = result["candidate"]
        self.assertEqual(candidate["bow-arc-opening"], 1543)
        self.assertEqual(sorted(candidate.values()), [921, 1096, 1096, 1331, 1543, 1545, 1545])
        self.assertEqual(result["archive"]["port-inner-arc"], 816)
        self.assertEqual(result["archive"]["starboard-inner-arc"], 816)
        assignment = (prep.ORIGINAL / "collision-piece-membership.bin").read_bytes()
        self.assertEqual(sum(slot != 255 for slot in assignment[1::2]), 272)

    def test_fidelity_contacts_fit_and_limits(self):
        fidelity = json.loads((prep.INTEGRATION / "CANDIDATE-FIDELITY.json").read_bytes())
        self.assertEqual(fidelity["allParts"], 14)
        self.assertEqual(fidelity["exactParts"], 7)
        self.assertEqual(fidelity["arcEndcapLateralSeparationLowerMillimetres"],
                         dict(detailed=4486, archiveNative=4486, candidateNative=4486))
        self.assertEqual(fidelity["fullUpperMillimetres"],
                         dict(candidateToArchive=1249, archiveToCandidate=1355,
                              candidateToDetailed=1249, detailedToCandidate=2500))
        fit = json.loads((prep.INTEGRATION / "CANDIDATE-FIT.json").read_bytes())
        self.assertEqual(fit["fullUpperMillimetres"], dict(detailed=1975, archiveNative=2115, candidateNative=2492))

    def test_signed_studio_definition_uses_candidate_and_exterior_artwork_probes(self):
        _, _, finite, source, _ = binding.load()
        package = json.loads(prep.build()[prep.STUDIO_NAME])
        self.assertEqual(package["metresPerUnit"], float(binding.SCALE))
        for key, label in (("detail", "detailed"), ("native", "candidateNative")):
            self.assertEqual(package[key]["vertices"], source[label][0]["vertices"])
            self.assertEqual(package[key]["triangles"], source[label][0]["faces"])
        self.assertEqual({probe["name"] for probe in package["clearances"]}, set(prep.STUDIO_PROBES))
        self.assertEqual(package["engineering"]["centerOfMass"], finite["centreMetres"])
        self.assertEqual(sum(package["engineering"]["inertia"], []), finite["inertiaAboutCentreOfMassKgM2"])

    def test_replay_pins_and_check_without_rewriting(self):
        criteria = json.loads((prep.INTEGRATION / "CRITERIA.json").read_bytes())
        for command in criteria["byteReplay"]["commands"]:
            _, separator, target = command.partition(" | cmp - ")
            if separator:
                self.assertIn(target, criteria["replayPins"])
        for path, expected in criteria["replayPins"].items():
            with self.subTest(path=path):
                self.assertEqual(hashlib.sha256((binding.ROOT / path).read_bytes()).hexdigest(), expected)
        with patch.object(prep, "replace_output", side_effect=AssertionError("check rewrote stream")):
            with patch("sys.argv", ["prepare_rabbit_explorer", "--check"]):
                prep.main()


if __name__ == "__main__":
    unittest.main()
