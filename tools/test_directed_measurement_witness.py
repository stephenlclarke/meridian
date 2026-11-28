"""Check directed fit's exact witness and refusal of an empty source mesh."""

from importlib import import_module
import unittest

from test_hull_measurement_replays import AUTHORED_MODES


class TestDirectedMeasurementWitness(unittest.TestCase):
    def test_every_localisation_producer_binds_a_witness_or_refuses_empty_input(self):
        mesh = {"vertices": [[0, 0, 0], [1, 0, 0], [0, 1, 0]], "faces": [[0, 1, 2]]}
        part = [{"index": 0, "name": "panel", "faceStart": 0, "faceCount": 1}]
        full = (mesh, part)
        empty = ({**mesh, "faces": []}, [])
        for hull, modes in AUTHORED_MODES.items():
            if (("--directions",), "CANDIDATE-FIT-LOCALIZATION.json") not in modes:
                continue
            with self.subTest(hull=hull):
                slug = hull.split("-", 1)[1].replace("-", "_")
                directed = import_module(f"measure_{slug}").directed
                report = directed(full, full)
                self.assertEqual(report["sourceWitness"]["face"], 0)
                self.assertEqual(report["sourceWitness"]["squaredDistanceMetres"], "0")
                self.assertEqual(report["nearestTarget"]["faces"], [0])
                with self.assertRaisesRegex(ValueError, "requires a measured source face"):
                    directed(empty, full)
