"""Check Griffon Mk II's original visible streams and its distinct generated inspection candidate."""

from hashlib import sha1, sha256
import json
import struct
import tomllib
import unittest
from unittest.mock import patch

import prepare_griffon_mk_ii_geometry as prep
import verify_griffon_mk_ii_geometry as proof


class GriffonMkIISources(unittest.TestCase):
    def test_frozen_git_source_records(self):
        originals = prep.load_frozen()
        report = json.loads((prep.INTEGRATION / "FROZEN-SOURCES.json").read_bytes())
        self.assertEqual(report["revision"], "a11fe16924d87d06021816f9037952afcbac08db")
        generated = originals.pop("GENERATED-COLLISION.json")
        self.assertEqual(sha256(generated).hexdigest(), "a162f31e9aa3d9de48018b235589f411dc50570c59bddb861e53f346c6ec96b1")
        for name, data in originals.items():
            with self.subTest(path=name):
                self.assertEqual(report["paths"][name]["bytes"], len(data))
                self.assertEqual(report["paths"][name]["sha256"], sha256(data).hexdigest())
                self.assertEqual(report["paths"][name]["blob"], sha1(f"blob {len(data)}\0".encode() + data).hexdigest())

    def test_original_visible_meshes_and_distinct_studio_collision_stream(self):
        prepared = prep.build()
        originals = prep.load_frozen()
        resource = tomllib.loads(originals["resource.toml"].decode())
        native = json.loads(originals["native-export.json"])["nativeDesign"]
        collision = json.loads(originals["GENERATED-COLLISION.json"])
        self.assertEqual(resource["nativeDesign"], native)
        self.assertEqual([len(resource["design"]["faces"]), len(native["faces"]), len(collision["nativeCandidate"]["faces"])],
                         [1644, 596, 380])
        self.assertFalse(resource["catalogue"]["playerFlyable"])
        self.assertEqual(str(prep.SCALE), "30000000/3215237")
        self.assertEqual(struct.unpack_from("<8sII", prepared["visual.bin"]), (b"GRI2VIZ1", 1644 * 3, 56))
        self.assertEqual(struct.unpack_from("<8sII", prepared["native-visual.bin"]), (b"GRI2VIZ1", 596 * 3, 56))
        studio = json.loads(prepared[prep.STUDIO_NAME])
        self.assertEqual(studio["detail"]["triangles"], resource["design"]["faces"])
        self.assertEqual(studio["native"]["triangles"], collision["nativeCandidate"]["faces"])
        self.assertEqual(studio["native"]["vertices"], prep.metre_space(collision["nativeCandidate"])["vertices"])
        self.assertEqual(len(studio["clearances"]), 7)
        self.assertEqual(studio["engineering"]["massKg"], 30031)

    def test_both_route_sets_and_all_original_overlap_points(self):
        result = proof.verify()
        self.assertEqual(prep.encode(result), (prep.INTEGRATION / "CONTACT-RESULT.json").read_bytes())
        guards = result["exportedCollisionRoutesMillimetres"]
        self.assertEqual(guards, result["archivedVisibleNativeRoutesMillimetres"])
        self.assertEqual(guards["port-inner-gate"], 1089)
        self.assertEqual(guards["starboard-inner-gate"], 1089)
        self.assertEqual(sorted(guards.values()), [1002, 1089, 1089, 1216, 1359, 1422, 1422])
        self.assertEqual(len(result["sourceJoints"]), 7)

    def test_fit_and_fidelity_record_comparison(self):
        fidelity = json.loads((prep.INTEGRATION / "candidate-fidelity.json").read_bytes())
        self.assertEqual((fidelity["exactParts"], fidelity["allParts"]), (13, 19))
        self.assertEqual(fidelity["fullUpperMillimetres"],
                         dict(candidateToArchive=983, archiveToCandidate=862,
                              candidateToDetailed=999, detailedToCandidate=1739))
        measurement = json.loads((prep.INTEGRATION / "candidate-fit.json").read_bytes())
        self.assertEqual(measurement["fullUpperMillimetres"], dict(detailed=1480, archiveNative=1597, candidateNative=1597))

    def test_replay_pins_and_check_without_rewriting(self):
        criteria = json.loads((prep.INTEGRATION / "CRITERIA.json").read_bytes())
        for path, expected in criteria["replayPins"].items():
            with self.subTest(path=path):
                self.assertEqual(sha256((prep.ROOT / path).read_bytes()).hexdigest(), expected)
        with patch.object(prep, "replace_output", side_effect=AssertionError("check rewrote stream")):
            with patch("sys.argv", ["prepare_griffon_mk_ii_geometry", "--check"]):
                prep.main()


if __name__ == "__main__":
    unittest.main()
