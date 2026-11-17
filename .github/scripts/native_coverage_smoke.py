"""Exercise the instrumented desktop through X11 and inspect presented pixels."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time


WIDTH = 1280
HEIGHT = 720
RECTANGLE = (WIDTH, HEIGHT)
SAVE_NAME = "commander.meridian"
MAX_SAVE_BYTES = 65_536


def run(*argv, env=None):
    return subprocess.check_output(argv, env=env, timeout=10)


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def await_result(app, description, check):
    deadline = time.monotonic() + 40
    while time.monotonic() < deadline:
        if app.poll() is not None:
            raise RuntimeError(f"Desktop exited {app.returncode} before {description}")
        result = check()
        if result:
            return result
        time.sleep(0.15)
    raise RuntimeError(f"Desktop timed out waiting for {description}")


def mapped_window(app):
    result = subprocess.run(
        ["xdotool", "search", "--onlyvisible", "--pid", str(app.pid)],
        capture_output=True, text=True, timeout=10, check=False,
    )
    ids = result.stdout.split()
    if result.returncode == 1 and not ids and not result.stderr:
        return None
    result.check_returncode()
    if len(ids) != 1 or not ids[0].isdecimal():
        raise RuntimeError(f"Expected one visible window for PID {app.pid}, found {ids}")
    return ids[0]


def focus(window):
    run("xdotool", "windowfocus", "--sync", window)
    if run("xdotool", "getwindowfocus").strip() != window.encode():
        raise RuntimeError("X11 focus did not reach the desktop window")


def count_pixels(rgb, bounds, predicate, step=2):
    left, top, right, bottom = bounds
    seen = 0
    for y in range(top, bottom, step):
        row = y * WIDTH
        for x in range(left, right, step):
            offset = 3 * (row + x)
            seen += predicate(*rgb[offset:offset + 3])
    return seen


def bright(r, g, b):
    return min(r, g, b) > 105


def green(r, g, b):
    return g > 165 and max(r, b) < 90


def title_frame(rgb):
    return (
        count_pixels(rgb, (450, 220, 830, 465), bright, step=4) > 100
        and count_pixels(rgb, (0, 650, 450, 720), bright) > 40
    )


def system_frame(rgb):
    # The System page renders a planet in its right pane and text in its left.
    return (
        count_pixels(rgb, (16, 104, 306, 328), bright) > 40
        and count_pixels(rgb, (490, 180, 1110, 476), lambda r, g, b: max(r, g, b) > 25, step=4) > 500
    )


def trading_frame(rgb):
    # The Market renders white price cells down the column starting at x=490.
    bands = [
        count_pixels(rgb, (490, y, 645, y + 13), bright)
        for y in range(124, 324, 20)
    ]
    return sum(hits > 5 for hits in bands) >= 8


def cockpit_frame(rgb):
    # The green scanner occupies the lower centre in the cockpit HUD.
    return count_pixels(rgb, (445, 620, 835, 720), green) > 40


def capture_phase(app, window, output, name, predicate):
    last_frame = None
    header = f"P6\n{WIDTH} {HEIGHT}\n255\n".encode()

    def presented_frame():
        nonlocal last_frame
        rgb = run("import", "-window", window, "-depth", "8", "rgb:-")
        if len(rgb) != WIDTH * HEIGHT * 3:
            raise RuntimeError("X11 window capture has unexpected dimensions")
        last_frame = rgb
        return rgb if predicate(rgb) else None

    try:
        rgb = await_result(app, name, presented_frame)
    except (RuntimeError, subprocess.SubprocessError):
        if last_frame is not None:
            (output / f"{name}-last.ppm").write_bytes(header + last_frame)
        raise
    image = output / f"{name}.ppm"
    image.write_bytes(header + rgb)
    title = run("xdotool", "getwindowname", window).decode().strip()
    return {"phase": name, "image": image.name, "sha256": sha256(image), "title": title}


def press(key):
    run("xdotool", "key", "--delay", "120", key)


def exercise(app, output, save, receipt):
    receipt["stage"] = "map and focus"
    window = await_result(app, "visible X11 window", lambda: mapped_window(app))
    receipt["x11_window"] = window
    focus(window)
    phases = receipt["phases"]
    receipt["stage"] = "title"
    phases.append(capture_phase(app, window, output, "title", title_frame))
    if (save / SAVE_NAME).exists():
        raise RuntimeError("Fresh title unexpectedly created a save")
    receipt["stage"] = "system"
    press("n")
    phases.append(capture_phase(app, window, output, "system", system_frame))
    if "New commander ready" not in phases[-1]["title"]:
        raise RuntimeError("System frame did not follow ordinary commander creation")
    receipt["stage"] = "trading"
    press("4")
    phases.append(capture_phase(app, window, output, "trading", trading_frame))
    if phases[-1]["sha256"] == phases[-2]["sha256"]:
        raise RuntimeError("Market did not present a distinct frame")
    receipt["stage"] = "hud"
    press("Escape")
    phases.append(capture_phase(app, window, output, "hud", cockpit_frame))
    receipt["stage"] = "normal quit"
    press("Escape")
    receipt["normal_quit_sent"] = True
    code = app.wait(timeout=20)
    if code:
        raise RuntimeError(f"Desktop normal quit exited {code}")


def verify_log(log):
    text = log.read_text()
    adapter = next((line for line in text.splitlines() if line.startswith("Meridian adapter: ")), "")
    if not all(value in adapter for value in ("llvmpipe", "device_type: Cpu", "backend: Vulkan")):
        raise RuntimeError(f"Expected Mesa llvmpipe software Vulkan, got {adapter!r}")
    if any(re.search(pattern, text, re.IGNORECASE) for pattern in (
        r"Meridian(?: GPU error| graphics device lost|:)", r"validation error",
        r"thread .* panicked", r"LLVM Profile Error",
    )):
        raise RuntimeError("Desktop log reports a rendering or profile error")
    return adapter


def verify_save(observer, directory, output, receipt):
    save = directory / SAVE_NAME
    if sorted(p.name for p in directory.iterdir()) != [SAVE_NAME, "session.lock"]:
        raise RuntimeError("Normal quit did not produce exactly one isolated commander save")
    before = sha256(save)
    env = dict(os.environ, LLVM_PROFILE_FILE=str(directory.parent / "observer.profraw"))
    stdout = output / "commander-observed.txt"
    stderr = output / "observer.stderr"
    attempt = {"argv": [str(observer), str(save)], "exit_code": None, "status": "starting",
               "stdout": stdout.name, "stderr": stderr.name}
    receipt["observer"] = attempt
    with stdout.open("xb") as out, stderr.open("xb") as err:
        try:
            process = subprocess.Popen(attempt["argv"], env=env, stdout=out, stderr=err)
        except OSError:
            attempt["status"] = "launch failed"
            raise
        attempt["pid"] = process.pid
        try:
            code = process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            attempt.update(exit_code=process.wait(timeout=10), status="timed out and reaped")
            raise
        attempt.update(exit_code=code, status="exited")
        if code:
            raise subprocess.CalledProcessError(code, attempt["argv"])
    evidence = stdout.read_text()
    required = ("saved_format=15\n", "mode: Docked", "credits_tenths=Some(7200)\n",
                "used_tonnes: 0", "fuel_units: 130")
    if not all(item in evidence for item in required) or before != sha256(save):
        raise RuntimeError("Save decoder did not confirm the untouched starter commander")
    return before


def retain_failed_save(directory, output):
    save = directory / SAVE_NAME
    entries = sorted(p.name for p in directory.iterdir())
    if save.is_symlink() or not save.is_file():
        return {"slot_entries": entries, "regular_commander": False}
    with save.open("rb") as stream:
        prefix = stream.read(MAX_SAVE_BYTES + 1)
    truncated = len(prefix) > MAX_SAVE_BYTES
    retained = prefix[:MAX_SAVE_BYTES]
    name = "commander.failed.prefix" if truncated else "commander.failed.meridian"
    (output / name).write_bytes(retained)
    return {"slot_entries": entries, "regular_commander": True, "artifact": name,
            "file_bytes": save.stat().st_size, "truncated": truncated,
            "retained_sha256": hashlib.sha256(retained).hexdigest()}


def run_app(argv, env, output, save, receipt):
    with (output / "desktop.log").open("xb") as stream:
        app = subprocess.Popen(argv, env=env, stdout=stream, stderr=subprocess.STDOUT)
        receipt["pid"] = app.pid
        try:
            exercise(app, output, save, receipt)
        finally:
            if app.poll() is None:
                try:
                    app.kill()
                except ProcessLookupError:
                    pass
                else:
                    receipt["forced_cleanup"] = True
                app.wait(timeout=10)
            receipt["exit"] = app.returncode
            receipt["exit_status"] = (
                "forced cleanup" if receipt["forced_cleanup"] else
                "normal quit" if receipt["normal_quit_sent"] and app.returncode == 0 else
                "unexpected exit"
            )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--observer", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True)
    argv_binary = args.binary.resolve(strict=True)
    started = time.monotonic()
    receipt = {
        "commit": run("git", "rev-parse", "HEAD").decode().strip(),
        "binary_sha256": sha256(argv_binary), "viewport": RECTANGLE, "pid": None,
        "exit": None, "normal_quit_sent": False, "forced_cleanup": False, "phases": [],
    }
    with tempfile.TemporaryDirectory(prefix="meridian-x11-smoke-") as temp:
        save = Path(temp) / "save"
        save.mkdir(mode=0o700)
        runtime = Path(temp) / "runtime"
        runtime.mkdir(mode=0o700)
        profile = args.output.resolve() / "desktop-%p.profraw"
        argv = [str(argv_binary), "--save-dir", str(save)]
        env = dict(os.environ, XDG_RUNTIME_DIR=str(runtime), LLVM_PROFILE_FILE=str(profile))
        env.pop("WAYLAND_DISPLAY", None)
        receipt.update(argv=argv, stage="launch")
        try:
            run_app(argv, env, args.output, save, receipt)
            receipt["stage"] = "backend and desktop log"
            receipt["adapter"] = verify_log(args.output / "desktop.log")
            receipt["stage"] = "native LLVM profile"
            native_profile = args.output / f"desktop-{receipt['pid']}.profraw"
            if not native_profile.is_file() or native_profile.stat().st_size == 0:
                raise RuntimeError("Normal exit produced no native LLVM coverage data")
            receipt.update(profraw=native_profile.name, profraw_sha256=sha256(native_profile))
            receipt["stage"] = "save and observer"
            receipt["save_sha256"] = verify_save(args.observer.resolve(strict=True), save,
                                                 args.output, receipt)
        except Exception as error:
            receipt.update(status="failed", failure={"type": type(error).__name__, "message": str(error)})
            try:
                receipt["failed_save"] = retain_failed_save(save, args.output)
            except OSError as retention_error:
                receipt["failed_save_retention_error"] = str(retention_error)
            raise
        else:
            receipt.update(status="passed", stage="complete")
        finally:
            receipt["seconds"] = round(time.monotonic() - started, 3)
            (args.output / "native-smoke.json").write_text(json.dumps(receipt, indent=2) + "\n")
            print(json.dumps(receipt), flush=True)


if __name__ == "__main__":
    main()
