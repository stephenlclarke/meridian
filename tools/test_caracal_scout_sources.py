"""Verify the two original Caracal Scout meshes, separate candidate and grouped artwork."""

from copy import deepcopy
from fractions import Fraction
import hashlib
import inspect
import json
from pathlib import Path
import struct
import tempfile
import tomllib
import unittest
from unittest.mock import patch

import numpy as np

import measure_banteng_geometry as shared_banteng
import measure_caracal_scout_geometry as measure
import measure_lanner_geometry as shared_fans
import prepare_caracal_scout_candidate as recipe
import prepare_caracal_scout_geometry as prep
import verify_caracal_scout_geometry as proof


class CaracalScoutSources(unittest.TestCase):
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
        candidate_bytes = (prep.ORIGINAL / "SH053-project-candidate.json").read_bytes()
        candidate = json.loads(candidate_bytes)
        self.assertEqual(prep.pin(candidate_bytes)["sha256"], prep.CANDIDATE_SHA)
        self.assertEqual(resource["nativeDesign"], archive["nativeDesign"])
        self.assertEqual((len(archive["nativeDesign"]["faces"]), len(candidate["nativeCandidate"]["faces"])), (1380, 492))
        self.assertEqual(contract["geometry"]["nativeFaceLimit"], 512)
        self.assertEqual(candidate["status"], "PROJECT_AUTHORED_ARTWORK_CANDIDATE")
        self.assertFalse(resource["catalogue"]["playerFlyable"])
        self.assertFalse(resource["catalogue"]["normalPurchaseEligible"])
        self.assertTrue(contract["access"]["playerFlyable"])
        self.assertEqual(contract["access"]["purchasePolicy"], "blueprintFabrication")

    def test_grouping_keeps_every_triangle_and_uv(self):
        inputs = prep.load_frozen()
        resource = tomllib.loads(inputs["resource.toml"].decode())
        candidate = json.loads((prep.ORIGINAL / "SH053-project-candidate.json").read_bytes())["nativeCandidate"]
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
        for name, faces in (("visual.bin", 5372), ("archive-native-visual.bin", 1380), ("candidate-native-visual.bin", 492)):
            self.assertEqual(struct.unpack_from("<8sII", outputs[name]), (b"CACSVIZ1", faces * 3, 56))

    def test_candidate_fidelity_and_both_contact_replays(self):
        observed = proof.verify()
        self.assertEqual(prep.encode(observed), (prep.INTEGRATION / "CONTACT-RESULT.json").read_bytes())
        self.assertEqual(observed["detailedWithArchivedNative"], observed["detailedWithCandidateNative"])
        self.assertEqual(sorted(observed["detailedWithCandidateNative"].values()), [317, 319, 358, 358, 597, 1376, 1376])
        fidelity = json.loads((prep.INTEGRATION / "candidate-fidelity.json").read_bytes())
        before = [row["verifiedUpperMillimetres"]["archiveToCandidate"] for row in fidelity["parts"]]
        after = [row["verifiedUpperMillimetres"].get("candidateToArchive", 0) for row in fidelity["parts"]]
        self.assertEqual((max(before), max(after)), (1761, 1233))
        self.assertEqual((fidelity["fixedParts"], fidelity["resampledParts"], fidelity["omittedBlindWellParts"]),
                         (15, 10, 14))
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
        self.assertEqual(definition["metresPerUnit"], float(Fraction(1500000, 171707)))
        self.assertEqual(definition["provenance"]["frozenNativeArchive"]["faces"], 1380)
        self.assertEqual(definition["provenance"]["separateNativeCandidate"]["faces"], 492)
        self.assertEqual(set(definition["detail"]["groups"] + definition["native"]["groups"]), set(range(89)))
        self.assertEqual(len(definition["clearances"]), 7)
        self.assertEqual(definition["engineering"]["massKg"], 40097)
        self.assertEqual(definition["engineering"]["centerOfMass"], finite["centreMetres"])
        self.assertEqual(sum(definition["engineering"]["inertia"], []), finite["inertiaAboutCentreOfMassKgM2"])

    def test_project_recipe_replays_without_changing_archived_parts(self):
        native, ownership, _ = recipe.source()
        candidate = recipe.build()
        self.assertEqual(recipe.encode_candidate(), recipe.PATH.read_bytes())
        self.assertEqual((len(candidate["parts"]), len(candidate["nativeCandidate"]["faces"])), (25, 492))
        omitted = set(candidate["omittedBlindWellParts"])
        self.assertEqual(len(omitted), 14)
        self.assertTrue(all(ownership[index]["creatorMethod"] == "Mesh.well" for index in omitted))
        self.assertEqual(len(native["faces"]), 1380)
        self.assertEqual({part["originalPartIndex"] for part in candidate["parts"]} | omitted, set(range(39)))

    def test_check_replays_outputs_without_writing(self):
        with patch.object(prep, "replace_output", side_effect=AssertionError("check rewrote stream")):
            with patch("sys.argv", ["prepare_caracal_scout_geometry", "--check"]):
                prep.main()

    def test_shared_banteng_functions_retain_the_original_pinned_sources(self):
        criteria = json.loads((prep.INTEGRATION / "CRITERIA.json").read_bytes())
        hashes = criteria["sharedBantengDependency"]["importedFunctionSourceSha256"]
        self.assertEqual(set(hashes), {"adaptive_upper", "section", "solid", "upper"})
        for name, expected in hashes.items():
            function = getattr(shared_banteng, name)
            self.assertIs(getattr(measure, name), function)
            self.assertEqual(hashlib.sha256(inspect.getsource(function).encode()).hexdigest(), expected)

    def test_both_complete_reference_fans_revalidate_original_points_without_qhull(self):
        source = tomllib.loads((prep.ORIGINAL / "resource.toml").read_text())
        ownership = json.loads((prep.INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes())["models"]
        candidate = json.loads((prep.ORIGINAL / "SH053-project-candidate.json").read_bytes())
        for mode, native in (
            ("archive", (source["nativeDesign"], ownership["nativeDesign"]["parts"])),
            ("candidate", (candidate["nativeCandidate"], candidate["parts"])),
        ):
            slots, meshes = measure.assemble((source["design"], ownership["design"]["parts"]), native)
            scaled = np.array(meshes[0]["vertices"] + meshes[1]["vertices"]) * float(prep.SCALE)
            groups = np.array(meshes[0]["groups"] + meshes[1]["groups"])
            records = measure.reference_records(mode, scaled, groups, len(slots))
            self.assertEqual(len(records), 89)
            self.assertEqual(sum(len(record["triangles"]) for record in records),
                             2972 if mode == "archive" else 2804)
            self.assertEqual(sum(len(record["sourceIndices"]) for record in records),
                             3992 if mode == "archive" else 3520)
            with (self.subTest(mode=mode),
                  patch.object(shared_fans, "ConvexHull", side_effect=AssertionError("unfrozen triangulation")),
                  patch.object(shared_fans, "bound_fan", wraps=shared_fans.bound_fan) as outward):
                for (key, identifier), record in zip(slots.items(), records, strict=True):
                    point_ids = np.flatnonzero(groups == identifier)
                    actual = measure.validated_reference_fan(record, key, scaled, point_ids, "Caracal Scout")
                    self.assertEqual(actual, [[record["sourceIndices"][i] for i in face] for face in record["triangles"]])
                self.assertEqual(outward.call_count, 89)

            key, identifier = tuple(slots.items())[5]
            point_ids = np.flatnonzero(groups == identifier)
            tampered = deepcopy(records[5])
            tampered["triangles"][0][0] = len(point_ids)
            with patch.object(shared_fans, "bound_fan") as outward:
                with self.assertRaisesRegex(ValueError, "out-of-support Caracal Scout"):
                    measure.validated_reference_fan(tampered, key, scaled, point_ids, "Caracal Scout")
                outward.assert_not_called()
            tampered = deepcopy(records[5])
            tampered["sourceIndices"][0] += 1
            with self.assertRaisesRegex(ValueError, "Caracal Scout reference support point identity changed"):
                measure.validated_reference_fan(tampered, key, scaled, point_ids, "Caracal Scout")
            tampered = deepcopy(records[5])
            face = tampered["triangles"][0]
            face[1], face[2] = face[2], face[1]
            tampered["triangles"].sort()
            with self.assertRaisesRegex(ValueError, "Caracal Scout reference fan failed exact outward or radial proof"):
                measure.validated_reference_fan(tampered, key, scaled, point_ids, "Caracal Scout")
            with patch.object(measure, "INTEGRATION", shared_fans.INTEGRATION):
                with self.assertRaisesRegex(ValueError, "Changed Caracal Scout reference triangulation"):
                    measure.reference_records(mode, scaled, groups, len(slots))
            raw = (prep.INTEGRATION / "REFERENCE-FANS.json").read_bytes()
            with tempfile.TemporaryDirectory() as folder:
                (Path(folder) / "REFERENCE-FANS.json").write_bytes(raw + b" ")
                with patch.object(measure, "INTEGRATION", Path(folder)):
                    with self.assertRaisesRegex(ValueError, "Changed Caracal Scout reference triangulation"):
                        measure.reference_records(mode, scaled, groups, len(slots))


if __name__ == "__main__":
    unittest.main()
