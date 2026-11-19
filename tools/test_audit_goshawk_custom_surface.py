"""Reproduce SH010's own fitted rounded surfaces and verify the original domains."""

from contextlib import redirect_stdout
import hashlib
from io import StringIO
import json
from pathlib import Path
import runpy
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch

import audit_goshawk_custom_surface as audit


class GoshawkCustomSurface(unittest.TestCase):
    def test_original_rounded_surface_certificates_replay_in_both_domains(self):
        saved = audit.OUTPUT.read_bytes()
        self.assertEqual(hashlib.sha256(saved).hexdigest(),
                         "d31912ecde1eeb2545f9fe3ad0aac5f6a2cc9d3e1100d74cca818ea5b60b04b3")
        accepted = json.loads(saved)
        stdout = StringIO()
        with redirect_stdout(stdout), patch.object(sys,"argv",[audit.__file__]):
            runpy.run_path(audit.__file__,run_name="__main__")
        replay = json.loads(stdout.getvalue())
        self.assertEqual(replay["hull"], "SH010")
        self.assertEqual(replay["manifestSha256"], accepted["manifestSha256"])
        self.assertEqual(replay["pieceCount"], 73)
        self.assertEqual([r["piece"] for r in replay["perPiece"]],[r["piece"] for r in accepted["perPiece"]])
        for source, minimum in (("detailed", 600),("nativeCandidate", 1000)):
            with self.subTest(source=source):
                summary = replay["sourceComparisons"][source]
                self.assertLessEqual(summary["certifiedUpperMillimetres"], audit.TARGET_MM[source])
                self.assertGreaterEqual(summary["largestExposedSample"]["certifiedLowerMillimetres"], minimum)
                self.assertEqual(summary["certifiedUpperMillimetres"],max(
                    row["comparisons"][source]["fullRoundedPieceUpperMillimetres"] for row in replay["perPiece"]))
        self.assertGreaterEqual(replay["mandatoryCrossSourceVertices"][0]["certifiedLowerMillimetres"], 995)
        self.assertGreaterEqual(replay["mandatoryCrossSourceVertices"][1]["certifiedLowerMillimetres"], 295)

    def test_audit_refuses_same_length_asset_mutation_and_wrong_original(self):
        with tempfile.TemporaryDirectory() as tmp:
            copied = Path(tmp)/"sh010"
            shutil.copytree(audit.ASSETS,copied)
            path = copied/"source-triangles.bin"
            changed = bytearray(path.read_bytes())
            changed[0] ^= 1
            path.write_bytes(changed)
            with self.assertRaisesRegex(ValueError,"Changed surface audit input"):
                audit.read_inputs(copied)
        with self.assertRaisesRegex(ValueError,"Wrong surface audit identity"):
            audit.read_inputs(audit.ROOT/"assets/fleet/sh008")
