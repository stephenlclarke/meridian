"""Reproduce SH021's own fitted rounded surfaces and verify the original domains."""

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

import audit_zebra_surface as audit


class ZebraSurface(unittest.TestCase):
    def test_original_rounded_surface_certificates_replay_in_both_domains(self):
        saved = audit.OUTPUT.read_bytes()
        self.assertEqual(hashlib.sha256(saved).hexdigest(),
                         "5aa4ece65ac9ff214dad08f8a868e09907575d28413aee31f9cb0b10c1eef8dd")
        accepted = json.loads(saved)
        stdout = StringIO()
        with redirect_stdout(stdout), patch.object(sys,"argv",[audit.__file__]):
            runpy.run_path(audit.__file__,run_name="__main__")
        replay = json.loads(stdout.getvalue())
        self.assertEqual(replay["hull"], "SH021")
        self.assertEqual(replay["manifestSha256"], accepted["manifestSha256"])
        self.assertEqual(replay["pieceCount"], 48)
        self.assertEqual([r["piece"] for r in replay["perPiece"]],[r["piece"] for r in accepted["perPiece"]])
        self.assertEqual(audit.TARGET_MM,{"detailed":1000,"nativeCandidate":1600})
        for source, lower, upper in (("detailed", 1060, 1063),("nativeCandidate", 1515, 1597)):
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
            (("detailed","nativeCandidate",904,1498),("nativeCandidate","detailed",182,1041)),strict=True
        ):
            self.assertEqual((retained["source"],retained["comparedTo"],retained["vertex"],
                              retained["certifiedLowerMillimetres"]),(source,other,vertex,minimum))
            self.assertEqual((witness["source"],witness["comparedTo"],witness["vertex"]),(source,other,vertex))
            self.assertGreaterEqual(witness["certifiedLowerMillimetres"],minimum)

        for label in ("keel cross bridge (aft)","keel cross bridge (forward)"):
            original=next(row for row in accepted["perPiece"] if row["piece"]==label)
            measured=next(row for row in replay["perPiece"] if row["piece"]==label)
            for source,upper in (("detailed",581),("nativeCandidate",1142)):
                self.assertEqual(original["comparisons"][source]["fullRoundedPieceUpperMillimetres"],upper)
                self.assertLessEqual(measured["comparisons"][source]["fullRoundedPieceUpperMillimetres"],upper)

    def test_audit_refuses_same_length_asset_mutation_and_wrong_original(self):
        with tempfile.TemporaryDirectory() as tmp:
            copied = Path(tmp)/"sh021"
            shutil.copytree(audit.ASSETS,copied)
            path = copied/"source-triangles.bin"
            changed = bytearray(path.read_bytes())
            changed[0] ^= 1
            path.write_bytes(changed)
            with self.assertRaisesRegex(ValueError,"Changed surface audit input"):
                audit.read_inputs(copied)
        with self.assertRaisesRegex(ValueError,"Wrong surface audit identity"):
            audit.read_inputs(audit.ROOT/"assets/fleet/sh008")
