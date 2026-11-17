"""Check CI orchestration and failure evidence independently of the native X11 run."""

import contextlib
import importlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, call, patch


SCRIPTS = Path(__file__).resolve().parents[1] / ".github/scripts"
with patch.object(sys, "path", [str(SCRIPTS), *sys.path]):
    workspace = importlib.import_module("coverage_workspace")
    checker = importlib.import_module("check_native_coverage")
    smoke = importlib.import_module("native_coverage_smoke")


class NativeCoverageHarnessTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name).resolve()
        patcher = patch.object(workspace, "ROOT", self.root)
        patcher.start()
        self.addCleanup(patcher.stop)

    def write(self, relative, text):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
        return path

    def test_workspace_paths_refuse_parent_and_absolute_inputs(self):
        self.assertEqual(workspace.workspace_path("target/debug"), self.root / "target/debug")
        for path in ("coverage/../../outside", self.root / "coverage"):
            with self.subTest(path=path), self.assertRaises(ValueError):
                workspace.workspace_path(path)

    def test_fixed_output_refuses_a_symlinked_ancestor(self):
        outside = self.root / "redirected"
        outside.mkdir()
        try:
            (self.root / "coverage").symlink_to(outside, target_is_directory=True)
        except OSError as error:
            self.skipTest(f"Symlinks unavailable: {error}")
        with self.assertRaisesRegex(ValueError, "must not use symlinks"):
            smoke.main()
        self.assertEqual(list(outside.iterdir()), [])

    def native_boundary_report(self):
        # Copy real Rust source. Only the four OS/GPU boundary rows receive fixture hits.
        required = {
            Path("crates/meridian-desktop/src/app.rs"): ("r.map(Renderer::logical_viewport)",),
            Path("crates/meridian-desktop/src/renderer.rs"): (
                "let scale = self.window.scale_factor();",
                "|bytes| self.queue.write_buffer(&self.hud_vertices, 0, bytes),",
            ),
        }
        records = []
        boundaries = []
        for path, markers in required.items():
            source = (SCRIPTS.parents[1] / path).read_text()
            self.write(path, source)
            records.append(f"SF:{self.root / path}")
            for line, text in enumerate(source.splitlines(), 1):
                if text.strip() in markers:
                    boundaries.append((path, line, text.strip()))
                    records.extend([f"DA:{line},0", f"DA:{line},3"])
            records.append("end_of_record")
        report = self.write("coverage/native/desktop.lcov", "\n".join(records))
        self.assertEqual(len(boundaries), 4)
        return report, boundaries

    def test_fixed_report_requires_positive_hits_for_every_native_adapter(self):
        report, boundaries = self.native_boundary_report()
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            checker.main()
        measurements = json.loads(output.getvalue())
        self.assertEqual(sum(len(rows) for rows in measurements.values()), 3)
        observed = [row for markers in measurements.values() for rows in markers.values() for row in rows]
        self.assertEqual(len(observed), 4)
        self.assertTrue(all(row["hits"] == 3 for row in observed))
        queue_line = next(line for _, line, marker in boundaries if "queue.write_buffer" in marker)
        report.write_text(report.read_text().replace(f"DA:{queue_line},3", f"DA:{queue_line},0"))
        with self.assertRaisesRegex(SystemExit, "queue.write_buffer"):
            checker.main()

    def test_zero_or_missing_hits_at_either_os_scale_read_or_other_native_boundary_refuse(self):
        report, boundaries = self.native_boundary_report()
        original = report.read_text()
        for _, line, marker in boundaries:
            for count in ("zero", "missing"):
                with self.subTest(marker=marker, line=line, count=count):
                    replacement = f"DA:{line},0" if count == "zero" else ""
                    changed = original.replace(f"DA:{line},3", replacement)
                    if count == "missing":
                        changed = changed.replace(f"DA:{line},0", "")
                    report.write_text(changed)
                    with self.assertRaises(SystemExit):
                        checker.main()

    def test_missing_or_duplicate_source_occurrences_refuse_despite_positive_fixture_hits(self):
        for occurrence_count in ("missing", "duplicate"):
            for marker in (
                "r.map(Renderer::logical_viewport)",
                "let scale = self.window.scale_factor();",
                "|bytes| self.queue.write_buffer(&self.hud_vertices, 0, bytes),",
            ):
                with self.subTest(marker=marker, occurrence_count=occurrence_count):
                    _, boundaries = self.native_boundary_report()
                    path = next(path for path, _, text in boundaries if text == marker)
                    source = self.root / path
                    text = source.read_text()
                    changed = (text.replace(marker, "", 1) if occurrence_count == "missing"
                               else text + "\n" + marker + "\n")
                    source.write_text(changed)
                    with self.assertRaises(SystemExit):
                        checker.main()

    def test_helper_scale_and_other_queue_hits_cannot_replace_the_native_boundary_hits(self):
        report, boundaries = self.native_boundary_report()
        renderer = Path("crates/meridian-desktop/src/renderer.rs")
        source = (self.root / renderer).read_text().splitlines()
        helper = next(n for n, text in enumerate(source, 1) if "to_logical::<f64>(scale_factor())" in text)
        other_queue = next(n for n, text in enumerate(source, 1) if text.strip() == "self.queue.write_buffer(")
        records = [f"SF:{self.root / renderer}", f"DA:{helper},9", f"DA:{other_queue},9", "end_of_record"]
        for _, line, marker in boundaries:
            if "scale_factor" not in marker and "queue.write_buffer" not in marker:
                continue
            with self.subTest(marker=marker, line=line):
                original, _ = self.native_boundary_report()
                report.write_text(original.read_text().replace(f"DA:{line},3", "") + "\n" + "\n".join(records))
                with self.assertRaises(SystemExit):
                    checker.main()

    def test_smoke_rejects_missing_instrumented_binaries_before_creating_output(self):
        with self.assertRaisesRegex(RuntimeError, "must be built under target/debug"):
            smoke.main()
        self.assertFalse((self.root / "coverage").exists())

    def test_ordinary_new_commander_accepts_current_title_and_keeps_frame_and_quit_gates(self):
        save = self.root / "save"
        save.mkdir()
        app = Mock()
        app.wait.return_value = 0
        receipt = {"phases": []}
        titles = ["Meridian", "Meridian | DOCKED | RUNNING | H / ? Help", "market", "hud"]
        frames = [
            {"phase": name, "sha256": name, "title": title}
            for name, title in zip(("title", "system", "trading", "hud"), titles)
        ]
        with (patch.object(smoke, "await_result", return_value="41"),
              patch.object(smoke, "focus") as focus,
              patch.object(smoke, "capture_phase", side_effect=frames) as capture,
              patch.object(smoke, "press") as press):
            smoke.exercise(app, self.root, save, receipt)

        focus.assert_called_once_with("41")
        self.assertEqual(capture.call_args_list, [
            call(app, "41", self.root, "title", smoke.title_frame),
            call(app, "41", self.root, "system", smoke.system_frame),
            call(app, "41", self.root, "trading", smoke.trading_frame),
            call(app, "41", self.root, "hud", smoke.cockpit_frame),
        ])
        self.assertEqual(press.call_args_list, [call("n"), call("4"), call("Escape"), call("Escape")])
        self.assertEqual(receipt["phases"], frames)
        self.assertTrue(receipt["normal_quit_sent"])
        app.wait.assert_called_once_with(timeout=20)

    def test_new_commander_guard_rejects_title_menu_pause_and_other_commander_states(self):
        save = self.root / "save"
        save.mkdir()
        for title in (
            "Meridian",
            "Meridian | DOCKED | MENU | H / ? Help",
            "Meridian | DOCKED | PAUSED | H / ? Help",
            "Meridian | FLIGHT | RUNNING | H / ? Help",
            "Meridian | LAUNCHING | RUNNING | H / ? Help",
            "CLARKE / XYZZY | DOCKED | RUNNING | H / ? Help",
            "Meridian | DOCKED | MENU | New commander ready",
        ):
            with self.subTest(title=title):
                app = Mock()
                app.wait.return_value = 0
                receipt = {"phases": []}
                frames = [
                    {"phase": "title", "sha256": "title", "title": "Meridian"},
                    {"phase": "system", "sha256": "system", "title": title},
                    {"phase": "trading", "sha256": "trading", "title": "market"},
                    {"phase": "hud", "sha256": "hud", "title": "hud"},
                ]
                with (patch.object(smoke, "await_result", return_value="41"),
                      patch.object(smoke, "focus"),
                      patch.object(smoke, "capture_phase", side_effect=frames) as capture,
                      patch.object(smoke, "press") as press,
                      self.assertRaisesRegex(RuntimeError, "ordinary commander creation")):
                    smoke.exercise(app, self.root, save, receipt)

                self.assertEqual(receipt["stage"], "system")
                capture.assert_called_with(app, "41", self.root, "system", smoke.system_frame)
                press.assert_called_once_with("n")
                app.wait.assert_not_called()

    def test_failure_receipt_retains_save_and_observer_streams_before_cleanup(self):
        self.write("target/debug/meridian-desktop", "desktop fixture")
        self.write("target/debug/examples/inspect_commander", "observer fixture")
        save_bytes = b"isolated test save"
        self.save_path = None

        def finished_app(argv, env, output, save, receipt):
            self.save_path = save
            (save / smoke.SAVE_NAME).write_bytes(save_bytes)
            (save / "session.lock").touch()
            (output / "desktop-41.profraw").write_bytes(b"profile")
            (output / "desktop.log").write_text(
                'Meridian adapter: llvmpipe, device_type: Cpu, backend: Vulkan\n'
            )
            receipt.update(pid=41, exit=0, normal_quit_sent=True, exit_status="normal quit")

        def refused_observer(argv, *, env, stdout, stderr):
            stdout.write(b"partial observer stdout\n")
            stderr.write(b"invalid commander\n")
            return type("RejectedObserver", (), {"pid": 42, "wait": lambda self, timeout: 2})()

        with (patch.object(smoke, "run_app", side_effect=finished_app),
              patch.object(smoke, "run", return_value=b"fixture-head\n"),
              patch.object(smoke.subprocess, "Popen", side_effect=refused_observer),
              contextlib.redirect_stdout(io.StringIO()), self.assertRaises(subprocess.CalledProcessError)):
            smoke.main()

        output = self.root / "coverage/native"
        result = json.loads((output / "native-smoke.json").read_text())
        self.assertEqual((result["stage"], result["status"]), ("save and observer", "failed"))
        self.assertEqual((result["pid"], result["exit_status"]), (41, "normal quit"))
        self.assertEqual((result["observer"]["pid"], result["observer"]["exit_code"]), (42, 2))
        self.assertEqual((output / result["observer"]["stdout"]).read_bytes(), b"partial observer stdout\n")
        self.assertEqual((output / result["observer"]["stderr"]).read_bytes(), b"invalid commander\n")
        self.assertEqual((output / result["failed_save"]["artifact"]).read_bytes(), save_bytes)
        self.assertFalse(self.save_path.exists())

    def test_log_and_exit_classifications_reject_incomplete_native_runs(self):
        log = self.write("desktop.log", "Meridian adapter: device_type: Cpu, backend: Gl\n")
        with self.assertRaisesRegex(RuntimeError, "Expected Mesa"):
            smoke.verify_log(log)
        log.write_text("Meridian adapter: llvmpipe, device_type: Cpu, backend: Vulkan\nMeridian GPU error\n")
        with self.assertRaisesRegex(RuntimeError, "rendering or profile error"):
            smoke.verify_log(log)
        receipt = {"forced_cleanup": False, "normal_quit_sent": False, "exit": 0}
        self.assertEqual(smoke.exit_status(receipt), "unexpected exit")
        receipt.update(normal_quit_sent=True)
        self.assertEqual(smoke.exit_status(receipt), "normal quit")
        receipt.update(forced_cleanup=True)
        self.assertEqual(smoke.exit_status(receipt), "forced cleanup")
