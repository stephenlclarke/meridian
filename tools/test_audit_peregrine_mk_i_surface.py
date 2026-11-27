"""Reproduce SH022's own fitted rounded surfaces and verify the original domains."""

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

import audit_peregrine_mk_i_surface as audit


class PeregrineMkISurface(unittest.TestCase):
    def test_original_rounded_surface_certificates_replay_in_both_domains(self):
        saved = audit.OUTPUT.read_bytes()
        self.assertEqual(hashlib.sha256(saved).hexdigest(),
                         "172efe3867ce39e77d0c4f39df997e2af27430673ffb3d88a75c427877df409c")
        accepted = json.loads(saved)
        stdout = StringIO()
        with redirect_stdout(stdout), patch.object(sys,"argv",[audit.__file__]):
            runpy.run_path(audit.__file__,run_name="__main__")
        replay = json.loads(stdout.getvalue())
        self.assertEqual(replay["hull"], "SH022")
        self.assertEqual(replay["manifestSha256"], accepted["manifestSha256"])
        self.assertEqual(replay["pieceCount"], 69)
        self.assertEqual([r["piece"] for r in replay["perPiece"]],[r["piece"] for r in accepted["perPiece"]])
        self.assertEqual(audit.TARGET_MM,{"detailed":900,"nativeCandidate":1740})
        for source, lower, upper in (("detailed", 1156, 1160),("nativeCandidate", 1708, 1736)):
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
            (("detailed","nativeCandidate",336,1692),("nativeCandidate","detailed",74,1138)),strict=True
        ):
            self.assertEqual((retained["source"],retained["comparedTo"],retained["vertex"],
                              retained["certifiedLowerMillimetres"]),(source,other,vertex,minimum))
            self.assertEqual((witness["source"],witness["comparedTo"],witness["vertex"]),(source,other,vertex))
            self.assertGreaterEqual(witness["certifiedLowerMillimetres"],minimum)

        for label,detailed_upper,native_upper in (("fork longitudinal envelope (negative X)",572,1124),("fork longitudinal envelope (positive X)",572,1124),("fork aft cross bridge",759,1497)):
            original=next(row for row in accepted["perPiece"] if row["piece"]==label)
            measured=next(row for row in replay["perPiece"] if row["piece"]==label)
            for source,upper in (("detailed",detailed_upper),("nativeCandidate",native_upper)):
                self.assertEqual(original["comparisons"][source]["fullRoundedPieceUpperMillimetres"],upper)
                self.assertLessEqual(measured["comparisons"][source]["fullRoundedPieceUpperMillimetres"],upper)

    def test_audit_refuses_same_length_asset_mutation_and_wrong_original(self):
        with tempfile.TemporaryDirectory() as tmp:
            copied = Path(tmp)/"sh022"
            shutil.copytree(audit.ASSETS,copied)
            path = copied/"source-triangles.bin"
            changed = bytearray(path.read_bytes())
            changed[0] ^= 1
            path.write_bytes(changed)
            with self.assertRaisesRegex(ValueError,"Changed surface audit input"):
                audit.read_inputs(copied)
        with self.assertRaisesRegex(ValueError,"Wrong surface audit identity"):
            audit.read_inputs(audit.ROOT/"assets/fleet/sh008")
