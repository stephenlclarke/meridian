"""Exercise the published Griffon support fans without live triangulation."""

import unittest
from unittest.mock import patch

import measure_griffon_geometry as griffon
import measure_griffon_mk_ii_geometry as mk_ii
import measure_griffon_spectre_geometry as spectre
import measure_lanner_geometry


class AuthoredGriffonFans(unittest.TestCase):
    def test_affected_modes_use_certified_fans_without_qhull(self):
        for measure, use_candidate in ((m, mode) for m in (griffon, mk_ii, spectre) for mode in (False, True)):
            with self.subTest(hull=measure.__name__, use_candidate=use_candidate):
                with (patch.object(measure_lanner_geometry, "ConvexHull",
                                   side_effect=AssertionError("unexpected live triangulation")),
                      patch.object(measure, "adaptive_upper", return_value=0) as bound):
                    report = measure.support_fit(measure.load(), use_candidate)
                self.assertTrue(report["pieces"])
                self.assertEqual(bound.call_count, len(report["pieces"]) * (3 if use_candidate else 2))
