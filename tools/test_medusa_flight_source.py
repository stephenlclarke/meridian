"""Exercise the exact original-derived Medusa collision package."""
from pathlib import Path
import unittest
from unittest.mock import patch

import prepare_medusa_flight as source


class MedusaFlightSource(unittest.TestCase):
    def test_physics_and_rounded_supports_match_the_verified_trial(self):
        self.assertEqual(source.collision_bytes(), source.OUTPUT.read_bytes())
        self.assertEqual(source.support_bytes(), source.SUPPORT_OUTPUT.read_bytes())

    def test_refuses_altered_triangle_or_support_sources(self):
        for path, loader in ((source.SOURCE, source.collision_bytes),
                             (source.SUPPORT_SOURCE, source.support_bytes)):
            with self.subTest(path=path.name), patch.object(Path, "read_bytes", return_value=b"altered"):
                with self.assertRaises(ValueError):
                    loader()
