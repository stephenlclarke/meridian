"""Check stock archive group provenance and per-vertex membership in shallow CI checkouts."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import verify_original_stock_inspection as verifier


class StockArchiveGroupTests(unittest.TestCase):
    def check_group_mutations(self, package_path, original_groups_sha256):
        package = json.loads((verifier.MERIDIAN_REPO / package_path).read_bytes())
        pin = package["provenance"]["groupSource"]
        self.assertEqual(pin, verifier.GROUP_SOURCES[package["id"]])
        # The digest comes from the two arrays in the pinned historical Git blob.
        groups = [package[level]["groups"] for level in ("detail", "native")]
        encoded = json.dumps(groups, separators=(",", ":")).encode()
        self.assertEqual(hashlib.sha256(encoded).hexdigest(), original_groups_sha256)
        archive = {"id": package["id"], "detail": {"groups": groups[0]}, "native": {"groups": groups[1]}}
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "changed.json"
            for level in ("detail", "native"):
                with self.subTest(level=level):
                    original = package[level]["groups"]
                    other = next(group for group in original if group != original[0])
                    package[level]["groups"] = [other, *original[1:]]
                    path.write_text(json.dumps(package))
                    package[level]["groups"] = original
                    with patch.object(verifier, "read_group_archive", return_value=archive) as read:
                        with self.assertRaisesRegex(ValueError, f"Changed {level} group membership"):
                            verifier.verify(path, folder)
                        read.assert_called_once_with(pin)
            for field in ("meridianRevision", "path", "sha256"):
                with self.subTest(field=field), patch.object(verifier, "read_group_archive") as read:
                    package["provenance"]["groupSource"] = {**pin, field: "changed"}
                    path.write_text(json.dumps(package))
                    with self.assertRaisesRegex(ValueError, "Unexpected Meridian group source"):
                        verifier.verify(path, folder)
                    read.assert_not_called()

    def test_vulture_mk_ii_group_membership(self):
        self.check_group_mutations(
            "integration/sh038-vulture-mk-ii/ORIGINAL-INSPECTION.json",
            "79da2b2d38c3f41ce337c8f0f05ae0a55cb301e607d0de74f714f9211697d462",
        )

    def test_group_archive_hash_checked_before_decode(self):
        pin = verifier.GROUP_SOURCES["SH038"]
        with patch.object(verifier.subprocess, "check_output", return_value=b"wrong archive") as read:
            with self.assertRaisesRegex(ValueError, "Meridian group archive hash mismatch"):
                verifier.read_group_archive(pin)
            read.assert_called_once_with([
                "git", "-C", verifier.MERIDIAN_REPO, "cat-file", "blob",
                f"{pin['meridianRevision']}:{pin['path']}",
            ])
