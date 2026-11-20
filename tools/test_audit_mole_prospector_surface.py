"""Reproduce SH017's own fitted rounded surfaces and verify the original domains."""

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

import audit_mole_prospector_surface as audit


class MoleProspectorSurface(unittest.TestCase):
    def test_original_rounded_surface_certificates_replay_in_both_domains(self):
        saved = audit.OUTPUT.read_bytes()
        self.assertEqual(hashlib.sha256(saved).hexdigest(),
                         "9ad33433866326321f64a1ccef4c33c69495aefae39c9748264e1cd13c7a3508")
        accepted = json.loads(saved)
        stdout = StringIO()
        with redirect_stdout(stdout), patch.object(sys,"argv",[audit.__file__]):
            runpy.run_path(audit.__file__,run_name="__main__")
        replay = json.loads(stdout.getvalue())
        self.assertEqual(replay["hull"], "SH017")
        self.assertEqual(replay["manifestSha256"], accepted["manifestSha256"])
        self.assertEqual(replay["pieceCount"], 83)
        self.assertEqual([r["piece"] for r in replay["perPiece"]],[r["piece"] for r in accepted["perPiece"]])
        self.assertEqual(audit.TARGET_MM,{"detailed":1500,"nativeCandidate":2050})
        for source, lower, upper in (("detailed", 1273, 1998),("nativeCandidate", 2021, 2130)):
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
            (("detailed","nativeCandidate",337,2004),("nativeCandidate","detailed",91,723)),strict=True
        ):
            self.assertEqual((retained["source"],retained["comparedTo"],retained["vertex"],
                              retained["certifiedLowerMillimetres"]),(source,other,vertex,minimum))
            self.assertEqual((witness["source"],witness["comparedTo"],witness["vertex"]),(source,other,vertex))
            self.assertGreaterEqual(witness["certifiedLowerMillimetres"],minimum)

        criteria = json.loads((audit.OUTPUT.parent/"CRITERIA.json").read_bytes())
        collars = criteria["reviewEvidence"]["retainedCollarUpperMillimetres"]
        self.assertEqual(collars["negative X"],{"detailed":[1485,1479,1500,1998],"nativeCandidate":[2047,2024,2024,2048]})
        self.assertEqual(collars["positive X"],{"detailed":[1493,1479,1500,1998],"nativeCandidate":[2047,2024,2024,2048]})
        for side in ("negative X","positive X"):
            for index in range(4):
                label = f"boring head collar ({side}) axial {index}"
                actual = next(row for row in replay["perPiece"] if row["piece"]==label)
                recorded = next(row for row in accepted["perPiece"] if row["piece"]==label)
                for source in ("detailed","nativeCandidate"):
                    with self.subTest(collar=label,source=source):
                        expected = collars[side][source][index]
                        self.assertEqual(recorded["comparisons"][source]["fullRoundedPieceUpperMillimetres"],expected)
                        self.assertLessEqual(actual["comparisons"][source]["fullRoundedPieceUpperMillimetres"],expected)

    def test_audit_refuses_same_length_asset_mutation_and_wrong_original(self):
        with tempfile.TemporaryDirectory() as tmp:
            copied = Path(tmp)/"sh017"
            shutil.copytree(audit.ASSETS,copied)
            path = copied/"source-triangles.bin"
            changed = bytearray(path.read_bytes())
            changed[0] ^= 1
            path.write_bytes(changed)
            with self.assertRaisesRegex(ValueError,"Changed surface audit input"):
                audit.read_inputs(copied)
        with self.assertRaisesRegex(ValueError,"Wrong surface audit identity"):
            audit.read_inputs(audit.ROOT/"assets/fleet/sh008")
