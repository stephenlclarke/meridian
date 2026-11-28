"""Checks for the published span triangulations used by alternative-fit reports."""

import unittest

import measure_dzo as dzo
import measure_dzo_mk_ii as dzo_mk_ii
import measure_oryx_voyager as oryx
from measure_collar_support_fans import load_span_fans, validate_span_fans


class AuthoredSpanFans(unittest.TestCase):
    def test_each_published_fan_covers_the_original_span(self):
        for measure, supports in ((dzo, dzo.COLLARS), (dzo_mk_ii, dzo_mk_ii.COLLARS), (oryx, ("ring-wing",))):
            with self.subTest(hull=measure.__name__):
                vertices, meshes, groups = measure.load()
                records = load_span_fans(measure.INTEGRATION / "OPTION-SUPPORT-FANS.json", supports)
                for name in supports:
                    for count in (4, 8):
                        spans = measure.collar_spans(groups[name], meshes["native"][0], count)
                        faces = validate_span_fans(vertices, spans, records[name][str(count)])
                        self.assertEqual([fan.tolist() for fan in faces], records[name][str(count)])

    def test_foreign_vertex_missing_span_and_reversed_face_are_refused(self):
        vertices, meshes, groups = dzo.load()
        name = dzo.COLLARS[0]
        spans = dzo.collar_spans(groups[name], meshes["native"][0], 4)
        records = load_span_fans(dzo.INTEGRATION / "OPTION-SUPPORT-FANS.json", dzo.COLLARS)[name]["4"]
        with self.assertRaisesRegex(ValueError, "original support span"):
            outside = max(spans[0]) + 1
            first, second, third = records[0][0]
            changed = [[[outside, second, third], *records[0][1:]], *records[1:]]
            validate_span_fans(vertices, spans, changed)
        with self.assertRaises(ValueError):
            validate_span_fans(vertices, spans, records[:-1])
        with self.assertRaises(AssertionError):
            reversed_face = [[[first, third, second], *records[0][1:]], *records[1:]]
            validate_span_fans(vertices, spans, reversed_face)
