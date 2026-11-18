#!/usr/bin/env python3
"""Pending human exception: bounded read-only ScreenCaptureKit window observer.

Import/policy fixtures use only Python stdlib. Native provider is never run unless
an explicit exception reference and exact reviewed scope/launch receipt validate.
The reference is evidence correlation, not a programmatic grant of permission.
"""
from __future__ import annotations
import argparse
import base64
import hashlib
import json
import math
import os
from pathlib import Path
import re
import struct
import subprocess
import sys
import time
import zlib

BUNDLE = "io.xyzzy.meridian.review-v2.ad5b9f9"
SOURCE = "ad5b9f9da259c85389cd662af2473e543e26ab11"
BINARY_SHA = "9e4a2f1e40431a6e480c886088e58256655bde8b6aa04862a184ab893d25ccdd"
MAX_IMAGE = 16 * 1024 * 1024
MAX_LOG = 8 * 1024 * 1024
MAX_REPLY = 24 * 1024 * 1024

class Rejected(ValueError):
    """An explicit unsupported/mismatched state; never a fallback request."""

def require(condition: bool, reason: str) -> None:
    if not condition:
        raise Rejected(reason)

def integer(value: object, low: int, high: int) -> bool:
    return type(value) is int and low <= value <= high

def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()

def read_limited(path: Path, limit: int) -> bytes:
    with path.open("rb") as stream:
        data = stream.read(limit + 1)
    require(len(data) <= limit, "input exceeds finite byte limit")
    return data

def bounds(value: object) -> dict[str, float]:
    require(isinstance(value, dict) and set(value) == {"x", "y", "width", "height"}, "exact point bounds required")
    require(all(type(v) in (int, float) and math.isfinite(v) for v in value.values()), "nonfinite/invalid bounds")
    require(0 < value["width"] <= 4096 and 0 < value["height"] <= 4096, "zero/oversize bounds")
    return {k: float(v) for k, v in value.items()}

def same_bounds(first: object, second: object) -> bool:
    a, b = bounds(first), bounds(second)
    return all(abs(a[key] - b[key]) <= 0.5 for key in a)

def select_window(windows: object, scope: dict) -> dict:
    require(isinstance(windows, list) and len(windows) <= 16, "invalid/excess owned window metadata")
    for window in windows:
        require(isinstance(window, dict), "invalid window metadata")
        require(window.get("pid") == scope["pid"] and window.get("bundle_id") == BUNDLE, "window owner mismatch")
        require(integer(window.get("window_id"), 1, 2**32 - 1), "invalid CGWindowID; pointer IDs forbidden")
    candidates = [w for w in windows if w.get("on_screen") is True and w.get("active") is True and type(w.get("window_layer")) is int and w["window_layer"] == 0]
    require(len(candidates) == 1, "missing or ambiguous visible normal target window")
    chosen = candidates[0]
    require(same_bounds(chosen.get("bounds"), scope["bounds"]), "reviewed window bounds mismatch")
    require(chosen.get("fully_on_one_display") is True and chosen.get("covers_display") is False, "offscreen/fullscreen window rejected")
    return chosen

def validate_scope(scope: dict) -> tuple[dict, bytes]:
    require(isinstance(scope, dict) and scope.get("schema") == 1, "unsupported scope schema")
    require(scope.get("bundle_id") == BUNDLE and scope.get("source") == SOURCE, "only the exact Meridian test build is allowed")
    require(integer(scope.get("pid"), 2, 2**31 - 1), "exact live owning PID required")
    scope["bounds"] = bounds(scope.get("bounds"))
    reference = scope.get("human_exception_reference")
    require(isinstance(reference, str) and reference.startswith("human-approved:") and 20 <= len(reference) <= 2048 and "PENDING" not in reference.upper(), "explicit genuine human exception is still required")
    require(scope.get("window_mode") == "normal" and scope.get("max_captures") == 3, "normal single-window/max3 scope required")
    directory = Path(scope["evidence_dir"])
    require(directory.is_absolute() and directory.is_dir(), "existing absolute evidence directory required")
    require(isinstance(scope.get("provider_sha256"), str) and re.fullmatch(r"[0-9a-f]{64}", scope["provider_sha256"]) is not None, "reviewed native provider hash required")
    launch = read_limited(Path(scope["launch_receipt"]), 1024 * 1024)
    receipt = json.loads(launch)
    require(receipt.get("pid") == scope["pid"] and receipt.get("source") == SOURCE, "owning launch receipt mismatch")
    require(receipt.get("binary", {}).get("sha256") == BINARY_SHA, "qualified signed executable identity mismatch")
    require(receipt.get("environment", {}).get("MERIDIAN_REVIEW_CAPTURE") == "1", "existing F8 capture must be enabled")
    return scope, launch

