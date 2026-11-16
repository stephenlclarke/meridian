"""Offline regressions for version selection, retention and downloaded bytes."""
import tempfile
import argparse
import contextlib
import io
import unittest
from pathlib import Path
from unittest.mock import patch

import preview_release as release


class PreviewVersionTests(unittest.TestCase):
    def test_initial_version_uses_workspace_base(self):
        self.assertEqual(release.next_tag(None, ["feat: flight"], "0.1.0"), "v0.1.0-preview.1")

    def test_conventional_commit_precedence_and_scopes(self):
        for messages, expected in [
            (["docs: controls"], "v2.3.4-preview.3"),
            (["fix(save): preserve a missile"], "v2.3.5-preview.1"),
            (["perf: identity fast path"], "v2.3.5-preview.1"),
            (["fix: bug", "feat(market): finite stock"], "v2.4.0-preview.1"),
            (["feat!: replace save format"], "v3.0.0-preview.1"),
            (["refactor(core)!: remove legacy API"], "v3.0.0-preview.1"),
            (["feat: reader\n\nBREAKING CHANGE: remove old format"], "v3.0.0-preview.1"),
            (["fix: reader\n\nBREAKING-CHANGE: replace API"], "v3.0.0-preview.1"),
            (["docs: explain feat: and BREAKING CHANGE: syntax"], "v2.3.4-preview.3"),
        ]:
            with self.subTest(messages=messages):
                self.assertEqual(release.next_tag("v2.3.4-preview.2", messages, "0.1.0"), expected)

    def test_stable_or_unrelated_tag_cannot_be_version_baseline(self):
        for tag in ["v1.0.0", "other-preview", "v1.0.0-preview.0"]:
            with self.assertRaises(ValueError):
                release.next_tag(tag, [], "0.1.0")

    def test_retention_uses_publication_order_and_preserves_nonpreview_releases(self):
        def row(identifier, tag, date, prerelease=True, draft=False):
            return dict(id=identifier, tag_name=tag, published_at=date,
                        prerelease=prerelease, draft=draft)
        rows = [row(1, "v0.1.0-preview.1", "2026-11-10"),
                row(2, "v0.2.0-preview.1", "2026-11-11"),
                row(3, "v0.3.0-preview.1", "2026-11-12"),
                row(4, "v0.4.0-preview.1", "2026-11-13"),
                row(5, "v1.0.0", "2026-11-14", False),
                row(6, "unrelated", "2026-11-15"),
                row(7, "v0.5.0-preview.1", None, draft=True)]
        managed = release.previews(rows)
        self.assertEqual([r["id"] for r in managed[:3]], [4, 3, 2])
        self.assertEqual([r["id"] for r in managed[3:]], [1])

    def test_download_digest_mismatch_stops_publication(self):
        def download(*args):
            destination = Path(args[args.index("--dir") + 1])
            (destination / "mac.zip").write_bytes(b"altered")
            return ""
        with patch.object(release, "run", side_effect=download):
            with self.assertRaisesRegex(ValueError, "mac.zip"):
                release.verify_downloads("v0.1.0-preview.1", {"mac.zip": "0" * 64})

    def test_downloads_all_match_before_success(self):
        def download(*args):
            destination = Path(args[args.index("--dir") + 1])
            (destination / "mac.zip").write_bytes(b"qualified")
            return ""
        with tempfile.TemporaryDirectory() as temporary:
            artifact = Path(temporary) / "original.zip"
            artifact.write_bytes(b"qualified")
            with patch.object(release, "run", side_effect=download):
                release.verify_downloads("v0.1.0-preview.1", {"mac.zip": release.digest(artifact)})


class PublicationOrderTests(unittest.TestCase):
    def exercise(self, failure=None):
        events = []
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in ["mac.zip", "win.zip", "notes.md"]:
                (root / name).write_bytes(b"qualified")
            args = argparse.Namespace(commit="a" * 40, macos=root / "mac.zip",
                                      windows=root / "win.zip", notes=root / "notes.md",
                                      receipt=root / "receipt.json", plan=False)
            def api(path, *fields):
                events.append(("api", path))
                if "/compare/" in path:
                    return {"status": "ahead"}
                if "/check-runs" in path:
                    return {"check_runs": [dict(id=i, name=n, conclusion="success")
                            for i, n in enumerate(["Rust (ubuntu-latest)", "Rust (windows-latest)",
                                                  "Rust (macos-latest)", "Documentation",
                                                  "Coverage reports", "SonarQube (main)"])]}
                if "/git/tags" in path:
                    return {"sha": "b" * 40}
                if "/releases/tags/" in path:
                    return {"html_url": "https://example.test/release"}
                return {}
            def run(*args):
                events.append(args)
                if args[:2] == ("git", "log"):
                    return "feat: flight\0"
                if args[:2] == ("git", "show"):
                    return '[workspace.package]\nversion = "0.1.0"'
                if args[:3] == ("gh", "release", "delete"):
                    self.assertTrue((root / "receipt.json").is_file())
                return ""
            def verify(tag, expected):
                events.append(("download-verified", tag))
                if failure == "download":
                    raise ValueError("download mismatch")
            def signature(archive):
                events.append(("signature-verified", archive))
                if failure == "signature":
                    raise ValueError("invalid signature")
            rows = [dict(id=i, tag_name=f"v0.1.0-preview.{i}", draft=False,
                         prerelease=True, published_at=f"2026-11-{10+i}") for i in range(1, 5)]
            with patch.object(release, "api", side_effect=api), \
                 patch.object(release, "run", side_effect=run), \
                 patch.object(release, "releases", side_effect=[[], rows]), \
                 patch.object(release, "verify_downloads", side_effect=verify), \
                 patch.object(release, "verify_macos", side_effect=signature), \
                 contextlib.redirect_stdout(io.StringIO()):
                if failure:
                    with self.assertRaises(ValueError):
                        release.publish(args)
                else:
                    release.publish(args)
        return events

    def test_publish_verifies_twice_and_saves_receipt_before_pruning(self):
        events = self.exercise()
        verifies = [i for i, event in enumerate(events) if event[0] == "download-verified"]
        edit = next(i for i, e in enumerate(events) if e[:3] == ("gh", "release", "edit"))
        deletion = next(i for i, e in enumerate(events) if e[:3] == ("gh", "release", "delete"))
        self.assertLess(verifies[0], edit)
        self.assertLess(edit, verifies[1])
        self.assertLess(verifies[1], deletion)
        self.assertEqual(events[deletion][3], "v0.1.0-preview.1")
        self.assertIn("--cleanup-tag", events[deletion])

    def test_bad_signature_creates_neither_tag_nor_release(self):
        events = self.exercise("signature")
        self.assertFalse(any("/git/tags" in str(e) for e in events))
        self.assertFalse(any(e[:2] == ("gh", "release") for e in events))

    def test_failed_download_does_not_publish_or_prune(self):
        events = self.exercise("download")
        self.assertFalse(any(e[:3] == ("gh", "release", "edit") for e in events))
        self.assertFalse(any(e[:3] == ("gh", "release", "delete") for e in events))


if __name__ == "__main__":
    unittest.main()
