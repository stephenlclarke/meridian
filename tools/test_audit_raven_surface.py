"""Reproduce SH015's own fitted rounded surfaces and verify the original domains."""

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

import audit_raven_surface as audit


class RavenSurface(unittest.TestCase):
    def test_original_rounded_surface_certificates_replay_in_both_domains(self):
        saved = audit.OUTPUT.read_bytes()
        self.assertEqual(hashlib.sha256(saved).hexdigest(),
                         "2fe417d15247951a02ec2777e4e22d9500fa8fa084b9a2ae523f8c497cd77a49")
        accepted = json.loads(saved)
        stdout = StringIO()
        with redirect_stdout(stdout), patch.object(sys,"argv",[audit.__file__]):
            runpy.run_path(audit.__file__,run_name="__main__")
        replay = json.loads(stdout.getvalue())
        self.assertEqual(replay["hull"], "SH015")
        self.assertEqual(replay["manifestSha256"], accepted["manifestSha256"])
        self.assertEqual(replay["pieceCount"], 70)
        self.assertEqual([r["piece"] for r in replay["perPiece"]],[r["piece"] for r in accepted["perPiece"]])
        self.assertEqual(audit.TARGET_MM,{"detailed":900,"nativeCandidate":1500})
        for source, lower, upper in (("detailed", 864, 900),("nativeCandidate", 1524, 1530)):
            with self.subTest(source=source):
                summary = replay["sourceComparisons"][source]
                retained = accepted["sourceComparisons"][source]
                self.assertEqual(retained["largestExposedSample"]["certifiedLowerMillimetres"],lower)
                self.assertEqual(retained["certifiedUpperMillimetres"],upper)
                self.assertLessEqual(summary["certifiedUpperMillimetres"], upper)
                self.assertGreaterEqual(summary["largestExposedSample"]["certifiedLowerMillimetres"], lower)
                self.assertEqual(summary["certifiedUpperMillimetres"],max(
                    row["comparisons"][source]["fullRoundedPieceUpperMillimetres"] for row in replay["perPiece"]))
        for retained, witness, (source, other, vertex, minimum) in zip(
            accepted["mandatoryCrossSourceVertices"],replay["mandatoryCrossSourceVertices"],
            (("detailed","nativeCandidate",336,1508),("nativeCandidate","detailed",86,862)),strict=True
        ):
            self.assertEqual((retained["source"],retained["comparedTo"],retained["vertex"],
                              retained["certifiedLowerMillimetres"]),(source,other,vertex,minimum))
            self.assertEqual((witness["source"],witness["comparedTo"],witness["vertex"]),(source,other,vertex))
            self.assertGreaterEqual(witness["certifiedLowerMillimetres"],minimum)

    def test_audit_refuses_same_length_asset_mutation_and_wrong_original(self):
        with tempfile.TemporaryDirectory() as tmp:
            copied = Path(tmp)/"sh015"
            shutil.copytree(audit.ASSETS,copied)
            path = copied/"source-triangles.bin"
            changed = bytearray(path.read_bytes())
            changed[0] ^= 1
            path.write_bytes(changed)
            with self.assertRaisesRegex(ValueError,"Changed surface audit input"):
                audit.read_inputs(copied)
        with self.assertRaisesRegex(ValueError,"Wrong surface audit identity"):
            audit.read_inputs(audit.ROOT/"assets/fleet/sh008")
