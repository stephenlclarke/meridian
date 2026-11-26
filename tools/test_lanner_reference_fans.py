"""Verify the pinned Lanner proof topology is exact, complete and tamper-evident."""

from copy import deepcopy
import hashlib
import json
from pathlib import Path
import tempfile
import tomllib
import unittest
from unittest.mock import patch

import numpy as np

import measure_lanner_geometry as measure


class LannerReferenceFans(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw = (measure.INTEGRATION / "REFERENCE-FANS.json").read_bytes()
        cls.document = json.loads(cls.raw)
        source = tomllib.loads((measure.ORIGINAL / "resource.toml").read_text())
        owners = json.loads((measure.INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes())["models"]
        candidate = json.loads((measure.ORIGINAL / "SH048-candidate.json").read_bytes())
        cls.modes = {}
        for mode, native in (
            ("archive", (source["nativeDesign"], owners["nativeDesign"]["parts"], 8)),
            ("candidate", (candidate["nativeCandidate"], candidate["parts"], 4)),
        ):
            slots, meshes = measure.assemble((source["design"], owners["design"]["parts"], 12), native)
            scaled = np.array(meshes[0]["vertices"] + meshes[1]["vertices"]) * float(measure.SCALE)
            groups = np.array(meshes[0]["groups"] + meshes[1]["groups"])
            cls.modes[mode] = slots, scaled, groups

    def records(self, mode="archive"):
        slots, scaled, groups = self.modes[mode]
        return measure.reference_records(mode, scaled, groups, len(slots))

    def test_both_complete_reference_fans_prove_outward_bounds_without_qhull(self):
        engine_groups = {
            "engine_pair/negative side/connected support",
            "engine_pair/positive side/connected support",
        }
        for mode in self.modes:
            slots, scaled, groups = self.modes[mode]
            records = self.records(mode)
            with (patch.object(measure, "ConvexHull", side_effect=AssertionError("unfrozen triangulation")),
                  patch.object(measure, "bound_fan", wraps=measure.bound_fan) as proof):
                seen = set()
                for (key, identifier), record in zip(slots.items(), records, strict=True):
                    point_ids = np.flatnonzero(groups == identifier)
                    actual = measure.validated_reference_fan(record, key, scaled, point_ids)
                    self.assertEqual(actual, [[record["sourceIndices"][i] for i in face] for face in record["triangles"]])
                    if key[0] in engine_groups:
                        # These two cross-platform fan differences are not exactly coplanar;
                        # the existing exact outward/radial proof must run on their original points.
                        seen.add(key[0])
                        checked_points, checked_faces = proof.call_args.args
                        np.testing.assert_array_equal(checked_points, scaled[point_ids])
                        self.assertEqual(checked_faces, record["triangles"])
                self.assertEqual(seen, engine_groups)
                self.assertEqual(proof.call_count, 75)

    def test_certificate_hash_rejects_even_valid_json_mutation(self):
        with tempfile.TemporaryDirectory() as folder:
            (Path(folder) / "REFERENCE-FANS.json").write_bytes(self.raw + b" ")
            with patch.object(measure, "INTEGRATION", Path(folder)):
                with self.assertRaisesRegex(ValueError, "Changed Lanner reference triangulation"):
                    self.records()

    def test_malformed_document_and_point_stream_are_refused_even_with_new_hash(self):
        cases = (
            (lambda doc: doc.update(schema=2), "schema or provenance"),
            (lambda doc: doc.update(derivedFromDiagnosticSha256="0" * 64), "schema or provenance"),
            (lambda doc: doc["modes"].pop("candidate"), "Missing or extra.*mode"),
            (lambda doc: doc["modes"].update(extra={}), "Missing or extra.*mode"),
            (lambda doc: doc["modes"]["archive"]["supports"].pop(), "support records"),
            (lambda doc: doc["modes"]["candidate"]["supports"].append({}), "support records"),
            (lambda doc: doc["modes"]["archive"].update(scaledSha256="0" * 64), "point stream or membership"),
            (lambda doc: doc["modes"]["archive"].update(groupSha256="0" * 64), "point stream or membership"),
        )
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "REFERENCE-FANS.json"
            for mutate, message in cases:
                doc = deepcopy(self.document)
                mutate(doc)
                raw = json.dumps(doc).encode()
                path.write_bytes(raw)
                with (self.subTest(message=message), patch.object(measure, "INTEGRATION", Path(folder)),
                      patch.object(measure, "REFERENCE_FANS_SHA", hashlib.sha256(raw).hexdigest())):
                    with self.assertRaisesRegex(ValueError, message):
                        self.records()

    def test_invalid_support_records_refuse_before_outward_proof(self):
        slots, scaled, groups = self.modes["archive"]
        key, identifier = tuple(slots.items())[53]
        point_ids = np.flatnonzero(groups == identifier)
        original = self.records()[53]
        cases = (
            (lambda r: r.update(part="another support"), "Malformed or misplaced"),
            (lambda r: r.update(occurrence=True), "Malformed or misplaced"),
            (lambda r: r.update(extra=1), "Malformed or misplaced"),
            (lambda r: r["sourceIndices"].reverse(), "point identity changed"),
            (lambda r: r.update(pointsSha256="0" * 64), "point identity changed"),
            (lambda r: r["triangles"][0].__setitem__(0, -1), "out-of-support"),
            (lambda r: r["triangles"][0].__setitem__(0, len(point_ids)), "out-of-support"),
            (lambda r: r["triangles"][0].__setitem__(0, True), "out-of-support"),
            (lambda r: r["triangles"].append(r["triangles"][0]), "Duplicate or noncanonical"),
            (lambda r: r["triangles"].pop(), "not a closed sphere"),
        )
        for mutate, message in cases:
            record = deepcopy(original)
            mutate(record)
            with self.subTest(message=message), patch.object(measure, "bound_fan") as proof:
                with self.assertRaisesRegex(ValueError, message):
                    measure.validated_reference_fan(record, key, scaled, point_ids)
                proof.assert_not_called()

    def test_reversed_reference_face_fails_exact_outward_proof(self):
        slots, scaled, groups = self.modes["archive"]
        key, identifier = tuple(slots.items())[53]
        record = deepcopy(self.records()[53])
        a, b, c = record["triangles"][0]
        record["triangles"][0] = [a, c, b]
        record["triangles"].sort()
        with self.assertRaisesRegex(ValueError, "failed exact outward or radial proof"):
            measure.validated_reference_fan(record, key, scaled, np.flatnonzero(groups == identifier))

    def test_shared_qhull_fan_and_rejection_of_duplicate_and_disconnected_points(self):
        vertices = np.array([(0., 0., 0.), (1., 0., 0.), (0., 1., 0.), (0., 0., 1.), (0., 0., 0.)])
        ids = np.arange(len(vertices))
        fan = measure.make_fan(vertices, ids)  # Shared by other hull measurements, including Hyena.
        record = {"part": "tetrahedron", "occurrence": 1, "sourceIndices": ids.tolist(),
                  "pointsSha256": measure.array_sha256(vertices, "<f8"), "triangles": fan}
        key = "tetrahedron", 1
        self.assertEqual(measure.validated_reference_fan(record, key, vertices, ids), fan)
        duplicate = deepcopy(record)
        replacement = []
        for face in fan:
            changed = [4 if i == 0 else i for i in face]
            start = changed.index(min(changed))
            replacement.append(changed[start:] + changed[:start])
        duplicate["triangles"] = sorted(replacement)
        with self.assertRaisesRegex(ValueError, "duplicate point representative"):
            measure.validated_reference_fan(duplicate, key, vertices, ids)

        vertices = np.concatenate((vertices[:4], vertices[:4] + 3.))
        ids = np.arange(len(vertices))
        record.update(sourceIndices=ids.tolist(), pointsSha256=measure.array_sha256(vertices, "<f8"),
                      triangles=fan + [[i + 4 for i in face] for face in fan])
        with self.assertRaisesRegex(ValueError, "Disconnected Lanner reference topology"):
            measure.validated_reference_fan(record, key, vertices, ids)


if __name__ == "__main__":
    unittest.main()