def marker(log: bytes, request: int, frame: int) -> dict:
    require(integer(request, 1, 3) and integer(frame, 1, 2**64 - 1), "request/frame must be finite IDs")
    text = log.decode("utf-8")
    pattern = re.compile(r"^MERIDIAN_REVIEW_CAPTURE v2 stage=observation_frame_assigned data=Tag \{ request: (\d+), frame: (\d+), review: ReviewId \{ token: (\d+), item: (\d+), operation: \"(Fit|Store)\", facing: (.*?) \} \}$", re.M)
    found = [m for m in pattern.finditer(text) if int(m[1]) == request]
    require(len(found) == 1 and int(found[0][2]) == frame, "missing/ambiguous/mismatched actual F8 marker")
    chosen = found[0]
    require(re.search(r"^MERIDIAN_REVIEW_CAPTURE v2 stage=observation_app_ack data=\(" + str(frame) + r", .*true, true\)$", text, re.M) is not None, "actual matching App acknowledgement required")
    require(re.search(r"^MERIDIAN_REVIEW_CAPTURE v2 stage=pixels_map_complete data=\(Tag \{ request: " + str(request) + r", frame: " + str(frame) + r",.*callback Ok", text, re.M) is not None, "named actual GPU copy readiness required; no wait/fallback")
    return {"request": request, "frame": frame, "review_token": int(chosen[3]), "item": int(chosen[4]), "operation": chosen[5], "log_snapshot_sha256": digest(log), "association": "monotonic bracket and F8 context only; not atomic drawable/display identity"}

def png_size(data: bytes) -> tuple[int, int]:
    require(33 <= len(data) <= MAX_IMAGE and data[:8] == b"\x89PNG\r\n\x1a\n", "bounded PNG required")
    offset, result = 8, None
    while offset < len(data):
        require(offset + 12 <= len(data), "truncated PNG")
        length = int.from_bytes(data[offset:offset+4], "big")
        end = offset + 12 + length
        require(end <= len(data), "truncated PNG chunk")
        kind, payload = data[offset+4:offset+8], data[offset+8:offset+8+length]
        require(zlib.crc32(kind + payload) & 0xffffffff == int.from_bytes(data[end-4:end], "big"), "PNG chunk CRC mismatch")
        if offset == 8:
            require(kind == b"IHDR" and length == 13, "PNG IHDR required")
            width, height = struct.unpack(">II", payload[:8])
            require(width > 0 and height > 0 and width * height * 4 <= MAX_IMAGE, "PNG dimensions exceed scope")
            result = width, height
        if kind == b"IEND":
            require(length == 0 and end == len(data) and result is not None, "invalid PNG end")
            return result
        offset = end
    raise Rejected("missing PNG end")

def create_new(path: Path, data: bytes) -> None:
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0)
    descriptor = os.open(path, flags, 0o600)
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())

