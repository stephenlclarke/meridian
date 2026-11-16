"""Offline regressions for version selection, retention and downloaded bytes."""
import tempfile
import argparse
import contextlib
import io
import json
import subprocess
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
            (destination / release.PACKAGE_FILES[0]).write_bytes(b"altered")
            return ""
        with patch.object(release, "run", side_effect=download):
            with self.assertRaisesRegex(ValueError, "Meridian-macos-arm64.zip"):
                release.verify_downloads("v0.1.0-preview.1", {release.PACKAGE_FILES[0]: "0" * 64})

    def test_downloads_all_match_before_success(self):
        def download(*args):
            destination = Path(args[args.index("--dir") + 1])
            (destination / release.PACKAGE_FILES[0]).write_bytes(b"qualified")
            return ""
        with tempfile.TemporaryDirectory() as temporary:
            artifact = Path(temporary) / "original.zip"
            artifact.write_bytes(b"qualified")
            with patch.object(release, "run", side_effect=download):
                release.verify_downloads("v0.1.0-preview.1", {release.PACKAGE_FILES[0]: release.digest(artifact)})


class PublicationOrderTests(unittest.TestCase):
    def exercise(self, failure=None):
        events = []
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "qualified"
            package.mkdir()
            for name in release.PACKAGE_FILES:
                (package / name).write_bytes(b"qualified")
            args = argparse.Namespace(commit="a" * 40, package="qualified", plan=False)
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
                    self.assertTrue((package / "release-receipt.json").is_file())
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
            with patch.object(release, "STAGING_ROOT", root), \
                 patch.object(release, "api", side_effect=api), \
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


