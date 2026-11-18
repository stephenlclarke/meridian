"""Offline fixture policy only. Never invokes AppKit, CoreGraphics or native subprocesses."""
import base64
import copy
import json
import io
from contextlib import redirect_stdout
from pathlib import Path
import struct
import tempfile
import unittest
import zlib
from unittest.mock import patch

import capture_meridian_window as capture


def png(width=3, height=2):
    def chunk(kind, payload):
        return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", zlib.crc32(kind + payload) & 0xffffffff)
    rows = b"".join(b"\0" + b"\x11\x22\x33\xff" * width for _ in range(height))
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b"")


def window():
    return {"pid": 12345, "bundle_id": capture.BUNDLE, "window_id": 91, "window_layer": 0,
            "on_screen": True, "active": True, "fully_on_one_display": True, "covers_display": False,
            "bounds": {"x": 10, "y": 20, "width": 3, "height": 2}, "title": "SYNTHETIC OFFLINE FIXTURE"}


def log_marker(request=1, frame=10):
    return (f'MERIDIAN_REVIEW_CAPTURE v2 stage=observation_frame_assigned data=Tag {{ request: {request}, frame: {frame}, review: ReviewId {{ token: 3, item: 17, operation: "Fit", facing: Some(Front) }} }}\n'
            f'MERIDIAN_REVIEW_CAPTURE v2 stage=observation_app_ack data=({frame}, (Stamp {{ session: 1, revision: 50 }}, 9), Some((Stamp {{ session: 1, revision: 50 }}, 9)), true, true)\n'
            f'MERIDIAN_REVIEW_CAPTURE v2 stage=pixels_map_complete data=(Tag {{ request: {request}, frame: {frame}, review: ReviewId {{ token: 3, item: 17, operation: "Fit", facing: Some(Front) }} }}, "callback Ok")\n').encode()


