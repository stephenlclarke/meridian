import tempfile
from pathlib import Path
import unittest

import verify_joined_material_faces as verifier


class JoinedMaterialEvidence(unittest.TestCase):
    def test_original_source_material_records_and_distinct_markers(self):
        for hull, glazing in [("SH012", 984), ("SH019", 960)]:
            faces, _, _ = verifier.source_faces(hull)
            self.assertEqual(sum(rgb == (116, 191, 204) for _, rgb in faces), glazing)
            self.assertEqual(sum(rgb == (185, 235, 217) for _, rgb in faces), 256)

    def test_grid_requires_every_original_face_raw_pixel_and_receipt(self):
        faces = [(2156, (116, 191, 204)), (2436, (185, 235, 217))]
        with tempfile.TemporaryDirectory() as folder:
            directory = Path(folder)
            pixels = bytearray(320 * 256 * 3)
            pixels[(4 * 320 + 4) * 3:(4 * 320 + 4) * 3 + 3] = bytes(faces[0][1])
            pixels[(4 * 320 + 12) * 3:(4 * 320 + 12) * 3 + 3] = bytes(faces[1][1])
            ppm = directory / "SH012-material-faces.ppm"
            tsv = directory / "SH012-material-faces.tsv"
            header = "source_face\tx\ty\traw_expected_rgb\tactual_rgb\n"
            rows = "2156\t4\t4\t116,191,204\t116,191,204\n2436\t12\t4\t185,235,217\t185,235,217\n"
            ppm.write_bytes(b"P6\n320 256\n255\n" + pixels)
            tsv.write_text(header + rows)
            self.assertEqual(verifier.verify_grid(directory, "SH012", faces), 2)
            tsv.write_text(header + rows.splitlines(keepends=True)[0])
            with self.assertRaisesRegex(ValueError, "incomplete"):
                verifier.verify_grid(directory, "SH012", faces)
            tsv.write_text(header + rows)
            pixels[(4 * 320 + 4) * 3] = 100  # A styled or substituted sample is not a raw pass.
            ppm.write_bytes(b"P6\n320 256\n255\n" + pixels)
            with self.assertRaisesRegex(ValueError, "raw GPU colour mismatch"):
                verifier.verify_grid(directory, "SH012", faces)


if __name__ == "__main__":
    unittest.main()
