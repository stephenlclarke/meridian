"""Reproduce the authored-metre SH019 rounded surface certificates."""

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

import audit_gazelle_surface as audit


class GazelleSurface(unittest.TestCase):
    def test_both_canonical_surfaces_and_all_eight_loop_sector_bounds_replay(self):
        raw=audit.OUTPUT.read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(),"2e6bc808295497c515fdca03d80fb7b4673d5514aa0ffa102525e597f0b7e21c")
        accepted=json.loads(raw)
        stdout=StringIO()
        with redirect_stdout(stdout),patch.object(sys,"argv",[audit.__file__]):
            runpy.run_path(audit.__file__,run_name="__main__")
        replay=json.loads(stdout.getvalue())
        self.assertEqual(replay["hull"],"SH019")
        self.assertEqual(replay["manifestSha256"],accepted["manifestSha256"])
        self.assertEqual(replay["pieceCount"],37)
        self.assertEqual([row["piece"] for row in replay["perPiece"]],[row["piece"] for row in accepted["perPiece"]])
        self.assertEqual(audit.TARGET_MM,{"detailed":700,"nativeCandidate":1400})
        for source,lower,upper in (("detailed",577,890),("nativeCandidate",1262,1400)):
            with self.subTest(source=source):
                retained=accepted["sourceComparisons"][source]; actual=replay["sourceComparisons"][source]
                self.assertEqual(retained["certifiedUpperMillimetres"],upper)
                self.assertEqual(retained["largestExposedSample"]["certifiedLowerMillimetres"],lower)
                self.assertLessEqual(actual["certifiedUpperMillimetres"],upper)
                self.assertGreaterEqual(actual["largestExposedSample"]["certifiedLowerMillimetres"],lower)
                self.assertEqual(actual["certifiedUpperMillimetres"],max(row["comparisons"][source]["fullRoundedPieceUpperMillimetres"] for row in replay["perPiece"]))
        for stored,current,expected in zip(accepted["mandatoryCrossSourceVertices"],replay["mandatoryCrossSourceVertices"],(("detailed","nativeCandidate",2966,1242),("nativeCandidate","detailed",102,88)),strict=True):
            source,other,vertex,lower=expected
            self.assertEqual((stored["source"],stored["comparedTo"],stored["vertex"],stored["certifiedLowerMillimetres"]),expected)
            self.assertEqual((current["source"],current["comparedTo"],current["vertex"]),(source,other,vertex))
            self.assertGreaterEqual(current["certifiedLowerMillimetres"],lower)
        for sector,native_upper in enumerate((921,921,921,921,1204,1118,1118,1204)):
            name=f"elliptical service loop sector {sector}"
            retained=next(row for row in accepted["perPiece"] if row["piece"]==name)
            measured=next(row for row in replay["perPiece"] if row["piece"]==name)
            for source,upper in (("detailed",890),("nativeCandidate",native_upper)):
                with self.subTest(sector=sector,source=source):
                    self.assertEqual(retained["comparisons"][source]["fullRoundedPieceUpperMillimetres"],upper)
                    self.assertLessEqual(measured["comparisons"][source]["fullRoundedPieceUpperMillimetres"],upper)

    def test_surface_audit_refuses_changed_support_or_wrong_original(self):
        with tempfile.TemporaryDirectory() as tmp:
            copied=Path(tmp)/"sh019";shutil.copytree(audit.ASSETS,copied)
            path=copied/"support.vertices";changed=bytearray(path.read_bytes());changed[24]^=1;path.write_bytes(changed)
            with self.assertRaisesRegex(ValueError,"Changed fixture"):
                audit.read_inputs(copied)
        with self.assertRaisesRegex(ValueError,"Changed fixture"):
            audit.read_inputs(audit.ROOT/"assets/fleet/sh008")
