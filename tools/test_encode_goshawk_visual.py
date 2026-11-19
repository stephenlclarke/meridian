"""Exercise the original SH008 packed source binding."""
import hashlib
import json
import unittest

from encode_goshawk_visual import ROOT, encode


class GoshawkVisualSource(unittest.TestCase):
    def setUp(self):
        self.directory = ROOT / "assets/fleet/sh008"
        self.resource, self.vertices, self.faces = (
            (self.directory / name).read_bytes()
            for name in ("resource.toml", "source.vertices", "source-triangles.bin")
        )

    def test_original_material_uv_and_collision_coordinates_reproduce_the_preview(self):
        manifest = json.loads((self.directory / "VISUAL-PROVENANCE.json").read_text())
        encoded = encode(self.resource, self.vertices, self.faces)
        self.assertEqual(encoded, (self.directory / "visual.bin").read_bytes())
        self.assertEqual(hashlib.sha256(encoded).hexdigest(), manifest["visual"]["sha256"])
        for path, source in manifest["sharedOriginalTextures"].items():
            self.assertEqual(hashlib.sha256((ROOT / path).read_bytes()).hexdigest(), source["sha256"])

    def test_original_geometry_assets_and_station_witness_match_their_pinned_bytes(self):
        manifest = json.loads((self.directory / "MANIFEST.json").read_text())
        for name, pin in manifest["files"].items():
            with self.subTest(asset=name):
                data = (self.directory / name).read_bytes()
                self.assertEqual(len(data), pin["bytes"])
                self.assertEqual(hashlib.sha256(data).hexdigest(), pin["sha256"])

        integration = ROOT / "integration/sh008-goshawk"
        criteria = json.loads((integration / "CRITERIA.json").read_text())
        witness = (integration / "SH028-LIP-WITNESS.json").read_bytes()
        self.assertEqual(
            hashlib.sha256(witness).hexdigest(), criteria["stationSlice"]["witnessSha256"]
        )

    def test_changed_detailed_positions_or_winding_refuse_render_encoding(self):
        def flipped(data, index):
            return data[:index] + bytes([data[index] ^ 1]) + data[index + 1:]

        with self.assertRaisesRegex(ValueError, "positions differ"):
            encode(self.resource, flipped(self.vertices, 16), self.faces)
        with self.assertRaisesRegex(ValueError, "winding differs"):
            encode(self.resource, self.vertices, flipped(self.faces, 0))
