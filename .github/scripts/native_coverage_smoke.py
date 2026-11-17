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


def exercise(app, output, save):
    window = await_result(app, "visible X11 window", lambda: mapped_window(app))
    focus(window)
    phases = [capture_phase(app, window, output, "title", title_frame)]
    if (save / SAVE_NAME).exists():
        raise RuntimeError("Fresh title unexpectedly created a save")
    press("n")
    phases.append(capture_phase(app, window, output, "system", system_frame))
    if "New commander ready" not in phases[-1]["title"]:
        raise RuntimeError("System frame did not follow ordinary commander creation")
    press("4")
    phases.append(capture_phase(app, window, output, "trading", trading_frame))
    if phases[-1]["sha256"] == phases[-2]["sha256"]:
        raise RuntimeError("Market did not present a distinct frame")
    press("Escape")
    phases.append(capture_phase(app, window, output, "hud", cockpit_frame))
    press("Escape")
    code = app.wait(timeout=20)
    if code:
        raise RuntimeError(f"Desktop normal quit exited {code}")
    return window, phases


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


def verify_save(observer, directory, output):
    save = directory / SAVE_NAME
    if sorted(p.name for p in directory.iterdir()) != [SAVE_NAME, "session.lock"]:
        raise RuntimeError("Normal quit did not produce exactly one isolated commander save")
    before = sha256(save)
    env = dict(os.environ, LLVM_PROFILE_FILE=str(directory.parent / "observer.profraw"))
    evidence = run(observer, save, env=env).decode()
    required = ("saved_format=15\n", "mode: Docked", "credits_tenths=Some(7200)\n",
                "used_tonnes: 0", "fuel_units: 130")
    if not all(item in evidence for item in required) or before != sha256(save):
        raise RuntimeError("Save decoder did not confirm the untouched starter commander")
    (output / "commander-observed.txt").write_text(evidence)
    return before


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--observer", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True)
    argv_binary = args.binary.resolve(strict=True)
    log = args.output / "desktop.log"
    started = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="meridian-x11-smoke-") as temp:
        save = Path(temp) / "save"
        save.mkdir(mode=0o700)
        runtime = Path(temp) / "runtime"
        runtime.mkdir(mode=0o700)
        profile = args.output.resolve() / "desktop-%p.profraw"
        argv = [str(argv_binary), "--save-dir", str(save)]
        env = dict(os.environ, XDG_RUNTIME_DIR=str(runtime), LLVM_PROFILE_FILE=str(profile))
        env.pop("WAYLAND_DISPLAY", None)
        with log.open("xb") as stream:
            app = subprocess.Popen(argv, env=env, stdout=stream, stderr=subprocess.STDOUT)
            try:
                window, phases = exercise(app, args.output, save)
            finally:
                if app.poll() is None:
                    app.kill()
                    app.wait(timeout=10)
        adapter = verify_log(log)
        native_profile = args.output / f"desktop-{app.pid}.profraw"
        if not native_profile.is_file() or native_profile.stat().st_size == 0:
            raise RuntimeError("Normal exit produced no native LLVM coverage data")
        saved_sha = verify_save(args.observer.resolve(strict=True), save, args.output)

    receipt = {
        "commit": run("git", "rev-parse", "HEAD").decode().strip(),
        "binary_sha256": sha256(argv_binary), "pid": app.pid, "x11_window": window,
        "viewport": RECTANGLE, "argv": argv, "exit": app.returncode, "adapter": adapter,
        "phases": phases, "save_sha256": saved_sha,
        "profraw": native_profile.name, "profraw_sha256": sha256(native_profile),
        "seconds": round(time.monotonic() - started, 3),
    }
    (args.output / "native-smoke.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt))


if __name__ == "__main__":
    main()
