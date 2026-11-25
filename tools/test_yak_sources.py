"""Verify the two original Yak meshes, separate candidate and grouped artwork."""

from copy import deepcopy
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import struct
import tempfile
import tomllib
import unittest
from unittest.mock import patch

import numpy as np

import measure_lanner_geometry as shared_fans
import measure_yak_geometry as measure
import prepare_yak_geometry as prep
import verify_yak_geometry as proof


class YakSources(unittest.TestCase):
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
        candidate_bytes = (prep.ORIGINAL / "SH050-candidate.json").read_bytes()
        candidate = json.loads(candidate_bytes)
        self.assertEqual(prep.pin(candidate_bytes)["sha256"], prep.CANDIDATE_SHA)
        self.assertEqual(resource["nativeDesign"], archive["nativeDesign"])
        self.assertEqual((len(archive["nativeDesign"]["faces"]), len(candidate["nativeCandidate"]["faces"])), (928, 480))
        self.assertEqual(contract["geometry"]["nativeFaceLimit"], 512)
        self.assertEqual(candidate["status"], "GENERATED_CANDIDATE_ONLY_NOT_INSTALLED_OR_NATIVE_QUALIFIED")
        self.assertFalse(resource["catalogue"]["playerFlyable"])
        self.assertFalse(resource["catalogue"]["normalPurchaseEligible"])
        self.assertTrue(contract["access"]["playerFlyable"])
        self.assertEqual(contract["access"]["purchasePolicy"], "blueprintFabrication")

    def test_grouping_keeps_every_triangle_and_uv(self):
        inputs = prep.load_frozen()
        resource = tomllib.loads(inputs["resource.toml"].decode())
        candidate = json.loads((prep.ORIGINAL / "SH050-candidate.json").read_bytes())["nativeCandidate"]
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
        for name, faces in (("visual.bin", 3448), ("archive-native-visual.bin", 928), ("candidate-native-visual.bin", 480)):
            self.assertEqual(struct.unpack_from("<8sII", outputs[name]), (b"YAKVIZ01", faces * 3, 56))

    def test_candidate_fidelity_and_both_contact_replays(self):
        observed = proof.verify()
        self.assertEqual(prep.encode(observed), (prep.INTEGRATION / "CONTACT-RESULT.json").read_bytes())
        self.assertEqual(observed["detailedWithArchivedNative"], observed["detailedWithCandidateNative"])
        self.assertEqual(sorted(observed["detailedWithCandidateNative"].values()), [508, 508, 508, 508, 1388, 2007, 2008])
        fidelity = json.loads((prep.INTEGRATION / "candidate-fidelity.json").read_bytes())
        before = [row["verifiedUpperMillimetres"]["archiveToCandidate"] for row in fidelity["parts"]]
        after = [row["verifiedUpperMillimetres"]["candidateToArchive"] for row in fidelity["parts"]]
        self.assertEqual((max(before), max(after), before.count(0), after.count(0)), (1169, 846, 16, 16))
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
        self.assertEqual(definition["metresPerUnit"], float(Fraction(400, 41)))
        self.assertEqual(definition["provenance"]["frozenNativeArchive"]["faces"], 928)
        self.assertEqual(definition["provenance"]["separateNativeCandidate"]["faces"], 480)
        self.assertEqual(set(definition["detail"]["groups"] + definition["native"]["groups"]), set(range(162)))
        self.assertEqual(len(definition["clearances"]), 7)
        self.assertEqual(definition["engineering"]["massKg"], 74803)
        self.assertEqual(definition["engineering"]["centerOfMass"], finite["centreMetres"])
        self.assertEqual(sum(definition["engineering"]["inertia"], []), finite["inertiaAboutCentreOfMassKgM2"])

    def test_check_replays_outputs_without_writing(self):
        with patch.object(prep, "replace_output", side_effect=AssertionError("check rewrote stream")):
            with patch("sys.argv", ["prepare_yak_geometry", "--check"]):
                prep.main()

    def test_both_complete_reference_fans_revalidate_original_points_without_qhull(self):
        source = tomllib.loads((prep.ORIGINAL / "resource.toml").read_text())
        ownership = json.loads((prep.INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes())["models"]
        candidate = json.loads((prep.ORIGINAL / "SH050-candidate.json").read_bytes())
        for mode, native in (
            ("archive", (source["nativeDesign"], ownership["nativeDesign"]["parts"])),
            ("candidate", (candidate["nativeCandidate"], candidate["parts"])),
        ):
            slots, meshes = measure.assemble((source["design"], ownership["design"]["parts"]), native)
            scaled = np.array(meshes[0]["vertices"] + meshes[1]["vertices"]) * float(prep.SCALE)
            groups = np.array(meshes[0]["groups"] + meshes[1]["groups"])
            records = measure.reference_records(mode, scaled, groups, len(slots))
            with (self.subTest(mode=mode),
                  patch.object(shared_fans, "ConvexHull", side_effect=AssertionError("unfrozen triangulation")),
                  patch.object(shared_fans, "bound_fan", wraps=shared_fans.bound_fan) as outward):
                for (key, identifier), record in zip(slots.items(), records, strict=True):
                    point_ids = np.flatnonzero(groups == identifier)
                    actual = measure.validated_reference_fan(record, key, scaled, point_ids, "Yak")
                    self.assertEqual(actual, [[record["sourceIndices"][i] for i in face] for face in record["triangles"]])
                self.assertEqual(outward.call_count, 162)

            key, identifier = tuple(slots.items())[5]
            point_ids = np.flatnonzero(groups == identifier)
            tampered = deepcopy(records[5])
            tampered["triangles"][0][0] = len(point_ids)
            with patch.object(shared_fans, "bound_fan") as outward:
                with self.assertRaisesRegex(ValueError, "out-of-support Yak"):
                    measure.validated_reference_fan(tampered, key, scaled, point_ids, "Yak")
                outward.assert_not_called()
            with patch.object(measure, "INTEGRATION", shared_fans.INTEGRATION):
                with self.assertRaisesRegex(ValueError, "Changed Yak reference triangulation"):
                    measure.reference_records(mode, scaled, groups, len(slots))
            raw = (prep.INTEGRATION / "REFERENCE-FANS.json").read_bytes()
            with tempfile.TemporaryDirectory() as folder:
                (Path(folder) / "REFERENCE-FANS.json").write_bytes(raw + b" ")
                with patch.object(measure, "INTEGRATION", Path(folder)):
                    with self.assertRaisesRegex(ValueError, "Changed Yak reference triangulation"):
                        measure.reference_records(mode, scaled, groups, len(slots))


if __name__ == "__main__":
    unittest.main()