class InputBoundaryTests(unittest.TestCase):
    def test_commit_rejects_option_and_revision_injection(self):
        for value in ["--help", "a" * 40 + " --repo other/repo", "main", "a" * 39, "g" * 40]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                release.source_commit(value)
        self.assertEqual(release.source_commit("a" * 40), "a" * 40)

    def test_initial_version_rejects_file_content_used_as_cli_options(self):
        for value in ["--repo other/repo", "../../outside", "0.1.0\n--help"]:
            with self.assertRaises(ValueError):
                release.next_tag(None, [], value)

    def test_runner_rejects_unlisted_tools_and_keeps_argv_separate(self):
        with patch.object(release.subprocess, "check_output", return_value=" ok\n") as call:
            self.assertEqual(release.run("git", "log", "--format=%B%x00", "a" * 40), "ok")
            call.assert_called_once_with(["git", "log", "--format=%B%x00", "a" * 40], text=True)
        with self.assertRaises(ValueError):
            release.run("sh", "-c", "anything")

    def test_package_refuses_traversal_symlinks_missing_files_and_existing_receipt(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "qualified"
            package.mkdir()
            for name in release.PACKAGE_FILES:
                (package / name).write_text("qualified")
            with patch.object(release, "STAGING_ROOT", root):
                files = release.package_files("qualified")
                self.assertEqual([p.name for p in files[:3]], list(release.PACKAGE_FILES))
                for token in [None, "../outside", "/absolute", "--help", "a/b", "UPPERCASE"]:
                    with self.subTest(token=token), self.assertRaises(ValueError):
                        release.package_files(token)
                alias = root / "staging-alias"
                alias.symlink_to(root, target_is_directory=True)
                with patch.object(release, "STAGING_ROOT", alias), self.assertRaises(ValueError):
                    release.package_files("qualified")
                (root / "link").symlink_to(package, target_is_directory=True)
                with self.assertRaises(ValueError):
                    release.package_files("link")
                notes = package / "RELEASE-NOTES.md"
                notes.unlink()
                with self.assertRaises(ValueError):
                    release.package_files("qualified")
                outside = root / "private.txt"
                outside.write_text("do not read or publish")
                notes.symlink_to(outside)
                with self.assertRaises(ValueError):
                    release.package_files("qualified")
                notes.unlink()
                notes.write_text("public release notes")
                (package / "release-receipt.json").write_text("prior receipt")
                with self.assertRaises(ValueError):
                    release.package_files("qualified")

    def test_download_names_cannot_traverse_or_become_options(self):
        with patch.object(release, "run"):
            for name in ["../private", "/absolute", "--help"]:
                with self.assertRaises(ValueError):
                    release.verify_downloads("v0.1.0-preview.1", {name: "0" * 64})

    def test_notes_are_written_as_data_to_fixed_metadata_files(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            notes = "../../outside\n--repo other/repo"
            checksums, metadata, full_notes = release.write_metadata(
                root, "v0.1.0-preview.1", "a" * 40, {release.PACKAGE_FILES[0]: "b" * 64}, notes)
            self.assertEqual({p.name for p in root.iterdir()},
                             {"SHA256SUMS.txt", "preview.json", "release-notes.md"})
            self.assertEqual(json.loads(metadata.read_text())["source"], "a" * 40)
            self.assertTrue(full_notes.read_text().startswith(notes))
            self.assertIn(release.PACKAGE_FILES[0], checksums.read_text())
            with self.assertRaises(FileExistsError):
                release.write_metadata(root, "v0.1.0-preview.1", "a" * 40, {}, notes)


class SignatureAndPlanningTests(unittest.TestCase):
    def test_signature_checks_extracted_app_and_rejects_missing_multiple_and_adhoc(self):
        for count, identity, valid in [(0, "", False), (2, "", False),
                                       (1, "Signature=adhoc", False),
                                       (1, "Authority=Developer ID Application: Verified", True)]:
            def tool(*args):
                if args[0] == "/usr/bin/ditto":
                    for i in range(count):
                        (Path(args[-1]) / f"App{i}.app").mkdir()
                return ""
            with self.subTest(count=count, identity=identity), \
                 patch.object(release, "run", side_effect=tool) as invocation, \
                 patch.object(release.subprocess, "run", return_value=argparse.Namespace(stderr=identity)):
                if valid:
                    release.verify_macos(Path("qualified.zip"))
                    self.assertTrue(any(call.args[:4] == ("/usr/bin/codesign", "--verify", "--deep", "--strict")
                                        for call in invocation.call_args_list))
                else:
                    with self.assertRaises(ValueError):
                        release.verify_macos(Path("qualified.zip"))

    def test_exact_checks_use_latest_result_and_refuse_missing_or_failed_sonar(self):
        checks = [dict(id=i, name=name, conclusion="success")
                  for i, name in enumerate(release.REQUIRED_CHECKS)]
        with patch.object(release, "api", return_value={"check_runs": checks}):
            release.verify_checks("a" * 40)
        for extra in [[], [dict(id=100, name="SonarQube (main)", conclusion="failure")]]:
            selected = checks[:-1] if not extra else checks + extra
            with patch.object(release, "api", return_value={"check_runs": selected}), \
                 self.assertRaises(ValueError):
                release.verify_checks("a" * 40)

    def test_api_and_paginated_releases_decode_structured_output(self):
        with patch.object(release, "run", return_value='{"sha":"abc"}'):
            self.assertEqual(release.api("path"), {"sha": "abc"})
        with patch.object(release, "run", return_value='[[{"id":1}],[{"id":2}]]'):
            self.assertEqual(release.releases(), [{"id": 1}, {"id": 2}])

    def test_plan_uses_validated_baseline_and_conventional_commits(self):
        previous = dict(tag_name="v0.1.0-preview.1", prerelease=True, draft=False,
                        published_at="2026-11-10", id=1)
        def api(path):
            return {"status": "ahead"} if "/compare/" in path else {"sha": "b" * 40}
        def command(*args):
            if args[1] == "log":
                return "feat: equipment\0"
            if args[1] == "show":
                return '[workspace.package]\nversion = "0.1.0"'
            return ""
        with patch.object(release, "api", side_effect=api), \
             patch.object(release, "releases", return_value=[previous]), \
             patch.object(release, "run", side_effect=command) as invocation:
            self.assertEqual(release.plan("a" * 40), ("v0.2.0-preview.1", []))
            self.assertIn(unittest.mock.call("git", "merge-base", "--is-ancestor", "b" * 40, "a" * 40),
                          invocation.call_args_list)
        for status in ["behind", "diverged"]:
            with patch.object(release, "api", return_value={"status": status}), self.assertRaises(ValueError):
                release.plan("a" * 40)
        for baseline in ["a" * 40, "--help"]:
            with patch.object(release, "api", side_effect=[{"status": "ahead"}, {"sha": baseline}]), \
                 patch.object(release, "releases", return_value=[previous]), \
                 patch.object(release, "run"), self.assertRaises(ValueError):
                release.plan("a" * 40)

    def test_cli_plan_does_not_read_packages_or_publish(self):
        with patch.object(release, "plan", return_value=("v0.1.0-preview.1", [])), \
             patch.object(release, "package_files") as packages, contextlib.redirect_stdout(io.StringIO()):
            release.main(["--commit", "a" * 40, "--plan"])
            packages.assert_not_called()
        with patch.object(release, "publish") as publish:
            release.main(["--commit", "a" * 40, "--package", "qualified"])
            self.assertEqual(publish.call_args.args[0].package, "qualified")


if __name__ == "__main__":
    unittest.main()
