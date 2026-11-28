"""Exercise the X11 coverage keeper's subprocess boundaries without a display server."""

import importlib
import os
from pathlib import Path
import signal
import sys
import tempfile
import unittest
from unittest.mock import Mock, call, patch


SCRIPTS = Path(__file__).resolve().parents[1] / ".github/scripts"
with patch.object(sys, "path", [str(SCRIPTS), *sys.path]):
    session = importlib.import_module("native_x11_session")


class NativeX11EntrypointTests(unittest.TestCase):
    def test_entrypoint_launches_only_the_fixed_coverage_command(self):
        with (patch.object(sys, "argv", [str(SCRIPTS / "native_x11_session.py")]),
              patch.object(session, "run_session", return_value=7) as launch):
            self.assertEqual(session.main(), 7)
        launch.assert_called_once_with(
            [sys.executable, "-B", "-m", "coverage", "run", "--parallel-mode",
             str(SCRIPTS / "native_coverage_smoke.py")],
            SCRIPTS.parents[1] / "coverage/x11-client.log",
        )

    def test_entrypoint_rejects_arguments_before_launch(self):
        with (patch.object(sys, "argv", [str(SCRIPTS / "native_x11_session.py"), "unexpected"]),
              patch.object(session, "run_session") as launch):
            with self.assertRaisesRegex(SystemExit, "does not accept arguments"):
                session.main()
        launch.assert_not_called()


class NativeX11GeometryTests(unittest.TestCase):
    def test_fragmented_geometry_uses_one_deadline_and_bounded_reads(self):
        stream = Mock()
        stream.read.side_effect = [b"1600 ", b"900", b"\n"]
        with (patch.object(session.time, "monotonic", side_effect=[100, 101, 103, 104]),
              patch.object(session.select, "select", return_value=([stream], [], [])) as ready):
            self.assertEqual(session.read_geometry(stream), b"1600 900\n")
        self.assertEqual(ready.call_args_list, [call([stream], [], [], n) for n in (9, 7, 6)])
        self.assertEqual(stream.read.call_args_list,
                         [call(session.MAX_GEOMETRY_BYTES - n) for n in (0, 5, 8)])

    def test_partial_geometry_without_newline_times_out_on_original_deadline(self):
        stream = Mock()
        stream.read.return_value = b"1600 900"
        with (patch.object(session.time, "monotonic", side_effect=[100, 101, 105]),
              patch.object(session.select, "select", side_effect=[([stream], [], []), ([], [], [])]) as ready):
            with self.assertRaisesRegex(RuntimeError, "Timed out waiting"):
                session.read_geometry(stream)
        self.assertEqual(ready.call_args_list, [call([stream], [], [], n) for n in (9, 5)])
        stream.read.assert_called_once_with(session.MAX_GEOMETRY_BYTES)

    def test_geometry_without_newline_cannot_exceed_record_size_limit(self):
        stream = Mock()
        stream.read.return_value = b"1" * session.MAX_GEOMETRY_BYTES
        with patch.object(session.select, "select", return_value=([stream], [], [])) as ready:
            with self.assertRaisesRegex(RuntimeError, "exceeded its size limit"):
                session.read_geometry(stream)
        self.assertEqual(ready.call_count, 1)
        stream.read.assert_called_once_with(session.MAX_GEOMETRY_BYTES)


