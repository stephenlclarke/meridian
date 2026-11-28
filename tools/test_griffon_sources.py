"""Check Griffon's original visible streams and its distinct collision archive."""

from hashlib import sha1, sha256
import json
import struct
import tomllib
import unittest
from unittest.mock import patch

import prepare_griffon_geometry as prep
import verify_griffon_geometry as proof


class GriffonSources(unittest.TestCase):
    def test_frozen_git_source_records(self):
        originals = prep.load_frozen()
        report = json.loads((prep.INTEGRATION / "FROZEN-SOURCES.json").read_bytes())
        self.assertEqual(report["revision"], "a11fe16924d87d06021816f9037952afcbac08db")
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
        collision = json.loads(originals["ARCHIVED-COLLISION.json"])
        self.assertEqual(resource["nativeDesign"], native)
        self.assertEqual([len(resource["design"]["faces"]), len(native["faces"]), len(collision["nativeDesign"]["faces"])],
                         [1644, 596, 512])
        self.assertFalse(resource["catalogue"]["playerFlyable"])
        self.assertEqual(str(prep.SCALE), "60000/6221")
        self.assertEqual(struct.unpack_from("<8sII", prepared["visual.bin"]), (b"GRIFVIZ1", 1644 * 3, 56))
        self.assertEqual(struct.unpack_from("<8sII", prepared["native-visual.bin"]), (b"GRIFVIZ1", 596 * 3, 56))
        studio = json.loads(prepared[prep.STUDIO_NAME])
        self.assertEqual(studio["detail"]["triangles"], resource["design"]["faces"])
        self.assertEqual(studio["native"]["triangles"], collision["nativeDesign"]["faces"])
        self.assertEqual(studio["native"]["vertices"], collision["nativeDesign"]["vertices"])
        self.assertEqual(len(studio["clearances"]), 7)
        self.assertEqual(studio["engineering"]["massKg"], 30998)

    def test_both_route_sets_and_all_original_overlap_points(self):
        result = proof.verify()
        self.assertEqual(prep.encode(result), (prep.INTEGRATION / "CONTACT-RESULT.json").read_bytes())
        guards = result["exportedCollisionRoutesMillimetres"]
        self.assertEqual(guards, result["archivedVisibleNativeRoutesMillimetres"])
        self.assertEqual(guards["port-inner-gate"], 1042)
        self.assertEqual(guards["starboard-inner-gate"], 1042)
        self.assertEqual(sorted(guards.values()), [1009, 1042, 1042, 1154, 1278, 1278, 1288])
        self.assertEqual(len(result["sourceJoints"]), 7)

    def test_fit_and_fidelity_record_comparison(self):
        fidelity = json.loads((prep.INTEGRATION / "candidate-fidelity.json").read_bytes())
        self.assertEqual((fidelity["exactParts"], fidelity["allParts"]), (17, 19))
        self.assertEqual(fidelity["fullUpperMillimetres"],
                         dict(candidateToArchive=884, archiveToCandidate=986,
                              candidateToDetailed=975, detailedToCandidate=1990))
        measurement = json.loads((prep.INTEGRATION / "candidate-fit.json").read_bytes())
        self.assertEqual(measurement["fullUpperMillimetres"], dict(detailed=1542, archiveNative=1636, candidateNative=1946))

    def test_replay_pins_and_check_without_rewriting(self):
        criteria = json.loads((prep.INTEGRATION / "CRITERIA.json").read_bytes())
        for path, expected in criteria["replayPins"].items():
            with self.subTest(path=path):
                self.assertEqual(sha256((prep.ROOT / path).read_bytes()).hexdigest(), expected)
        with patch.object(prep, "replace_output", side_effect=AssertionError("check rewrote stream")):
            with patch("sys.argv", ["prepare_griffon_geometry", "--check"]):
                prep.main()


if __name__ == "__main__":
    unittest.main()
