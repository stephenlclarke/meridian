"""Check the Goshawk production importer against the shipped geometry and source pins."""
import shutil
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from unittest.mock import patch

import import_goshawk_envelope_source as importer


class GoshawkSourceImport(unittest.TestCase):
    def invoke(self, destination, *args):
        with patch.object(importer, "ASSET", destination), patch.object(
            sys, "argv", ["import_goshawk_envelope_source.py", *map(str, args)]
        ), redirect_stdout(StringIO()):
            importer.main()

    def test_production_rebuild_matches_shipped_files_and_check_detects_corruption(self):
        fixture_paths = {
            "snapshot/Sources/L337Core/Resources/Ships/SH008.toml": "resource.toml",
            "proposal/replacement-hulls/SH008-design.toml": "original-design.toml",
            "snapshot/docs/fleet-design-v2/ships/SH008.toml": "contract.toml",
            "snapshot/LICENSE": "LICENSE",
        }
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for relative, name in fixture_paths.items():
                source = root / relative
                source.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(importer.ASSET / name, source)
            out = root / "output"
            out.mkdir()
            self.invoke(out, "--snapshot", root / "snapshot", "--proposal", root / "proposal")
            expected = set(fixture_paths.values()) | {
                "source.vertices", "source-triangles.bin", "source-piece-membership.bin",
                "COMPONENT-PIECES.json", "MANIFEST.json",
            }
            self.assertEqual({p.name for p in out.iterdir()}, expected)
            for name in expected:
                with self.subTest(name=name):
                    self.assertEqual((out / name).read_bytes(), (importer.ASSET / name).read_bytes())
            self.invoke(out, "--check")
            faces = out / "source-triangles.bin"
            damaged = faces.read_bytes()[12:]
            faces.write_bytes(damaged)
            with self.assertRaisesRegex(AssertionError, "source-triangles"):
                self.invoke(out, "--check")
            self.assertEqual(faces.read_bytes(), damaged)

    def test_optimized_python_refuses_both_cli_modes_before_accessing_assets(self):
        with tempfile.TemporaryDirectory() as tmp:
            script = Path(tmp) / "tools" / "import_goshawk_envelope_source.py"
            script.parent.mkdir()
            shutil.copyfile(importer.__file__, script)
            for flag in ("-O", "-OO"):
                for args in (["--check"], ["--snapshot", tmp, "--proposal", tmp]):
                    with self.subTest(flag=flag, args=args):
                        result = subprocess.run(
                            [sys.executable, "-B", flag, str(script), *args],
                            capture_output=True, text=True, check=False,
                        )
                        self.assertNotEqual(result.returncode, 0)
                        self.assertIn("requires Python assertions", result.stderr)
                        self.assertEqual(result.stdout, "")
                        self.assertFalse((Path(tmp) / "assets").exists())

    def test_each_changed_original_input_is_rejected_before_encoding(self):
        originals = {name: (importer.ASSET / name).read_bytes() for name in importer.PINS}
        for name, original in originals.items():
            with self.subTest(name=name), self.assertRaises(AssertionError) as error:
                importer.encode({**originals, name: original + b"\n"})
            self.assertEqual(str(error.exception), name)
