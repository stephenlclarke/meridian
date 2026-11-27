"""Test the portable game lookup and macOS nested app signing contract."""

import plistlib
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).parent / "hull_studio"))
import package_app
import showroom_bundle


class ShowroomPackagingTests(unittest.TestCase):
    def test_frozen_macos_looks_up_signed_nested_game(self):
        contents = Path("dist/Meridian Hull Studio.app/Contents")
        executable = contents / "MacOS/Meridian Hull Studio"
        validator = contents / "Frameworks/meridian-hull-tool"
        self.assertEqual(
            showroom_bundle.showroom_executable(validator, executable, "darwin", True),
            contents
            / "Helpers/Meridian Hull Showroom.app/Contents/MacOS/meridian-desktop",
        )

    def test_windows_and_development_keep_direct_executables(self):
        for platform, frozen, name in (
            ("darwin", False, "meridian-desktop"),
            ("linux", False, "meridian-desktop"),
            ("win32", False, "meridian-desktop.exe"),
            ("win32", True, "meridian-desktop.exe"),
        ):
            with self.subTest(platform=platform, frozen=frozen):
                validator = Path("release/validator")
                self.assertEqual(
                    showroom_bundle.showroom_executable(
                        validator, Path("python"), platform, frozen
                    ),
                    validator.with_name(name),
                )

    def test_nested_bundle_identity_and_signing_order(self):
        for identity in ("Developer ID Application: Example", None):
            with self.subTest(identity=identity), tempfile.TemporaryDirectory() as tmp:
                folder = Path(tmp)
                app = folder / "Meridian Hull Studio.app"
                (app / "Contents").mkdir(parents=True)
                with (app / "Contents/Info.plist").open("wb") as stream:
                    plistlib.dump({"CFBundleVersion": "7"}, stream)
                game = folder / "game"
                game.write_bytes(b"original executable")
                game.chmod(0o755)
                with mock.patch.object(package_app.subprocess, "run") as run:
                    package_app.embed_macos_showroom(app, game, identity)
                child = app / "Contents/Helpers/Meridian Hull Showroom.app"
                executable = child / "Contents/MacOS/meridian-desktop"
                self.assertEqual(executable.read_bytes(), game.read_bytes())
                with (child / "Contents/Info.plist").open("rb") as stream:
                    info = plistlib.load(stream)
                self.assertEqual(info["CFBundleExecutable"], executable.name)
                self.assertEqual(info["CFBundleVersion"], "7")
                self.assertEqual(info["CFBundlePackageType"], "APPL")
                self.assertEqual(
                    info["CFBundleIdentifier"], "io.xyzzy.meridian.hullstudio.showroom"
                )
                self.assertNotIn("LSUIElement", info)
                signing = ["/usr/bin/codesign", "--force", "--sign", identity or "-"]
                if identity:
                    signing.extend(["--options", "runtime", "--timestamp"])
                self.assertEqual(
                    run.call_args_list,
                    [
                        mock.call([*signing, str(child)], check=True),
                        mock.call([*signing, str(app)], check=True),
                        mock.call(
                            ["/usr/bin/codesign", "--verify", "--deep", "--strict", str(app)],
                            check=True,
                        ),
                    ],
                )


if __name__ == "__main__":
    unittest.main()
