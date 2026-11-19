"""Synthetic receipts exercise the verifier without qualifying a GPU."""
import tempfile
import unittest
from pathlib import Path

from verify_goshawk_showroom_evidence import POSES, SAMPLES, VARIANTS, verify


class GoshawkShowroomEvidence(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory()
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)
        for pose in POSES:
            for variant, red in zip(VARIANTS, [10, 14, 14, 15], strict=True):
                self.write_ppm(pose, variant, red)
            (self.root / f"{pose}-gpu.tsv").write_text(
                "unsupported: adapter has no TIMESTAMP_QUERY\n"
            )

    def write_ppm(self, pose, variant, red):
        (self.root / f"{pose}-{variant}.ppm").write_bytes(b"P6\n1 1\n255\n" + bytes([red, 0, 0]))

    def test_reads_stable_pairs_and_marks_missing_timestamp_feature(self):
        result = verify(self.root, write_diffs=True)
        self.assertEqual(result["front"]["gpu_timing"], {"status": "unsupported"})
        self.assertEqual(result["front"]["shimmer"], {"changed_pixels": 1, "peak_channel_delta": 4})
        self.assertEqual(result["front"]["phase"], {"changed_pixels": 1, "peak_channel_delta": 1})
        self.assertEqual((self.root / "front-shimmer-delta-16x.ppm").read_bytes()[-3:], bytes([64, 0, 0]))

    def test_converts_raw_supported_gpu_ticks_and_rejects_invalid_pairs(self):
        header = "sample\tstart_tick\tend_tick\tns_per_tick\n"
        samples = "".join(f"{i}\t100\t{101 + i}\t2\n" for i in range(SAMPLES))
        for pose in POSES:
            (self.root / f"{pose}-gpu.tsv").write_text(header + samples)
        result = verify(self.root)["oblique"]["gpu_timing"]
        self.assertEqual(result["status"], "measured")
        self.assertEqual((result["min_ns"], result["median_ns"], result["p95_ns"]), (2, 17, 32))
        (self.root / "front-gpu.tsv").write_text(header + samples.replace("100\t101", "100\t99", 1))
        with self.assertRaisesRegex(ValueError, "Invalid GPU timestamp sample"):
            verify(self.root)

    def test_rejects_drifting_repeats_and_missing_temporal_modulation(self):
        self.write_ppm("front", "repeat", 13)
        with self.assertRaisesRegex(ValueError, "repeated production frame changed"):
            verify(self.root)
        self.write_ppm("front", "repeat", 14)
        for pose in POSES:
            self.write_ppm(pose, "phase", 14)
        with self.assertRaisesRegex(ValueError, "No nonzero same-pose"):
            verify(self.root)
