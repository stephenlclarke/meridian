"""Authoring imports, measurements and the local validator boundary."""

import io
import json
import struct
import sys
import tempfile
import threading
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).parent / "hull_studio"))
import definitions
import fit
import studio


def sample():
    mesh = {"vertices": [[0, 0, 0], [1, 0, 0], [0, 1, 0]], "triangles": [[0, 1, 2]]}
    return {
        "id": "TEST-SKIF",
        "detail": dict(mesh),
        "native": dict(mesh),
        "metresPerUnit": 1,
        "roundingMetres": 0.01,
    }


class HullStudioTests(unittest.TestCase):
    def test_reference_keeps_exact_original_topology_and_appearance(self):
        src = definitions.ROOT / "assets/fleet/sh023"
        hull = definitions.parse(definitions.reference_bytes())
        self.assertEqual(hull["engineering"]["massKg"], 101208)
        self.assertEqual(
            [len(hull[k]["triangles"]) for k in ("detail", "native")], [3484, 512]
        )
        groups = bytes(hull["detail"]["groups"] + hull["native"]["groups"])
        self.assertEqual(groups, (src / "source-piece-membership.bin").read_bytes())
        self.assertEqual(len(set(groups)), 69)
        positions = b"".join(
            struct.pack("<3d", *p)
            for k in ("detail", "native")
            for p in hull[k]["vertices"]
        )
        self.assertEqual(positions, (src / "source.vertices").read_bytes()[16:])
        self.assertEqual(len(hull["faceTextures"]), 3484)
        self.assertEqual(len(hull["faceMaterials"]), 3484)
        self.assertIn("MIT License", hull["provenance"]["license"])
        self.assertEqual(len(hull["clearances"]), 4)
        self.assertLessEqual(fit.fit_report(hull, {})["maxMetres"], 1.133)

    def test_grouping_is_disjoint_unless_explicitly_shared(self):
        hull = sample()
        hull["detail"]["vertices"] += [[10, 0, 0], [10, 1, 0], [10, 0, 1]]
        hull["detail"]["triangles"] = [[0, 1, 2], [3, 4, 5]]
        result = definitions.prepare(hull)
        self.assertEqual(result["detail"]["groups"], [0, 0, 0, 1, 1, 1])
        self.assertTrue(
            set(result["native"]["groups"]).isdisjoint(result["detail"]["groups"])
        )
        self.assertNotIn("groups", hull["detail"])
        hull["native"]["groups"] = [7, 7, 7]
        hull["detail"]["groups"] = [7, 7, 7, 8, 8, 8]
        self.assertEqual(definitions.prepare(hull), hull)

    def test_invalid_input_and_external_path_refused(self):
        for raw in [b'{"id":1,"id":2}', b"[NaN]", b"[Infinity]"]:
            with self.assertRaises(ValueError):
                definitions.parse(raw)
        hull = sample()
        hull["detail"]["triangles"][0] = [0, 1, -1]
        with self.assertRaisesRegex(ValueError, "invalid triangle"):
            definitions.prepare(hull)
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            p = root / "hull.json"
            (root / "detail.json").write_bytes(b'{"vertices":[]}')
            p.write_text('{"detail":{"file":"detail.json"}}')
            self.assertEqual(
                definitions.local_definition(p)["detail"], {"vertices": []}
            )
            p.write_text('{"detail":{"file":"../other.json"}}')
            with self.assertRaisesRegex(ValueError, "directory"):
                definitions.local_definition(p)

    def test_nearest_sample_uses_face_interiors_and_respects_scale(self):
        face = [[0, 0, 0], [4, 0, 0], [0, 4, 0]]
        for query, expected in [
            ([1, 1, 1], [1, 1, 0]),
            ([5, 0, 1], [4, 0, 0]),
            ([2, 3, 1], [1.5, 2.5, 0]),
        ]:
            self.assertEqual(list(fit.closest(query, face)), expected)
        package = definitions.prepare(sample())
        package["metresPerUnit"] = 4
        envelope = {
            "vertices": [[1, 1, 1], [1, 2, 1], [2, 1, 1]],
            "triangles": [[0, 1, 2]],
        }
        measured = fit.fit_report(package, envelope)
        self.assertEqual(measured["maxMetres"], 1)
        self.assertEqual(measured["nearestPoint"][2], 0)

    def test_archived_mesh_uses_inspect_and_refuses_flight_export(self):
        archive = sample() | {"schema": 2}
        with self.assertRaisesRegex(ValueError, "cannot be exported for flight"):
            studio.flight_export_definition(archive)
        ordinary = sample() | {"schema": 1}
        self.assertIs(studio.flight_export_definition(ordinary), ordinary)

        def validate(args, **kwargs):
            self.assertEqual(args[1], "inspect")
            self.assertEqual(definitions.parse(Path(args[2]).read_bytes())["schema"], 2)
            Path(args[4]).write_bytes(b'{"valid":true}')
            return mock.Mock(returncode=0)

        with mock.patch.object(studio.subprocess, "run", side_effect=validate):
            _, report = studio.build(archive, Path("/example/meridian-hull-tool"))
        self.assertEqual(report, {"valid": True})

    def test_game_preview_uses_a_private_profile_and_the_exact_package(self):
        preview = studio.GamePreview(Path("/example/meridian-desktop"))
        finished, cleaned = threading.Event(), threading.Event()
        child = mock.Mock()
        child.stdout = io.BytesIO(b"MERIDIAN_HULL_STUDIO_READY\n")
        child.wait.side_effect = finished.wait
        original = tempfile.TemporaryDirectory

        def workspace(*args, **kwargs):
            folder = original(*args, **kwargs)
            cleanup = folder.cleanup

            def close():
                cleanup()
                cleaned.set()

            folder.cleanup = close
            return folder

        with mock.patch.object(
            studio.subprocess, "Popen", return_value=child
        ) as launch:
            with mock.patch.object(
                studio.tempfile, "TemporaryDirectory", side_effect=workspace
            ):
                preview.launch({"id": "CONFIG-ONLY"})
            try:
                argv = launch.call_args.args[0]
                profile = Path(argv[argv.index("--save-dir") + 1])
                authored = Path(argv[argv.index("--inspect-hull-package") + 1])
                self.assertEqual(authored.parent, profile.parent)
                self.assertEqual(
                    json.loads(authored.read_bytes()), {"id": "CONFIG-ONLY"}
                )
                self.assertEqual(list(profile.iterdir()), [])
                self.assertIn("--open-authored-showroom", argv)
                with self.assertRaisesRegex(RuntimeError, "Close the open Showroom"):
                    preview.launch({"id": "OTHER"})
            finally:
                finished.set()
            self.assertTrue(cleaned.wait(2))
            self.assertFalse(profile.parent.exists())

    def test_game_launch_failure_frees_the_private_workspace(self):
        preview = studio.GamePreview(Path("/missing/desktop"))
        with mock.patch.object(
            studio.subprocess, "Popen", side_effect=OSError("missing")
        ) as launch:
            with self.assertRaisesRegex(OSError, "missing"):
                preview.launch({})
            self.assertFalse(launch.call_args.kwargs["cwd"].exists())
            self.assertFalse(preview.running)
        with self.assertRaisesRegex(RuntimeError, "unavailable"):
            studio.GamePreview(None).launch({})

    def test_process_start_without_a_rendered_frame_reports_the_actual_error(self):
        preview = studio.GamePreview(Path("/example/meridian-desktop"))
        child = mock.Mock(stdout=io.BytesIO(b"Graphics adapter unavailable\n"))
        child.wait.return_value = 1
        with mock.patch.object(studio.subprocess, "Popen", return_value=child):
            with self.assertRaisesRegex(RuntimeError, "Graphics adapter unavailable"):
                preview.launch({"id": "TEST"})
        self.assertFalse(preview.running)

    def test_validation_failure_never_exports(self):
        def reject(argv, **kwargs):
            self.assertEqual(argv[1], "validate")
            self.assertEqual(json.loads(Path(argv[2]).read_bytes())["id"], "TEST-SKIF")
            report = {"valid": False, "diagnostics": [{"message": "Blocked bow"}]}
            Path(argv[-1]).write_bytes(definitions.encode(report))
            return studio.subprocess.CompletedProcess(argv, 1, b"", b"Blocked bow")

        with mock.patch.object(studio.subprocess, "run", side_effect=reject):
            with self.assertRaisesRegex(studio.InvalidHull, "Blocked bow") as context:
                studio.build(sample(), Path("validator"))
            self.assertFalse(context.exception.report["valid"])


if __name__ == "__main__":
    unittest.main()
