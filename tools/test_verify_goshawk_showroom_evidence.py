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
        self.views = self.root / "hull-local-views.tsv"
        self.views.write_text("pose\tx\ty\tz\nfront\t0\t0\t1\noblique\t0.6\t0\t0.8\nupper\t0\t0.8\t0.6\n")
        for pose in POSES:
            for variant, red in zip(VARIANTS, [10, 14, 14, 15], strict=True):
                self.write_ppm(pose, variant, red)
            (self.root / f"{pose}-gpu.tsv").write_text(
                "unsupported: adapter has no TIMESTAMP_QUERY\n"
            )

    def write_ppm(self, pose, variant, red):
        (self.root / f"{pose}-{variant}.ppm").write_bytes(b"P6\n1 1\n255\n" + bytes([red, 0, 0]))

    def test_reads_stable_pairs_and_marks_missing_timestamp_feature(self):
        result = verify(self.root, write_diffs=True, timing_dir=self.root)
        self.assertEqual(result["front"]["gpu_timing"], {"status": "unsupported"})
        self.assertEqual(result["front"]["shimmer"], {"changed_pixels": 1, "peak_channel_delta": 4})
        self.assertEqual(result["front"]["phase"], {"changed_pixels": 1, "peak_channel_delta": 1})
        self.assertEqual((self.root / "front-shimmer-delta-16x.ppm").read_bytes()[-3:], bytes([64, 0, 0]))

    def write_timing(self, pose, first, period=2, overlap=False):
        rows = ["sample\tstart_tick\tend_tick\tns_per_tick\n"]
        for index in range(SAMPLES):
            start = first if overlap and index == 1 else first + 20 * index
            rows.append(f"{index}\t{start}\t{start + index + 1}\t{period}\n")
        (self.root / f"{pose}-gpu.tsv").write_text("".join(rows))

    def measured_timing(self):
        for index, pose in enumerate(POSES):
            self.write_timing(pose, 100 + index * 1000)

    def test_converts_raw_supported_gpu_ticks_and_rejects_nonpositive_intervals(self):
        self.measured_timing()
        result = verify(self.root, timing_dir=self.root)["oblique"]["gpu_timing"]
        self.assertEqual(result["status"], "measured")
        self.assertEqual((result["min_ns"], result["median_ns"], result["p95_ns"]), (2, 17, 32))
        self.assertEqual(result["ns_per_tick"], 2)
        timing = self.root / "front-gpu.tsv"
        timing.write_text(timing.read_text().replace("100\t101", "100\t99", 1))
        with self.assertRaisesRegex(ValueError, "Invalid GPU timestamp sample"):
            verify(self.root, timing_dir=self.root)

    def test_refuses_partial_measurement_and_stray_samples_on_unsupported_adapter(self):
        self.measured_timing()
        path = self.root / "front-gpu.tsv"
        rows = path.read_text().splitlines(keepends=True)
        path.write_text("".join(rows[:-1]))
        with self.assertRaisesRegex(ValueError, "sample count"):
            verify(self.root, timing_dir=self.root)
        path.write_text("unsupported: adapter has no TIMESTAMP_QUERY\n" + rows[1])
        with self.assertRaisesRegex(ValueError, "Unsupported timing file contains samples"):
            verify(self.root, timing_dir=self.root)

    def test_rejects_overlapping_intervals_within_and_across_poses(self):
        self.measured_timing()
        self.write_timing("front", 100, overlap=True)
        with self.assertRaisesRegex(ValueError, "within a pose"):
            verify(self.root, timing_dir=self.root)
        self.write_timing("front", 100)
        self.write_timing("oblique", 400)
        with self.assertRaisesRegex(ValueError, "across poses"):
            verify(self.root, timing_dir=self.root)

    def test_rejects_a_new_timestamp_period_on_the_same_queue(self):
        self.measured_timing()
        self.write_timing("oblique", 1100, period=3)
        with self.assertRaisesRegex(ValueError, "period changed across poses"):
            verify(self.root, timing_dir=self.root)

    def test_visual_verification_is_independent_of_invalid_or_missing_gpu_timing(self):
        self.measured_timing()
        (self.root / "front-gpu.tsv").unlink()
        self.assertNotIn("gpu_timing", verify(self.root)["front"])
        with self.assertRaises(FileNotFoundError):
            verify(self.root, timing_dir=self.root)
        self.write_timing("front", 100)
        front = self.root / "front-gpu.tsv"
        front.write_text(front.read_text().replace("100\t101", "100\t18446744073709551615", 1))
        self.assertNotIn("gpu_timing", verify(self.root)["front"])
        with self.assertRaisesRegex(ValueError, "Invalid GPU timestamp sample"):
            verify(self.root, timing_dir=self.root)

    def test_rejects_an_oblique_capture_with_only_image_plane_roll(self):
        self.views.write_text(self.views.read_text().replace(
            "oblique\t0.6\t0\t0.8", "oblique\t0\t0\t1"
        ))
        with self.assertRaisesRegex(ValueError, "Missing distinct"):
            verify(self.root)

    def test_rejects_arbitrary_same_size_phase_images_against_both_references(self):
        self.write_ppm("front", "phase", 255)
        with self.assertRaisesRegex(ValueError, "shifted versus solid"):
            verify(self.root)
        self.write_ppm("front", "phase", 23)
        with self.assertRaisesRegex(ValueError, "shifted versus normal"):
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
