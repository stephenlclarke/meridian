"""Independently check every SH009 display record and reject input/output mutations."""
import hashlib
from collections import Counter
import json
import math
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
import tomllib
import unittest

from encode_goshawk_mk2_visual import ASSETS, ROOT, TEXTURE_DIRECTORY, encode, prepare_visual, texture_provenance


def flip(data, index):
    return data[:index] + bytes([data[index] ^ 1]) + data[index + 1:]


class GoshawkMk2Visual(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.resource = (ASSETS / "resource.toml").read_bytes()
        cls.points = (ASSETS / "source.vertices").read_bytes()
        cls.triangles = (ASSETS / "source-triangles.bin").read_bytes()
        cls.visual = (ASSETS / "visual.bin").read_bytes()

    def test_shared_record_encoder_retains_the_original_receipt_pin(self):
        helper = ROOT / "tools/encode_goshawk_visual.py"
        self.assertEqual(hashlib.sha256(helper.read_bytes()).hexdigest(),
                         "9e524e9a32a237d71d212c8b5c34ecf12cde5a8e08885b1d03991b95c278f61c")
        self.assertEqual(encode(self.resource, self.points, self.triangles), self.visual)

    def test_every_original_corner_winding_material_uv_and_texture_record(self):
        model = tomllib.loads(self.resource.decode())["design"]
        self.assertEqual(struct.unpack_from("<8sII", self.visual), (b"GOS2VIZ1", 10596, 56))
        self.assertEqual(len(self.visual), 16 + 3532 * 3 * 56)
        used_textures, used_emissive = Counter(), Counter()
        repeated = False
        # Read the source directly. Do not use the production encoder or its triangle traversal.
        for face_index in range(len(model["faces"])):
            face = model["faces"][face_index]
            a, b, c = (model["vertices"][index] for index in face)
            edge_one = [b[i] - a[i] for i in range(3)]
            edge_two = [c[i] - b[i] for i in range(3)]
            normal = [edge_one[(i + 1) % 3] * edge_two[(i + 2) % 3]
                      - edge_one[(i + 2) % 3] * edge_two[(i + 1) % 3] for i in range(3)]
            magnitude = math.hypot(*normal)
            self.assertGreater(magnitude, 0)
            material = model["materials"][model["faceMaterials"][face_index]]
            rgba = tuple(channel / 255 for channel in material["colour"].to_bytes(4, "big"))
            mapping = model["faceTextures"][face_index]
            used_textures[mapping["texture"]] += 1
            used_emissive[material["emissive"]] += 1
            for corner in range(3):
                start = 16 + (face_index * 3 + corner) * 56
                record = self.visual[start:start + 56]
                self.assertEqual(record[:12], struct.pack("<3f", *model["vertices"][face[corner]]))
                stored_normal = struct.unpack("<3f", record[12:24])
                self.assertAlmostEqual(math.hypot(*stored_normal), 1.0, places=6)
                self.assertAlmostEqual(math.fsum(n * s / magnitude for n, s in zip(normal, stored_normal)), 1.0, places=6)
                self.assertEqual(record[24:40], struct.pack("<4f", *rgba))
                uv = mapping["uv"][corner]
                self.assertEqual(record[40:48], struct.pack("<2f", *uv))
                self.assertEqual(record[48:], struct.pack("<II", mapping["texture"], material["emissive"]))
                repeated |= any(value < 0 or value > 1 for value in uv)
        self.assertEqual(used_textures, {0: 1356, 2: 2176})
        self.assertEqual(used_emissive, {False: 3348, True: 184})
        self.assertTrue(repeated)
        self.assertTrue(all(texture["repeat"] for texture in model["textures"]))

    def test_production_reproduces_visual_and_provenance(self):
        prepared = prepare_visual(ASSETS)
        provenance = json.loads(prepared["VISUAL-PROVENANCE.json"])
        self.assertEqual(prepared["visual.bin"], self.visual)
        self.assertEqual(prepared["VISUAL-PROVENANCE.json"], (ASSETS / "VISUAL-PROVENANCE.json").read_bytes())
        self.assertEqual(hashlib.sha256(self.visual).hexdigest(), provenance["visual"]["sha256"])
        for path, row in provenance["sharedOriginalTextures"].items():
            data = (ROOT / path).read_bytes()
            self.assertEqual(len(data), row["width"] * row["height"] * 4)
            self.assertEqual(hashlib.sha256(data).hexdigest(), row["sha256"])

    def test_resource_detailed_winding_and_native_suffix_mutations_refuse(self):
        reversed_winding = self.triangles[4:8] + self.triangles[:4] + self.triangles[8:]
        cases = [
            (self.resource + b"\n", self.points, self.triangles, "resource"),
            (self.resource, flip(self.points, 16), self.triangles, "positions differ"),
            (self.resource, self.points, reversed_winding, "winding differs"),
            (self.resource, flip(self.points, len(self.points) - 1), self.triangles, "complete SH009 geometry"),
            (self.resource, self.points, flip(self.triangles, len(self.triangles) - 1), "complete SH009 geometry"),
        ]
        for resource, points, triangles, error in cases:
            with self.subTest(error=error), self.assertRaisesRegex(ValueError, error):
                encode(resource, points, triangles)

    def test_changed_texture_and_manifest_refuse(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp) / "textures"
            folder.mkdir()
            for name in ["TEXTURE-SOURCE.json", "l337-patrol-diffuse.rgba", "l337-strike-diffuse.rgba", "l337-service-details.rgba"]:
                shutil.copyfile(TEXTURE_DIRECTORY / name, folder / name)
            texture = folder / "l337-patrol-diffuse.rgba"
            original = texture.read_bytes()
            texture.write_bytes(flip(original, 0))
            with self.assertRaisesRegex(ValueError, "Changed original shared texture:"):
                texture_provenance(folder)
            texture.write_bytes(original)
            manifest = folder / "TEXTURE-SOURCE.json"
            manifest.write_bytes(manifest.read_bytes() + b"\n")
            with self.assertRaisesRegex(ValueError, "Changed original shared texture manifest"):
                texture_provenance(folder)

    def test_cli_check_does_not_overwrite_corrupt_visual_or_provenance(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp) / "sh009"
            shutil.copytree(ASSETS, folder)
            script = ROOT / "tools/encode_goshawk_mk2_visual.py"
            command = [sys.executable, "-B", str(script), "--assets", str(folder)]
            subprocess.run(command, check=True, capture_output=True)
            subprocess.run([*command, "--check"], check=True, capture_output=True)
            for name in ("visual.bin", "VISUAL-PROVENANCE.json"):
                with self.subTest(name=name):
                    path = folder / name
                    original = path.read_bytes()
                    corrupt = flip(original, len(original) // 2)
                    path.write_bytes(corrupt)
                    failure = subprocess.run([*command, "--check"], capture_output=True, text=True, check=False)
                    self.assertNotEqual(failure.returncode, 0)
                    self.assertIn(f"Changed SH009 visual output: {name}", failure.stderr)
                    self.assertEqual(path.read_bytes(), corrupt)
                    path.write_bytes(original)
            components = folder / "COMPONENTS.json"
            original = components.read_bytes()
            corrupt = original + b"\n"
            components.write_bytes(corrupt)
            failure = subprocess.run([*command, "--check"], capture_output=True, text=True, check=False)
            self.assertNotEqual(failure.returncode, 0)
            self.assertIn("Changed staged SH009 geometry: COMPONENTS.json", failure.stderr)
            self.assertEqual(components.read_bytes(), corrupt)