@unittest.skipUnless(os.name == "posix", "X11 keeper uses POSIX pipes, signals and process reaping")
class NativeX11SessionTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.log = self.root / "client.log"
        self.pid = self.root / "client.pid"
        self.launched = self.root / "launched"

    def client(self, action):
        return (sys.executable, "-u", "-c", (
            "import os, sys; from pathlib import Path; "
            "Path(sys.argv[1]).write_text(str(os.getpid())); "
            "assert sys.stdin.readline() == 'getdisplaygeometry\\n'; "
            "print('fixture client', file=sys.stderr); " + action
        ), str(self.pid))

    def command(self, action=""):
        return [sys.executable, "-c", (
            "import os, signal, sys; from pathlib import Path; "
            "pid = int(Path(sys.argv[1]).read_text()); os.kill(pid, 0); "
            "path = Path(sys.argv[2]); path.write_text((path.read_text() if path.exists() else '') + 'launch\\n'); "
            + action
        ), str(self.pid), str(self.launched)]

    def assert_client_stopped(self):
        with self.assertRaises(ProcessLookupError):
            os.kill(int(self.pid.read_text()), 0)

    def test_ready_client_stays_live_for_command_and_is_reaped(self):
        with patch.object(session, "X11_CLIENT", self.client("print('1600 900'); sys.stdin.read()")):
            self.assertEqual(session.run_session(self.command(), self.log), 0)
        self.assertTrue(self.launched.exists())
        self.assert_client_stopped()
        self.assertIn("fixture client\nAuthenticated X11 display geometry: 1600 900\n", self.log.read_text())
        self.assertIn("Native coverage exit status: 0", self.log.read_text())

    def test_command_failure_is_propagated_without_retry_and_keeper_reaped(self):
        with patch.object(session, "X11_CLIENT", self.client("print('1600 900'); sys.stdin.read()")):
            action = "sys.exit(7)"
            self.assertEqual(session.run_session(self.command(action), self.log), 7)
        self.assertEqual(self.launched.read_text(), "launch\n")
        self.assert_client_stopped()
        self.assertIn("Native coverage exit status: 7", self.log.read_text())

    def test_signalled_command_cannot_appear_successful(self):
        with patch.object(session, "X11_CLIENT", self.client("print('1600 900'); sys.stdin.read()")):
            action = "os.kill(os.getpid(), signal.SIGTERM)"
            self.assertEqual(session.run_session(self.command(action), self.log), 128 + signal.SIGTERM)
        self.assert_client_stopped()

    def test_unready_client_does_not_launch_command_and_retains_diagnostics(self):
        with patch.object(session, "X11_CLIENT", self.client("sys.exit(1)")):
            with self.assertRaisesRegex(RuntimeError, "did not establish a live display"):
                session.run_session(self.command(), self.log)
        self.assertFalse(self.launched.exists())
        self.assert_client_stopped()
        self.assertIn("fixture client", self.log.read_text())
        self.assertIn("did not establish a live display", self.log.read_text())

    def test_bad_geometry_does_not_launch_command(self):
        with patch.object(session, "X11_CLIENT", self.client("print('0 900'); sys.stdin.read()")):
            with self.assertRaisesRegex(RuntimeError, "did not establish a live display"):
                session.run_session(self.command(), self.log)
        self.assertFalse(self.launched.exists())
        self.assert_client_stopped()

    def test_partial_geometry_at_eof_does_not_launch_and_reaps_keeper(self):
        with patch.object(session, "X11_CLIENT", self.client("sys.stdout.write('1600 900'); sys.exit(0)")):
            with self.assertRaisesRegex(RuntimeError, "did not establish a live display"):
                session.run_session(self.command(), self.log)
        self.assertFalse(self.launched.exists())
        self.assert_client_stopped()
        self.assertIn("1600 900", self.log.read_text())

    def test_readiness_timeout_does_not_launch_command(self):
        with (patch.object(session, "X11_CLIENT", self.client("sys.stdin.read()")),
              patch.object(session.select, "select", return_value=([], [], []))):
            with self.assertRaisesRegex(RuntimeError, "Timed out waiting"):
                session.run_session(self.command(), self.log)
        self.assertFalse(self.launched.exists())
        self.assertIn("Timed out waiting", self.log.read_text())

    def test_keeper_exit_during_successful_command_is_not_accepted(self):
        with patch.object(session, "X11_CLIENT", self.client("print('1600 900'); sys.stdin.read()")):
            action = "os.kill(pid, signal.SIGTERM); import time; time.sleep(0.1)"
            with self.assertRaisesRegex(RuntimeError, "exited during native coverage"):
                session.run_session(self.command(action), self.log)
        self.assert_client_stopped()


if __name__ == "__main__":
    unittest.main()
