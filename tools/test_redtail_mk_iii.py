"""Verify the original Redtail Mk III stock package and its source inputs."""
import json
from pathlib import Path
import shutil
import tempfile
import tomllib
import unittest
from unittest.mock import patch

import prepare_redtail_mk_iii as redtail


class RedtailMkIIITests(unittest.TestCase):
    def test_package_rebuild_preserves_scale_groups_and_original_texture_coordinates(self):
        package = redtail.definition()
        encoded = json.dumps(package, separators=(",", ":"), allow_nan=False) + "\n"
        self.assertEqual(encoded.encode(), redtail.DEST.read_bytes())
        self.assertEqual((package["id"], package["metresPerUnit"]), ("SH103", 1))
        self.assertEqual(set(package["detail"]["groups"]), set(range(26)))
        self.assertEqual(set(package["native"]["groups"]), set(range(26, 33)))

        source = tomllib.loads((redtail.ASSETS / "resource.toml").read_text())["design"]
        self.assertEqual(len(package["faceTextures"]), 2328)
        for actual, original in zip(package["faceTextures"], source["faceTextures"], strict=True):
            index = original["texture"]
            texture = None if index < 0 else source["textures"][index]["asset"]
            self.assertEqual(actual, {"texture": texture, "uv": original["uv"]})

    def test_changed_native_archive_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            assets = Path(tmp) / "sh103"
            shutil.copytree(redtail.ASSETS, assets)
            native = assets / "native-export.json"
            native.write_bytes(native.read_bytes().replace(b"SH103", b"SH102", 1))
            with patch.object(redtail, "ASSETS", assets):
                with self.assertRaisesRegex(ValueError, "Changed original Redtail native-export.json"):
                    redtail.definition()
