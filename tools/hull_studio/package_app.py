"""Bundle the editor, reference ship and native validator using PyInstaller."""

import argparse
import hashlib
import os
import plistlib
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from definitions import HERE, ROOT, encode, reference_bytes, reference_fit
from showroom_bundle import (
    MAC_EXECUTABLE,
    MAC_SHOWROOM_ID,
    MAC_SHOWROOM_NAME,
    MAC_SHOWROOM_RELATIVE,
)


def embed_macos_showroom(app, game, identity):
    child = app / "Contents" / MAC_SHOWROOM_RELATIVE
    executable = child / MAC_EXECUTABLE
    executable.parent.mkdir(parents=True)
    shutil.copy2(game, executable)
    with (app / "Contents/Info.plist").open("rb") as stream:
        outer = plistlib.load(stream)
    with (child / "Contents/Info.plist").open("wb") as stream:
        plistlib.dump(
            {
                "CFBundleIdentifier": MAC_SHOWROOM_ID,
                "CFBundleName": MAC_SHOWROOM_NAME,
                "CFBundleDisplayName": MAC_SHOWROOM_NAME,
                "CFBundleExecutable": executable.name,
                "CFBundlePackageType": "APPL",
                "CFBundleVersion": outer.get("CFBundleVersion", "1"),
                "CFBundleShortVersionString": outer.get(
                    "CFBundleShortVersionString", "1.0"
                ),
                "NSHighResolutionCapable": True,
            },
            stream,
        )
    signing = ["/usr/bin/codesign", "--force", "--sign", identity or "-"]
    if identity:
        signing.extend(["--options", "runtime", "--timestamp"])
    # Seal the independently identifiable helper before re-sealing the enclosing
    # PyInstaller app. Do not recursively overwrite its existing code signatures.
    for bundle in (child, app):
        subprocess.run([*signing, str(bundle)], check=True)
    subprocess.run(
        ["/usr/bin/codesign", "--verify", "--deep", "--strict", str(app)], check=True
    )


def package(validator, destination, identity, game):
    platform = sys.platform
    if platform not in ("darwin", "win32"):
        raise ValueError("The packaged editor targets macOS and Windows")
    if not (HERE / "web/index.html").is_file() or not validator.is_file():
        raise ValueError("The editor interface and native validator must both exist")
    if game is not None and not game.is_file():
        raise ValueError("The optional Showroom game executable must exist")
    with tempfile.TemporaryDirectory(prefix="meridian-hull-build-") as folder:
        stage = Path(folder)
        reference = stage / "reference.json"
        reference.write_bytes(reference_bytes())
        # Do not package an outdated binary or an invalid bundled ship.
        subprocess.run(
            [
                str(validator),
                "validate",
                str(reference),
                "--report",
                str(stage / "report.json"),
            ],
            check=True,
        )
        fit = stage / "reference-fit.json"
        fit.write_bytes(encode(reference_fit()))
        binary = stage / (
            "meridian-hull-tool.exe" if platform == "win32" else "meridian-hull-tool"
        )

        shutil.copy2(validator, binary)
        cmd = [
            sys.executable,
            "-m",
            "PyInstaller",
            "--clean",
            "--windowed",
            "--onedir",
            "--name",
            "Meridian Hull Studio",
            "--distpath",
            str(destination),
            "--workpath",
            str(stage / "build"),
            "--specpath",
            str(stage),
            "--add-binary",
            f"{binary}{os.pathsep}.",
            "--add-data",
            f"{reference}{os.pathsep}.",
            "--add-data",
            f"{fit}{os.pathsep}.",
            "--add-data",
            f"{HERE / 'web'}{os.pathsep}web",
            "--add-data",
            f"{ROOT / 'LICENSE'}{os.pathsep}notices",
            "--add-data",
            f"{ROOT / 'assets/fleet/sh023/LICENSE'}{os.pathsep}notices/sh023",
        ]
        if game and platform == "win32":
            desktop = stage / "meridian-desktop.exe"
            shutil.copy2(game, desktop)
            cmd.extend(["--add-binary", f"{desktop}{os.pathsep}."])
        if platform == "darwin":
            cmd.extend(["--osx-bundle-identifier", "io.xyzzy.meridian.hullstudio"])
            if identity:
                cmd.extend(["--codesign-identity", identity])
        cmd.append(str(HERE / "studio.py"))
        subprocess.run(cmd, check=True)
        if game and platform == "darwin":
            embed_macos_showroom(destination / "Meridian Hull Studio.app", game, identity)
        print(
            f"Packaged SH023 reference SHA-256 {hashlib.sha256(reference.read_bytes()).hexdigest()}"
        )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--validator", type=Path, required=True)
    parser.add_argument("--dist", type=Path, required=True)
    parser.add_argument(
        "--game", type=Path, help="Optional compatible Meridian Showroom executable"
    )
    parser.add_argument(
        "--signing-identity", help="Existing macOS Developer ID Application identity"
    )
    ns = parser.parse_args()
    package(
        ns.validator.resolve(),
        ns.dist.resolve(),
        ns.signing_identity,
        ns.game.resolve() if ns.game else None,
    )
