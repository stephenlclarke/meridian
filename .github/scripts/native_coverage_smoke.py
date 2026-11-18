"""Exercise the instrumented desktop through X11 and inspect presented pixels."""

import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

from coverage_workspace import ROOT, workspace_path


WIDTH = 1280
HEIGHT = 720
RECTANGLE = (WIDTH, HEIGHT)
SAVE_NAME = "commander.meridian"
MAX_SAVE_BYTES = 65_536
MULE_TITLE = "Meridian INTERNAL MULE VALIDATION / NO SAVE | FLIGHT | PAUSED | H / ? Help"
MULE_REGION = (70, 260, 620, 460)
MULE_STEPS = 16


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
    if phases[-1]["title"] != "Meridian | DOCKED | RUNNING | H / ? Help":
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


def mule_opening_frame(rgb):
    # Sample open space above the initial hull. Its dark paint is visible by
    # contrast with space, even when no paint channel reaches text brightness.
    left, top, _, _ = MULE_REGION
    index = 3 * (top * WIDTH + left)
    sky = rgb[index:index + 3]

    def contrasts_with_sky(r, g, b):
        return max(abs(r - sky[0]), abs(g - sky[1]), abs(b - sky[2])) >= 24

    return count_pixels(rgb, MULE_REGION, contrasts_with_sky, step=2) > 80


def mule_changed_pixels(before, after):
    """Count strong RGB changes in the projected opening/approach hull region, outside the HUD."""
    left, top, right, bottom = MULE_REGION
    return sum(
        max(abs(before[index + channel] - after[index + channel]) for channel in range(3)) >= 24
        for y in range(top, bottom, 2)
        for x in range(left, right, 2)
        for index in (3 * (y * WIDTH + x),)
    )


def exercise_mule(app, output, profile_root, receipt):
    receipt["stage"] = "Mule map and focus"
    window = await_result(app, "visible Mule X11 window", lambda: mapped_window(app))
    receipt["x11_window"] = window
    focus(window)
    receipt["stage"] = "Mule opening hull"
    await_result(app, "paused Mule title", lambda: run(
        "xdotool", "getwindowname", window,
    ).decode().strip() == MULE_TITLE)
    opening = capture_phase(
        app, window, output, "mule-opening",
        mule_opening_frame,
    )
    receipt["phases"].append(opening)
    image = (output / opening["image"]).read_bytes().split(b"\n", 3)[3]
    if opening["title"] != MULE_TITLE:
        raise RuntimeError("Opening frame is not the paused Mule validation scene")
    receipt["stage"] = "Mule inbound movement"
    # Each distinct key press advances World once while the physical observer and station stay fixed.
    run("xdotool", "key", "--delay", "30", *(["period"] * MULE_STEPS))
    approach = capture_phase(
        app, window, output, "mule-approach",
        lambda rgb: mule_changed_pixels(image, rgb) > 200,
    )
    receipt["phases"].append(approach)
    moved = (output / approach["image"]).read_bytes().split(b"\n", 3)[3]
    receipt["changed_hull_region_samples"] = mule_changed_pixels(image, moved)
    if approach["title"] != MULE_TITLE:
        raise RuntimeError("Mule approach did not present in the isolated paused scene")
    receipt["stage"] = "Mule normal quit"
    press("Escape")
    receipt["normal_quit_sent"] = True
    code = app.wait(timeout=20)
    if code or profile_root.exists():
        raise RuntimeError(f"Mule exit {code} or unexpected profile directory")