class PolicyFixtures(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="meridian-window-policy-fixture-")
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        launch = self.directory / "launch.json"
        launch.write_bytes(capture.json_bytes({"pid":12345,"source":capture.SOURCE,"binary":{"sha256":capture.BINARY_SHA},"environment":{"MERIDIAN_REVIEW_CAPTURE":"1"}}))
        self.log = self.directory / "native-stderr.log"
        self.log.write_bytes(b"".join(log_marker(i, i * 10) for i in range(1, 4)))
        self.scope = {"schema":1,"pid":12345,"bundle_id":capture.BUNDLE,"source":capture.SOURCE,
                      "bounds":window()["bounds"],"window_mode":"normal","max_captures":3,
                      "human_exception_reference":"human-approved: SYNTHETIC FIXTURE, NOT REAL AUTHORIZATION",
                      "evidence_dir":str(self.directory),"launch_receipt":str(launch),"native_stderr":str(self.log),"provider_sha256":"1"*64}
        self.scope_path = self.directory / "scope.json"
        self.scope_path.write_bytes(capture.json_bytes(self.scope))
        self.calls = []

    def provider(self, _path, mode, payload, _hash):
        # Explicit fake provider: no subprocess/UI API. Tests prove policy, not OS capture.
        self.calls.append(mode)
        if mode == "enumerate":
            return {"status":"ok","pid":12345,"bundle_id":capture.BUNDLE,"windows":[window()]}
        image = png()
        return {"status":"ok","pid":12345,"bundle_id":capture.BUNDLE,
                "chosen_before":window(),"chosen_after":window(),"capture_started_uptime_ns":100,"capture_ended_uptime_ns":110,
                "image_width":3,"image_height":2,"png_bytes":len(image),"png_sha256":capture.digest(image),"png_base64":base64.b64encode(image).decode()}

    def run_capture(self, request=1, frame=10, provider=None):
        return capture.capture(self.scope_path, Path("/not-run/offline-fixture-provider"), request, frame, provider or self.provider)

    def test_unique_owned_normal_window_is_selected_by_cg_id(self):
        self.assertEqual(capture.select_window([window()], self.scope)["window_id"], 91)
        other = window(); other["window_layer"] = 3
        self.assertEqual(capture.select_window([other, window()], self.scope)["window_id"], 91)

    def test_ambiguous_missing_foreign_hidden_minimized_or_fullscreen_reject(self):
        inactive_second = window()
        inactive_second["active"] = False
        cases = [[], [window(),window()], [window(),inactive_second]]
        for key, value in [("pid",12346),("bundle_id","other.app"),("on_screen",False),("active",False),("window_layer",1),("fully_on_one_display",False),("covers_display",True)]:
            candidate=window();candidate[key]=value;cases.append([candidate])
        for entries in cases:
            with self.subTest(entries=entries), self.assertRaises(capture.Rejected):
                capture.select_window(entries,self.scope)

    def test_bounds_mismatch_pointer_identifier_and_boolean_id_reject(self):
        for value in [0,2**32,0x600000123456,True]:
            candidate=window();candidate["window_id"]=value
            with self.assertRaises(capture.Rejected):capture.select_window([candidate],self.scope)
        candidate=window();candidate["bounds"]["x"]+=2
        with self.assertRaises(capture.Rejected):capture.select_window([candidate],self.scope)
        invalid=copy.deepcopy(self.scope);invalid["bounds"]["x"]=float("nan")
        with self.assertRaises(capture.Rejected):capture.select_window([window()],invalid)

    def test_pending_exception_stops_before_any_provider_or_subprocess(self):
        self.scope["human_exception_reference"]="PENDING"
        self.scope_path.write_bytes(capture.json_bytes(self.scope))
        with patch.object(capture.subprocess,"run",side_effect=AssertionError("native subprocess forbidden")):
            with self.assertRaises(capture.Rejected):self.run_capture()
        self.assertEqual(self.calls,[])

    def test_wrong_launch_pid_bundle_source_binary_or_capture_env_reject(self):
        for key,value in [("pid",54321),("source","0"*40),("bundle_id","other.app")]:
            bad=copy.deepcopy(self.scope);bad[key]=value
            with self.assertRaises(capture.Rejected):capture.validate_scope(bad)
        launch=Path(self.scope["launch_receipt"])
        for receipt in [{"pid":12345,"source":capture.SOURCE,"binary":{"sha256":"0"*64}}, {"pid":12345,"source":capture.SOURCE,"binary":{"sha256":capture.BINARY_SHA},"environment":{}}]:
            launch.write_bytes(capture.json_bytes(receipt))
            with self.assertRaises(capture.Rejected):capture.validate_scope(copy.deepcopy(self.scope))

    def test_stale_log_from_another_run_directory_is_rejected_before_provider(self):
        other = self.directory / "other-run"
        other.mkdir()
        wrong = other / "native-stderr.log"
        wrong.write_bytes(self.log.read_bytes())
        self.scope["native_stderr"] = str(wrong)
        self.scope_path.write_bytes(capture.json_bytes(self.scope))
        with self.assertRaises(capture.Rejected):
            self.run_capture()
        self.assertEqual(self.calls, [])

    def test_marker_requires_actual_unique_frame_map_and_app_ack(self):
        self.assertEqual(capture.marker(log_marker(),1,10)["frame"],10)
        for data,request,frame in [(log_marker(),1,11),(log_marker()+log_marker(),1,10),(log_marker().replace(b"callback Ok",b"callback failed"),1,10),(log_marker().replace(b"true, true",b"false, true"),1,10),(log_marker(),4,10)]:
            with self.assertRaises(capture.Rejected):capture.marker(data,request,frame)

    def test_three_successful_fixtures_are_bounded_and_create_new(self):
        for n in range(1,4):
            result=self.run_capture(n,n*10)
            self.assertEqual(result["status"],"captured")
            self.assertEqual(Path(result["artifact"]["path"]).read_bytes(),png())
            self.assertGreaterEqual(result["python_monotonic_end_ns"],result["python_monotonic_start_ns"])
            self.assertEqual(result["native_drawable_presented"],"UNOBSERVABLE")
        self.assertEqual(self.calls,["enumerate","capture"]*3)
        with self.assertRaises((capture.Rejected,FileExistsError)):self.run_capture(3,30)
        with self.assertRaises(capture.Rejected):self.run_capture(4,40)
        self.assertEqual(len(list(self.directory.glob("*.png"))),3)

    def test_native_identity_drift_is_failure_without_image_and_blocks_later_capture(self):
        def drift(path,mode,payload,sha):
            reply=self.provider(path,mode,payload,sha)
            if mode=="capture":reply["chosen_after"]["window_id"]=92
            return reply
        result=self.run_capture(provider=drift)
        self.assertEqual(result["status"],"failed")
        self.assertEqual(len(list(self.directory.glob("*.png"))),0)
        before=len(self.calls)
        with self.assertRaises(capture.Rejected):self.run_capture(2,20)
        self.assertEqual(len(self.calls),before)

    def test_native_failure_and_timeout_do_not_fallback_or_retry(self):
        for error in [capture.Rejected("permission unavailable"),capture.subprocess.TimeoutExpired("fixture",8)]:
            with self.subTest(error=error), tempfile.TemporaryDirectory() as directory:
                self.scope["evidence_dir"]=directory;self.scope_path.write_bytes(capture.json_bytes(self.scope))
                attempted=[]
                def fail(*_args):attempted.append(1);raise error
                result=self.run_capture(provider=fail)
                self.assertEqual(result["status"],"failed");self.assertEqual(attempted,[1])
                self.assertFalse(list(Path(directory).glob("*.png")))

    def test_corrupt_png_hash_dimensions_and_preexisting_output_never_publish(self):
        for kind in ["hash","dimensions","crc"]:
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as directory:
                self.scope["evidence_dir"]=directory;self.scope_path.write_bytes(capture.json_bytes(self.scope))
                def bad(path,mode,payload,sha):
                    reply=self.provider(path,mode,payload,sha)
                    if mode=="capture":
                        if kind=="hash":reply["png_sha256"]="0"*64
                        elif kind=="dimensions":reply["image_width"]=4
                        else:
                            raw=bytearray(png());raw[-1]^=1;reply["png_base64"]=base64.b64encode(raw).decode()
                    return reply
                self.assertEqual(self.run_capture(provider=bad)["status"],"failed")
                self.assertFalse(list(Path(directory).glob("*.png")))
        self.scope["evidence_dir"]=str(self.directory);self.scope_path.write_bytes(capture.json_bytes(self.scope))
        existing=self.directory/"window-capture-1-frame-10.png";existing.write_bytes(b"retain")
        before=len(self.calls)
        with self.assertRaises(capture.Rejected):self.run_capture()
        self.assertEqual(len(self.calls),before);self.assertEqual(existing.read_bytes(),b"retain")

    def test_native_negative_status_cannot_publish_even_with_valid_identity(self):
        for failed_mode in ["enumerate", "capture"]:
            with self.subTest(mode=failed_mode), tempfile.TemporaryDirectory() as directory:
                self.scope["evidence_dir"] = directory
                self.scope_path.write_bytes(capture.json_bytes(self.scope))
                def negative(path, mode, payload, sha):
                    reply = self.provider(path, mode, payload, sha)
                    if mode == failed_mode:
                        reply["status"] = "rejected"
                    return reply
                self.assertEqual(self.run_capture(provider=negative)["status"], "failed")
                self.assertFalse(list(Path(directory).glob("*.png")))

    def run_cli(self, provider=None):
        original = capture.capture
        def offline(*args, **kwargs):
            return original(*args, call=provider or self.provider, **kwargs)
        argv = ["capture_meridian_window.py", "--scope", str(self.scope_path),
                "--provider", "/not-run/offline-fixture-provider", "--request", "1", "--frame", "10"]
        output = io.StringIO()
        with patch.object(capture, "capture", side_effect=offline), patch.object(capture.sys, "argv", argv), redirect_stdout(output):
            status = capture.main()
        return status, json.loads(output.getvalue())

    def test_cli_final_receipt_fsync_and_cleanup_failures_retain_capture_and_stop(self):
        original_create, original_unlink = capture.create_new, Path.unlink
        for failure in ["receipt_fsync", "cleanup"]:
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                self.scope["evidence_dir"] = directory
                self.scope_path.write_bytes(capture.json_bytes(self.scope))
                def create(path, data):
                    if failure == "receipt_fsync" and path.name == "window-capture-1.json":
                        with patch.object(capture.os, "fsync", side_effect=OSError("injected receipt fsync")):
                            return original_create(path, data)
                    return original_create(path, data)
                def unlink(path, *args, **kwargs):
                    if failure == "cleanup" and path.name == "window-capture.inflight":
                        raise OSError("injected lock cleanup")
                    return original_unlink(path, *args, **kwargs)
                self.calls.clear()
                with patch.object(capture, "create_new", side_effect=create), patch.object(Path, "unlink", unlink):
                    status, result = self.run_cli()
                self.assertEqual(status, 2)
                self.assertEqual(result["status"], "capture_attempted_outcome_unknown")
                self.assertTrue(result["native_provider_attempted"])
                self.assertTrue(result["capture_attempted"])
                root = Path(directory)
                self.assertEqual((root / "window-capture-1-frame-10.png").read_bytes(), png())
                self.assertTrue((root / "window-capture-1.claim.json").exists())
                self.assertTrue((root / "window-capture-1.json").exists())
                self.assertTrue((root / "window-capture.inflight").exists())
                with self.assertRaises(FileExistsError):
                    self.run_capture()
                self.assertEqual(self.calls, ["enumerate", "capture"])
                next_slot = self.run_capture(2, 20)
                self.assertEqual(next_slot["status"], "failed")
                self.assertFalse(next_slot["native_provider_attempted"])
                self.assertFalse(next_slot["capture_attempted"])
                self.assertEqual(self.calls, ["enumerate", "capture"])
                self.assertTrue((root / "window-capture.inflight").exists())
                self.assertEqual((root / "window-capture-1-frame-10.png").read_bytes(), png())
                self.assertTrue((root / "window-capture-1.claim.json").exists())
                self.assertTrue((root / "window-capture-1.json").exists())

    def test_cli_preflight_failure_truthfully_records_no_native_attempt(self):
        self.scope["human_exception_reference"] = "PENDING"
        self.scope_path.write_bytes(capture.json_bytes(self.scope))
        status, result = self.run_cli()
        self.assertEqual(status, 2)
        self.assertEqual(result["status"], "rejected_before_capture")
        self.assertFalse(result["native_provider_attempted"])
        self.assertFalse(result["capture_attempted"])
        self.assertEqual(self.calls, [])

    def test_cli_enumeration_then_receipt_failure_does_not_claim_no_native_work(self):
        original_create = capture.create_new
        def create(path, data):
            if path.name == "window-capture-1.json":
                raise OSError("injected receipt failure")
            return original_create(path, data)
        def fail(*args):
            self.calls.append("enumerate")
            raise capture.Rejected("injected enumeration rejection")
        with patch.object(capture, "create_new", side_effect=create):
            status, result = self.run_cli(provider=fail)
        self.assertEqual(status, 2)
        self.assertEqual(result["status"], "native_provider_attempted_outcome_unknown")
        self.assertTrue(result["native_provider_attempted"])
        self.assertFalse(result["capture_attempted"])
        self.assertEqual(self.calls, ["enumerate"])
        self.assertTrue((self.directory / "window-capture-1.claim.json").exists())
        with self.assertRaises(FileExistsError):
            self.run_capture()
        self.assertEqual(self.calls, ["enumerate"])

    def test_native_source_has_single_window_filter_and_no_input_or_permission_request(self):
        source=Path(__file__).with_name("meridian_window_capture.swift").read_text()
        self.assertIn("SCContentFilter(desktopIndependentWindow: window)",source)
        self.assertIn("CGPreflightScreenCaptureAccess()",source)
        self.assertIn("chosen.1.window_id",source)
        for forbidden in ["CGRequestScreenCaptureAccess", "SCContentSharingPicker", "SCContentFilter(display:", "captureImage(in:", "CGEvent(", "NSAppleScript", "activate(options:", "setFrame("]:
            self.assertNotIn(forbidden,source)

if __name__=="__main__":
    unittest.main()
