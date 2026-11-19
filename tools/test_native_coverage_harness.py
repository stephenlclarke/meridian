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
        other_queue = next(n for n, text in enumerate(source, 1)
                           if ".write_buffer(&self.projection," in text)
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

    def test_ordinary_start_opens_commander_without_new_input_and_keeps_frame_and_quit_gates(self):
        save = self.root / "save"
        save.mkdir()
        app = Mock()
        app.wait.return_value = 0
        receipt = {"phases": []}
        titles = [smoke.COMMANDER_TITLE, "Meridian | DOCKED | RUNNING | H / ? Help", "market", "hud"]
        frames = [
            {"phase": name, "sha256": name, "title": title}
            for name, title in zip(("commander", "system", "trading", "hud"), titles)
        ]
        with (patch.object(smoke, "await_result", return_value="41"),
              patch.object(smoke, "focus") as focus,
              patch.object(smoke, "capture_phase", side_effect=frames) as capture,
              patch.object(smoke, "press") as press):
            smoke.exercise(app, self.root, save, receipt)

        focus.assert_called_once_with("41")
        self.assertEqual(capture.call_args_list, [
            call(app, "41", self.root, "commander", smoke.commander_frame),
            call(app, "41", self.root, "system", smoke.system_frame),
            call(app, "41", self.root, "trading", smoke.trading_frame),
            call(app, "41", self.root, "hud", smoke.cockpit_frame),
        ])
        self.assertEqual(press.call_args_list, [call("2"), call("4"), call("Escape"), call("Escape")])
        self.assertEqual(receipt["phases"], frames)
        self.assertTrue(receipt["normal_quit_sent"])
        app.wait.assert_called_once_with(timeout=20)

    def test_system_guard_rejects_wrong_window_title_and_other_commander_states(self):
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
                    {"phase": "commander", "sha256": "commander", "title": smoke.COMMANDER_TITLE},
                    {"phase": "system", "sha256": "system", "title": title},
                    {"phase": "trading", "sha256": "trading", "title": "market"},
                    {"phase": "hud", "sha256": "hud", "title": "hud"},
                ]
                with (patch.object(smoke, "await_result", return_value="41"),
                      patch.object(smoke, "focus"),
                      patch.object(smoke, "capture_phase", side_effect=frames) as capture,
                      patch.object(smoke, "press") as press,
                      self.assertRaisesRegex(RuntimeError, "ordinary Commander details")):
                    smoke.exercise(app, self.root, save, receipt)

                self.assertEqual(receipt["stage"], "system")
                capture.assert_called_with(app, "41", self.root, "system", smoke.system_frame)
                press.assert_called_once_with("2")
                app.wait.assert_not_called()

    def test_cold_commander_checks_the_same_first_frame_and_unmodified_save_before_quit(self):
        save = self.root / smoke.SAVE_NAME
        save.write_bytes(b"existing commander")
        digest = smoke.sha256(save)
        frame = {"title": smoke.COMMANDER_TITLE, "sha256": "same-commander-frame"}
        app = Mock()
        app.wait.return_value = 0
        for saved_digest, frame_digest in ((digest, frame["sha256"]), ("wrong", frame["sha256"]),
                                            (digest, "wrong")):
            with self.subTest(saved_digest=saved_digest, frame_digest=frame_digest):
                receipt = {"phases": [], "save_sha256_before_start": saved_digest,
                           "fresh_commander_frame_sha256": frame_digest}
                with (patch.object(smoke, "await_result", return_value="41"),
                      patch.object(smoke, "focus"),
                      patch.object(smoke, "capture_phase", return_value=frame) as capture,
                      patch.object(smoke, "press") as press):
                    if saved_digest == digest and frame_digest == frame["sha256"]:
                        smoke.cold_commander(app, self.root, self.root, receipt)
                        press.assert_called_once_with("Escape")
                        self.assertTrue(receipt["normal_quit_sent"])
                    else:
                        with self.assertRaisesRegex(RuntimeError, "without rewriting its save"):
                            smoke.cold_commander(app, self.root, self.root, receipt)
                        press.assert_not_called()
                    capture.assert_called_once_with(app, "41", self.root, "cold-commander",
                                                    smoke.commander_frame)
                    self.assertEqual(receipt["save_sha256_at_first_frame"], digest)

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

    def test_mule_native_report_requires_the_production_upload_and_both_draws(self):
        source = (SCRIPTS.parents[1] / checker.MULE_GPU).read_text()
        self.write(checker.MULE_GPU, source)
        markers = {
            line for line, text in enumerate(source.splitlines(), 1)
            if text.strip() in checker.MULE_GPU_MARKERS
        }
        self.assertEqual(len(markers), 8)
        rows = markers | set(range(1, 268))
        hit_rows = markers | set(range(1, 230))
        target = self.write(
            "coverage/native/mule-desktop.lcov",
            f"SF:{self.root / checker.MULE_GPU}\n"
            + "".join(f"DA:{n},{int(n in hit_rows)}\n" for n in sorted(rows))
            + "end_of_record\n",
        )
        with contextlib.redirect_stdout(io.StringIO()):
            checker.check_mule_coverage()
        original = target.read_text()
        for marker in markers:
            target.write_text(original.replace(f"DA:{marker},1\n", f"DA:{marker},0\n"))
            with self.subTest(marker=marker), self.assertRaisesRegex(SystemExit, "missing"):
                checker.check_mule_coverage()
        target.write_text(
            f"SF:{self.root / checker.MULE_GPU}\n"
            + "".join(f"DA:{n},{int(n in markers)}\n" for n in sorted(rows))
            + "end_of_record\n",
        )
        with contextlib.redirect_stdout(io.StringIO()), self.assertRaisesRegex(SystemExit, "below 80"):
            checker.check_mule_coverage()

    def test_mule_journey_accepts_distinct_hull_pixels_and_keeps_profile_absent(self):
        app = Mock()
        app.wait.return_value = 0
        receipt = {"phases": []}
        profile = self.root / "profile"
        before = bytes(smoke.WIDTH * smoke.HEIGHT * 3)
        after = bytearray(before)
        x0, y0, _, _ = smoke.MULE_REGION
        for y in range(y0, y0 + 50, 2):
            for x in range(x0, x0 + 50, 2):
                after[3 * (y * smoke.WIDTH + x)] = 80
        self.assertEqual(smoke.mule_changed_pixels(before, before), 0)
        header = b"P6\n1280 720\n255\n"
        for name, pixels in (("mule-opening", before), ("mule-approach", after)):
            (self.root / f"{name}.ppm").write_bytes(header + pixels)
        frames = [
            {"phase": name, "image": f"{name}.ppm", "title": smoke.MULE_TITLE}
            for name in ("mule-opening", "mule-approach")
        ]
        with (patch.object(smoke, "await_result", return_value="41"),
              patch.object(smoke, "focus"),
              patch.object(smoke, "run") as run,
              patch.object(smoke, "capture_phase", side_effect=frames) as capture,
              patch.object(smoke, "press") as press):
            smoke.exercise_mule(app, self.root, profile, receipt)
        self.assertFalse(capture.call_args_list[1].args[4](before))
        self.assertTrue(capture.call_args_list[1].args[4](after))
        self.assertEqual(receipt["changed_hull_region_samples"], 625)
        self.assertEqual(receipt["phases"], frames)
        run.assert_called_once_with("xdotool", "key", "--delay", "30", *(["period"] * smoke.MULE_STEPS))
        press.assert_called_once_with("Escape")
        self.assertFalse(profile.exists())

    def test_mule_opening_requires_hull_contrast_inside_the_expected_region(self):
        sky = bytes((10, 16, 32))
        image = bytearray(sky * (smoke.WIDTH * smoke.HEIGHT))
        self.assertFalse(smoke.mule_opening_frame(image))
        left, top, _, _ = smoke.MULE_REGION
        for y in range(top + 30, top + 50):
            start = 3 * (y * smoke.WIDTH + left + 40)
            image[start:start + 90] = bytes((54, 60, 62)) * 30
        self.assertTrue(smoke.mule_opening_frame(image))

    def test_mule_log_requires_two_presented_draws_and_a_working_shimmer(self):
        log = self.write("mule.log", "")
        adapter = "Meridian adapter: llvmpipe, device_type: Cpu, backend: Vulkan\n"
        record = ('MULE_PRESENTED_DRAW_WALL {"presentedFrames":2,"totalNanoseconds":35,'
                  '"minNanoseconds":10,"maxNanoseconds":25}\n')
        log.write_text(adapter + record)
        self.assertEqual(smoke.verify_mule_log(log)[1]["presentedFrames"], 2)
        for text in (adapter, adapter + record * 2, adapter + record.replace("35", "999"),
                     adapter + record + "Meridian Mule shimmer unavailable: disabled\n"):
            with self.subTest(text=text), self.assertRaises(RuntimeError):
                log.write_text(text)
                smoke.verify_mule_log(log)

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