def verify_mule_log(log):
    adapter = verify_log(log)
    text = log.read_text()
    if "Meridian Mule shimmer unavailable:" in text:
        raise RuntimeError("Mule production shimmer was disabled")
    prefix = "MULE_PRESENTED_DRAW_WALL "
    rows = [line.removeprefix(prefix) for line in text.splitlines() if line.startswith(prefix)]
    if len(rows) != 1:
        raise RuntimeError("Expected exactly one Mule normal-exit presentation aggregate")
    timing = json.loads(rows[0])
    frames, total, minimum, maximum = (
        timing.get(key) for key in (
            "presentedFrames", "totalNanoseconds", "minNanoseconds", "maxNanoseconds",
        )
    )
    if (any(type(value) is not int for value in (frames, total, minimum, maximum))
            or frames < 2 or not 0 < minimum <= maximum <= total
            or not frames * minimum <= total <= frames * maximum):
        raise RuntimeError("Mule timing did not verify multiple presented draws")
    return adapter, timing


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
    required = ("saved_format=17\n", "mode: Docked", "credits_tenths=Some(7200)\n",
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


def run_app(argv, env, output, save, receipt, *, journey=exercise, log_name="desktop.log"):
    with (output / log_name).open("xb") as stream:
        app = subprocess.Popen(argv, env=env, stdout=stream, stderr=subprocess.STDOUT)
        receipt["pid"] = app.pid
        try:
            journey(app, output, save, receipt)
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
            receipt["exit_status"] = exit_status(receipt)


def exit_status(receipt):
    if receipt["forced_cleanup"]:
        return "forced cleanup"
    if receipt["normal_quit_sent"] and receipt["exit"] == 0:
        return "normal quit"
    return "unexpected exit"


def at_stage(receipt, label, operation, *args):
    receipt["stage"] = label
    return operation(*args)


def native_profile_for(output, pid, prefix="desktop"):
    path = output / f"{prefix}-{pid}.profraw"
    if not path.is_file() or path.stat().st_size == 0:
        raise RuntimeError("Normal exit produced no native LLVM coverage data")
    return path


def run_mule(binary, output, ordinary):
    started = time.monotonic()
    argv = [str(binary), "--internal-mule-validation", "dock"]
    receipt = {
        "commit": ordinary["commit"], "binary_sha256": sha256(binary), "viewport": RECTANGLE,
        "argv": argv, "stage": "launch", "pid": None, "exit": None,
        "normal_quit_sent": False, "forced_cleanup": False, "phases": [],
    }
    log = output / "mule-desktop.log"
    with tempfile.TemporaryDirectory(prefix="meridian-x11-mule-") as temp:
        root = Path(temp)
        runtime = root / "runtime"
        runtime.mkdir(mode=0o700)
        profile_root = root / "profile"
        env = dict(
            os.environ, XDG_RUNTIME_DIR=str(runtime), XDG_DATA_HOME=str(profile_root),
            LLVM_PROFILE_FILE=str(output / "desktop-mule-%p.profraw"),
        )
        env.pop("WAYLAND_DISPLAY", None)
        try:
            if receipt["binary_sha256"] != ordinary["binary_sha256"]:
                raise RuntimeError("Instrumented desktop changed between native journeys")
            run_app(argv, env, output, profile_root, receipt, journey=exercise_mule, log_name=log.name)
            receipt["adapter"], receipt["presented_draw_wall"] = at_stage(
                receipt, "Mule backend, shimmer and presentation", verify_mule_log, log,
            )
            native_profile = at_stage(
                receipt, "Mule native LLVM profile", native_profile_for, output, receipt["pid"], "desktop-mule",
            )
            receipt.update(profraw=native_profile.name, profraw_sha256=sha256(native_profile))
        except Exception as error:
            receipt.update(status="failed", failure={"type": type(error).__name__, "message": str(error)})
            raise
        else:
            receipt.update(status="passed", stage="complete")
        finally:
            receipt["seconds"] = round(time.monotonic() - started, 3)
            (output / "mule-native-smoke.json").write_text(json.dumps(receipt, indent=2) + "\n")
            print(json.dumps(receipt), flush=True)


def main():
    output = workspace_path("coverage/native")
    argv_binary = workspace_path("target/debug/meridian-desktop")
    observer = workspace_path("target/debug/examples/inspect_commander")
    if not argv_binary.is_file() or not observer.is_file():
        raise RuntimeError("The instrumented desktop and observer must be built under target/debug")
    output.mkdir(parents=True)
    started = time.monotonic()
    receipt = {
        "commit": run("git", "-C", str(ROOT), "rev-parse", "HEAD").decode().strip(),
        "binary_sha256": sha256(argv_binary), "viewport": RECTANGLE, "pid": None,
        "exit": None, "normal_quit_sent": False, "forced_cleanup": False, "phases": [],
    }
    with tempfile.TemporaryDirectory(prefix="meridian-x11-smoke-") as temp:
        save = Path(temp) / "save"
        save.mkdir(mode=0o700)
        runtime = Path(temp) / "runtime"
        runtime.mkdir(mode=0o700)
        profile = output / "desktop-%p.profraw"
        argv = [str(argv_binary), "--save-dir", str(save)]
        env = dict(os.environ, XDG_RUNTIME_DIR=str(runtime), LLVM_PROFILE_FILE=str(profile))
        env.pop("WAYLAND_DISPLAY", None)
        receipt.update(argv=argv, stage="launch")
        try:
            run_app(argv, env, output, save, receipt)
            receipt["adapter"] = at_stage(
                receipt, "backend and desktop log", verify_log, output / "desktop.log",
            )
            native_profile = at_stage(
                receipt, "native LLVM profile", native_profile_for, output, receipt["pid"],
            )
            receipt.update(profraw=native_profile.name, profraw_sha256=sha256(native_profile))
            receipt["save_sha256"] = at_stage(
                receipt, "save and observer", verify_save, observer, save, output, receipt,
            )
        except Exception as error:
            receipt.update(status="failed", failure={"type": type(error).__name__, "message": str(error)})
            try:
                receipt["failed_save"] = retain_failed_save(save, output)
            except OSError as retention_error:
                receipt["failed_save_retention_error"] = str(retention_error)
            raise
        else:
            receipt.update(status="passed", stage="complete")
        finally:
            receipt["seconds"] = round(time.monotonic() - started, 3)
            (output / "native-smoke.json").write_text(json.dumps(receipt, indent=2) + "\n")
            print(json.dumps(receipt), flush=True)
    run_mule(argv_binary, output, receipt)


if __name__ == "__main__":
    main()
