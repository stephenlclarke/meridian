from pathlib import Path
import tempfile
import unittest

import verify_styled_material_faces as verifier


class StyledMaterialEvidence(unittest.TestCase):
    def test_independent_expected_styled_values_bind_all_source_faces(self):
        for hull, glass_count in [("SH012", 984), ("SH019", 960)]:
            faces, _, _ = verifier.source_faces(hull)
            self.assertEqual(sum(verifier.STYLED[raw] == (100, 175, 188) for _, raw in faces), glass_count)
            self.assertEqual(sum(verifier.STYLED[raw] == (100, 150, 132) for _, raw in faces), 256)

    def test_styled_receipt_rejects_raw_wrong_and_missing_pixels(self):
        faces = [(2156, (116, 191, 204)), (2436, (185, 235, 217))]
        with tempfile.TemporaryDirectory() as folder:
            directory = Path(folder)
            pixels = bytearray(320 * 256 * 3)
            start_a, start_b = (4 * 320 + 4) * 3, (4 * 320 + 12) * 3
            pixels[start_a:start_a + 3] = bytes((100, 175, 188))
            pixels[start_b:start_b + 3] = bytes((100, 150, 132))
            ppm = directory / "SH012-styled-material-faces.ppm"
            tsv = directory / "SH012-styled-material-faces.tsv"
            header = "source_face\tx\ty\tsource_raw_rgb\tstyled_expected_rgb\tactual_rgb\n"
            rows = "2156\t4\t4\t116,191,204\t100,175,188\t100,175,188\n2436\t12\t4\t185,235,217\t100,150,132\t100,150,132\n"
            ppm.write_bytes(b"P6\n320 256\n255\n" + pixels)
            tsv.write_text(header + rows)
            self.assertEqual(verifier.verify_styled_grid(directory, "SH012", faces), 2)
            tsv.write_text(header + rows.splitlines(keepends=True)[0])
            with self.assertRaisesRegex(ValueError, "incomplete"):
                verifier.verify_styled_grid(directory, "SH012", faces)
            tsv.write_text(header + rows.replace("116,191,204", "100,175,188"))
            with self.assertRaisesRegex(ValueError, "styled receipt mismatch"):
                verifier.verify_styled_grid(directory, "SH012", faces)
            tsv.write_text(header + rows)
            for wrong in [(116, 191, 204), (100, 150, 132), (0, 0, 0)]:
                pixels[start_a:start_a + 3] = bytes(wrong)
                ppm.write_bytes(b"P6\n320 256\n255\n" + pixels)
                with self.assertRaisesRegex(ValueError, "styled GPU colour mismatch"):
                    verifier.verify_styled_grid(directory, "SH012", faces)

    def test_visible_whole_hull_receipts_match_actual_ppms(self):
        with tempfile.TemporaryDirectory() as folder:
            directory = Path(folder)
            rows = ["pose\tstyled_glazing_pixels\tstyled_emitter_pixels\n"]
            for pose in ["front", "oblique", "upper", "aft"]:
                (directory / f"SH019-{pose}-styled-solid.ppm").write_bytes(b"P6\n2 1\n255\n" + bytes((100, 175, 188, 100, 150, 132)))
                rows.append(f"{pose}\t1\t1\n")
            receipt = directory / "SH019-styled-visible.tsv"
            receipt.write_text("".join(rows))
            self.assertEqual(verifier.verify_visible_hull(directory, "SH019"), [4, 4])
            receipt.write_text("".join(rows).replace("front\t1\t1", "front\t2\t1"))
            with self.assertRaisesRegex(ValueError, "whole-hull receipt mismatch"):
                verifier.verify_visible_hull(directory, "SH019")


if __name__ == "__main__":
    unittest.main()