def json_bytes(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n").encode()

def provider_call(provider: Path, mode: str, payload: dict, expected_hash: str) -> dict:
    require(sys.platform == "darwin", "native provider is macOS only")
    require(provider.is_absolute() and digest(read_limited(provider, 32 * 1024 * 1024)) == expected_hash, "native provider executable hash mismatch")
    result = subprocess.run([str(provider), mode], input=json_bytes(payload), stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=8, check=False)
    require(len(result.stdout) <= MAX_REPLY and len(result.stderr) <= 65536, "native response exceeds bound")
    reply = json.loads(result.stdout)
    require(result.returncode == 0 and reply.get("status") == "ok", "native observer rejected/failed: " + str(reply.get("reason", "no success"))[:512])
    return reply

def capture(scope_path: Path, provider: Path, request: int, frame: int, call=provider_call) -> dict:
    raw_scope = read_limited(scope_path, 65536)
    scope, launch = validate_scope(json.loads(raw_scope))
    log = read_limited(Path(scope["native_stderr"]), MAX_LOG)
    observation = marker(log, request, frame)
    directory, scope_hash = Path(scope["evidence_dir"]), digest(raw_scope)
    for previous in range(1, request):
        receipt = json.loads(read_limited(directory / f"window-capture-{previous}.json", 65536))
        require(receipt.get("status") == "captured" and receipt.get("scope_sha256") == scope_hash and receipt["marker"]["frame"] < frame, "prior failure/incomplete/mismatched session; stop without retry")
    binding = directory / "window-capture-scope.sha256"
    try:
        create_new(binding, scope_hash.encode())
    except FileExistsError:
        require(read_limited(binding, 64).decode() == scope_hash, "session scope changed")
    started = time.monotonic_ns()
    claim = {"scope_sha256": scope_hash, "marker": observation, "pid": scope["pid"], "bundle_id": BUNDLE, "python_monotonic_start_ns": started, "human_exception_reference": scope["human_exception_reference"]}
    create_new(directory / f"window-capture-{request}.claim.json", json_bytes(claim))
    require(not (directory / f"window-capture-{request}.json").exists() and not (directory / f"window-capture-{request}-frame-{frame}.png").exists(), "evidence destination already exists; no capture attempted")
    lock = directory / "window-capture.inflight"
    acquired = False
    receipt = dict(claim, status="failed", launch_receipt_sha256=digest(launch), native_drawable_presented="UNOBSERVABLE", other_app_or_display_images="NEVER_REQUESTED")
    try:
        create_new(lock, str(request).encode()); acquired = True
        native_scope = {key: scope[key] for key in ["pid", "bundle_id", "bounds", "human_exception_reference"]}
        enumeration = call(provider, "enumerate", {"scope": native_scope}, scope["provider_sha256"])
        require(enumeration.get("status") == "ok" and enumeration.get("pid") == scope["pid"] and enumeration.get("bundle_id") == BUNDLE, "native enumeration identity mismatch")
        selected = select_window(enumeration.get("windows"), scope)
        reply = call(provider, "capture", {"scope": native_scope, "selected": selected}, scope["provider_sha256"])
        require(reply.get("status") == "ok" and reply.get("pid") == scope["pid"] and reply.get("bundle_id") == BUNDLE, "native capture identity mismatch")
        before, after = select_window([reply.get("chosen_before")], scope), select_window([reply.get("chosen_after")], scope)
        require(before["window_id"] == after["window_id"] == selected["window_id"], "native window changed during capture")
        require(integer(reply.get("capture_started_uptime_ns"), 1, 2**64-1) and integer(reply.get("capture_ended_uptime_ns"), reply["capture_started_uptime_ns"], 2**64-1), "native monotonic bracket required")
        image = base64.b64decode(reply["png_base64"], validate=True)
        width, height = png_size(image)
        require((width, height) == (reply.get("image_width"), reply.get("image_height")) and len(image) == reply.get("png_bytes") and digest(image) == reply.get("png_sha256"), "capture byte/dimension/hash mismatch")
        image_path = directory / f"window-capture-{request}-frame-{frame}.png"
        create_new(image_path, image)
        receipt.update(status="captured", selected_window=selected, enumeration=enumeration, capture={k:v for k,v in reply.items() if k != "png_base64"}, artifact={"path":str(image_path),"bytes":len(image),"sha256":digest(image)})
    except Exception as error:
        receipt["reason"] = str(error)[:1024]
    finally:
        receipt["python_monotonic_end_ns"] = time.monotonic_ns()
        try:
            create_new(directory / f"window-capture-{request}.json", json_bytes(receipt))
        finally:
            if acquired:
                lock.unlink()  # only this invocation's own transient lock; claims/evidence persist
    return receipt

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scope", type=Path, required=True)
    parser.add_argument("--provider", type=Path, required=True)
    parser.add_argument("--request", type=int, required=True)
    parser.add_argument("--frame", type=int, required=True)
    args = parser.parse_args()
    try:
        result = capture(args.scope, args.provider, args.request, args.frame)
        print(json.dumps({"status":result["status"],"request":args.request,"frame":args.frame,"reason":result.get("reason")}, sort_keys=True))
        return 0 if result["status"] == "captured" else 2
    except Exception as error:
        print(json.dumps({"status":"rejected_before_capture","reason":str(error)[:1024]}, sort_keys=True))
        return 2
if __name__ == "__main__":
    raise SystemExit(main())
