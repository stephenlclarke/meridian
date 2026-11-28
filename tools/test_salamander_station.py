"""Exercise the selected gate revision and its full rounded inspection passage."""
import json
import tomllib
import unittest
from fractions import Fraction

from prepare_salamander_station import FOLDER, ROOT, aperture_margin, definition


class SalamanderStationTests(unittest.TestCase):
    def test_preserves_each_original_corner_at_the_split_seams(self):
        source = tomllib.loads((ROOT/"assets/fleet/sh141/resource.toml").read_text())
        pkg = definition()
        self.assertEqual(pkg, json.loads((FOLDER/"SALAMANDER-STUDIO.json").read_bytes()))
        for original, level in (("design", "detail"), ("nativeDesign", "native")):
            mesh, output = source[original], pkg[level]
            for old, new in zip(mesh["faces"], output["triangles"], strict=True):
                self.assertEqual([mesh["vertices"][i] for i in old], [output["vertices"][i] for i in new])
                self.assertEqual(len({output["groups"][i] for i in new}), 1)

    def test_full_sweep_margin_detects_a_bridge_across_the_open_port(self):
        package = definition()
        self.assertEqual(aperture_margin(package), Fraction(49, 50))
        native = package["native"]
        native["groups"] = [native["groups"][0]] * len(native["groups"])
        self.assertLess(aperture_margin(package), 0)
