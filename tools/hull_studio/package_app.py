"""Bundle the editor, reference ship and native validator using PyInstaller."""

import argparse
import hashlib
import os
import subprocess
import sys
import tempfile
from pathlib import Path

from definitions import HERE, ROOT, encode, reference_bytes, reference_fit


def package(validator, destination, identity):
    platform = sys.platform
    if platform not in ("darwin", "win32"):
        raise ValueError("The packaged editor targets macOS and Windows")
    if not (HERE / "web/index.html").is_file() or not validator.is_file():
        raise ValueError("The editor interface and native validator must both exist")
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
        import shutil

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
        if platform == "darwin":
            cmd.extend(["--osx-bundle-identifier", "io.xyzzy.meridian.hullstudio"])
            if identity:
                cmd.extend(["--codesign-identity", identity])
        cmd.append(str(HERE / "studio.py"))
        subprocess.run(cmd, check=True)
        print(
            f"Packaged SH023 reference SHA-256 {hashlib.sha256(reference.read_bytes()).hexdigest()}"
        )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--validator", type=Path, required=True)
    parser.add_argument("--dist", type=Path, required=True)
    parser.add_argument(
        "--signing-identity", help="Existing macOS Developer ID Application identity"
    )
    ns = parser.parse_args()
    package(ns.validator.resolve(), ns.dist.resolve(), ns.signing_identity)
