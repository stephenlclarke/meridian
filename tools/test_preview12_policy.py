"""Offline positive and refusal cases for the explicitly scoped Preview 12 gate."""
import argparse
import contextlib
import hashlib
import io
import json
import os
import plistlib
import stat
import struct
import subprocess
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest.mock import patch

import preview12_policy as policy
import preview_release as release

SOURCE, TREE, COMPILED = "a" * 40, "b" * 40, "c" * 40
MEDUSA_SCRIPT = (Path(__file__).parent.parent / "integration/sh171-medusa-preview/Launch Medusa Preview.cmd").read_text()
FILE = stat.S_IFREG | 0o644
EXEC = stat.S_IFREG | 0o755
DIRECTORY = stat.S_IFDIR | 0o755
LINK = stat.S_IFLNK | 0o777
APPLEDOUBLE = b"\x00\x05\x16\x07\x00\x02\x00\x00" + bytes(18)


def encoded(value):
    return json.dumps(value).encode()


def digest(data):
    return hashlib.sha256(data).hexdigest()


def pe():
    data = bytearray(90)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 0x3C, 64)
    data[64:68] = b"PE\0\0"
    struct.pack_into("<H", data, 68, 0x8664)
    struct.pack_into("<H", data, 88, 0x20B)
    return bytes(data)


def archive(path, files):
    with zipfile.ZipFile(path, "w") as bundle:
        for relative, (mode, data) in files.items():
            name = relative if relative.startswith("__MACOSX/") else f"{policy.ROOT}/{relative}"
            info = zipfile.ZipInfo(name)
            info.create_system = 3
            info.external_attr = mode << 16
            bundle.writestr(info, data)
    with zipfile.ZipFile(path) as bundle:
        members = []
        for info in bundle.infolist():
            mode = info.external_attr >> 16
            kind = policy._member_kind(info.filename, mode, info.file_size)
            members.append({"path": info.filename, "kind": kind, "mode": mode, "size": info.file_size,
                            "sha256": None if info.is_dir() else digest(bundle.read(info))})
    return {"archiveSha256": digest(path.read_bytes()), "compiledSource": COMPILED, "members": members}


def notice_inventory(files, platform):
    content = b"approved source notice"
    inventory = {"status": "EXACT_SOURCE_INVENTORY", "sourceTrees": {SOURCE: TREE}, "noticeCount": 1,
                 "files": {"LICENSE": {"bytes": len(content), "sha256": digest(content)}}}
    files["SOURCE-NOTICE-INVENTORY.json"] = (FILE, encoded(inventory))
    prefixes = [f"{app}/Contents/Resources/Notices/" for app in (policy.FLEET, policy.MEDUSA, policy.SHOWROOM)] if platform == "macos" else ["notices/"]
    for prefix in prefixes:
        files[prefix + "LICENSE"] = FILE, content


def runtime_manifest(files, platform, paths):
    mac = platform == "macos"
    studio = policy.STUDIO if mac else policy.WINDOWS_STUDIO
    runtime = []
    for path in paths:
        mode, data = files[path]
        runtime.append({"path": path, "kind": "file", "bytes": len(data), "sha256": digest(data), "mode": mode,
                        "componentId": "python", "sourceArtifact": "pinned-python", "noticePaths": ["notices/LICENSE"]})
    manifest = {"schema": 1, "protectedSource": SOURCE, "sourceTree": TREE, "compiledSource": COMPILED,
                "platform": platform, "target": "aarch64-apple-darwin" if mac else "x86_64-pc-windows-msvc",
                "studioRoot": studio, "buildReceiptSHA256": "d" * 64, "runtimeFiles": runtime,
                "components": [{"id": "python"}]}
    files["STUDIO-RUNTIME-MANIFEST.json"] = FILE, encoded(manifest)


def mac_files():
    files = {"Read Me.txt": (FILE, b"read me"), "Documentation/README.md": (FILE, b"docs"),
             "Source/SOURCE-FILES.json": (FILE, b"{}")}
    apps = {}
    for index, (app, (identifier, executable)) in enumerate(policy._bundle_specs(COMPILED).items(), 1):
        info = {"CFBundleIdentifier": identifier, "CFBundlePackageType": "APPL", "CFBundleExecutable": executable}
        if app in (policy.FLEET, policy.MEDUSA):
            info.update(MeridianSourceRevision=SOURCE, MeridianSourceTree=TREE, MeridianCompiledRevision=COMPILED,
                        MeridianSaveNamespace=policy._namespace(app, COMPILED))
            files[f"{app}/Contents/MacOS/Meridian"] = EXEC, b"game"
        if app != policy.SHOWROOM:
            apps[app] = {"bundleIdentifier": identifier, "appleNotaryStatus": "Accepted",
                         "appleNotarySubmissionID": f"00000000-0000-0000-0000-{index:012d}"}
        files[f"{app}/Contents/Info.plist"] = FILE, plistlib.dumps(info)
        files[f"{app}/Contents/MacOS/{executable}"] = EXEC, b"executable"
        files[f"{app}/Contents/_CodeSignature/CodeResources"] = FILE, b"signature"
    files["BUILD-IDENTITY.json"] = FILE, encoded({"protectedReleaseSource": SOURCE, "protectedSourceTree": TREE,
        "compiledSource": COMPILED, "version": "0.12.0", "build": 12, "apps": apps})
    notice_inventory(files, "macos")
    runtime = f"{policy.STUDIO}/Contents/Frameworks/libpython.dylib"
    files[runtime] = EXEC, b"library"
    runtime_manifest(files, "macos", [runtime])
    files[f"__MACOSX/{policy.ROOT}/._Read Me.txt"] = FILE, APPLEDOUBLE
    return files


def windows_files():
    ordinary = f"saves-preview12-{SOURCE[:8]}"
    files = {name: (EXEC, pe()) for name in policy.WINDOWS_EXECUTABLES}
    files.update({f"{ordinary}/": (DIRECTORY, b""), "saves-medusa-preview/": (DIRECTORY, b""),
        "Launch Meridian.cmd": (FILE, policy._windows_launcher(f'"%~dp0bin\\meridian-desktop.exe" --save-dir "%~dp0{ordinary}"')),
        "Launch Hull Studio.cmd": (FILE, policy._windows_launcher(f'"%~dp0{policy.WINDOWS_STUDIO}\\Meridian Hull Studio.exe"')),
        "Launch Medusa Preview.cmd": (FILE, policy._launcher_bytes(MEDUSA_SCRIPT.splitlines())),
        "SOURCE-IDENTITY.json": (FILE, encoded({"protectedMain": SOURCE, "gitTree": TREE, "compiledSource": COMPILED,
            "profileDirectory": ordinary, "medusaProfileDirectory": "saves-medusa-preview"}))})
    notice_inventory(files, "windows")
    runtime = sorted(policy.WINDOWS_NATIVE_RUNTIME)
    files.update({name: (EXEC, pe()) for name in runtime})
    runtime_manifest(files, "windows", runtime)
    return files


def run(*args):
    if args[:2] == ("git", "rev-parse"):
        return TREE
    if args[:2] == ("git", "show"):
        return MEDUSA_SCRIPT.strip()
    if args[:3] == ("/usr/bin/ditto", "-x", "-k"):
        with zipfile.ZipFile(args[3]) as bundle:
            for info in bundle.infolist():
                if info.filename.startswith("__MACOSX/"):
                    continue
                target = Path(args[4]) / info.filename
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(bundle.read(info))
                target.chmod(stat.S_IMODE(info.external_attr >> 16))
        return ""
    raise AssertionError(args)


def signature(*args):
    if args[1] == "-dvv":
        app = Path(args[-1])
        identifier = plistlib.loads((app / "Contents/Info.plist").read_bytes())["CFBundleIdentifier"]
        return f"Identifier={identifier}\nTeamIdentifier={policy.TEAM}\nAuthority={policy.AUTHORITY}\nflags=0x10000(runtime)"
    if args[0] == policy.GATEKEEPER:
        return "accepted\nsource=Notarized Developer ID"
    return "ok"


class Preview12PolicyTests(unittest.TestCase):
    def test_complete_mac_and_windows_layout_with_individual_signatures_tickets(self):
        with tempfile.TemporaryDirectory() as folder:
            mac, windows = Path(folder) / release.PACKAGE_FILES[0], Path(folder) / release.PACKAGE_FILES[1]
            mac_approval, windows_approval = archive(mac, mac_files()), archive(windows, windows_files())
            approval = {"schema": 1, "source": SOURCE, "tree": TREE, "platforms": {"macos": mac_approval, "windows": windows_approval}}
            data = encoded(approval)
            (mac.parent / policy.APPROVAL).write_bytes(data)
            # Windows hosts cannot reproduce Unix permission bits in mocked ditto extraction.
            extraction = patch.object(policy, "_extract_inventory") if os.name == "nt" else contextlib.nullcontext()
            with extraction, patch.object(policy, "_checked_output", side_effect=signature) as checks:
                actual_sha, hashes = policy.verify(mac, windows, SOURCE, run)
            self.assertEqual(actual_sha, digest(data))
            self.assertEqual(hashes, {mac.name: mac_approval["archiveSha256"], windows.name: windows_approval["archiveSha256"]})
            self.assertEqual(sum(call.args[1:4] == ("--verify", "--deep", "--strict") for call in checks.call_args_list), 4)
            self.assertEqual(sum(call.args[:3] == (policy.STAPLER, "stapler", "validate") for call in checks.call_args_list), 3)
            self.assertEqual(sum(call.args[0] == policy.GATEKEEPER for call in checks.call_args_list), 3)

    def test_mac_wrong_bundle_extra_app_and_missing_ticket_refuse_even_if_approved(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "mac.zip"
            files = mac_files()
            files["Unexpected.app/Contents/Info.plist"] = FILE, b"extra"
            approved = archive(path, files)
            with self.assertRaisesRegex(ValueError, "wrapper inventory"):
                policy.verify_macos(path, approved, SOURCE, TREE, run)
            files = mac_files()
            key = f"{policy.MEDUSA}/Contents/Info.plist"
            data = plistlib.loads(files[key][1]); data["CFBundleIdentifier"] = "wrong"
            files[key] = FILE, plistlib.dumps(data)
            approved = archive(path, files)
            with patch.object(policy, "_extract_inventory"), patch.object(policy, "_checked_output", side_effect=signature), self.assertRaisesRegex(ValueError, "plist identity"):
                policy.verify_macos(path, approved, SOURCE, TREE, run)
            approved = archive(path, mac_files())
            def bad_ticket(*args):
                if args[0] == policy.STAPLER and args[-1].endswith(policy.MEDUSA):
                    raise subprocess.CalledProcessError(1, args)
                return signature(*args)
            with patch.object(policy, "_extract_inventory"), patch.object(policy, "_checked_output", side_effect=bad_ticket), self.assertRaises(subprocess.CalledProcessError):
                policy.verify_macos(path, approved, SOURCE, TREE, run)

    def test_mac_duplicate_notary_receipt_and_missing_standalone_notice_refuse(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "mac.zip"
            files = mac_files()
            record = json.loads(files["BUILD-IDENTITY.json"][1])
            record["apps"][policy.MEDUSA]["appleNotarySubmissionID"] = record["apps"][policy.FLEET]["appleNotarySubmissionID"]
            files["BUILD-IDENTITY.json"] = FILE, encoded(record)
            approved = archive(path, files)
            with patch.object(policy, "_extract_inventory"), patch.object(policy, "_checked_output", side_effect=signature), self.assertRaisesRegex(ValueError, "distinct notarization"):
                policy.verify_macos(path, approved, SOURCE, TREE, run)
            files = mac_files()
            del files[f"{policy.SHOWROOM}/Contents/Resources/Notices/LICENSE"]
            approved = archive(path, files)
            with self.assertRaisesRegex(ValueError, "Missing required Preview 12 file"):
                policy.verify_macos(path, approved, SOURCE, TREE, run)

    def test_windows_refuses_extra_runtime_wrong_launcher_and_non_x64(self):
        def add_unapproved_runtime_with_manifest(files):
            extra = f"{policy.WINDOWS_STUDIO}/_internal/VCRUNTIME140.dll"
            files[extra] = EXEC, pe()
            runtime_manifest(files, "windows", sorted(policy.WINDOWS_NATIVE_RUNTIME | {extra}))
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "win.zip"
            for change, reason in (
                (lambda f: f.update({"bin/extra.dll": (EXEC, pe())}), "DLL/PYD inventory"),
                (lambda f: f.update({f"{policy.WINDOWS_STUDIO}/_internal/unlisted.pyd": (EXEC, pe())}), "DLL/PYD inventory"),
                (add_unapproved_runtime_with_manifest, "DLL/PYD inventory"),
                (lambda f: f.update({"Launch Meridian.cmd": (FILE, b"wrong")}), "ordinary launcher"),
                (lambda f: f.update({"Launch Medusa Preview.cmd": (FILE, b"wrong")}), "Medusa launcher"),
                (lambda f: f.update({policy.WINDOWS_VALIDATOR: (EXEC, b"not PE")}), "invalid PE"),
                (lambda f: f.update({f"saves-preview12-{SOURCE[:8]}/unexpected.txt": (FILE, b"private")}), "empty isolated profile"),
            ):
                with self.subTest(reason=reason):
                    files = windows_files(); change(files)
                    approved = archive(path, files)
                    with self.assertRaisesRegex(ValueError, reason):
                        policy.verify_windows(path, approved, SOURCE, TREE, run)

    def test_approved_zip_digest_inventory_links_and_appledouble_are_exact(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "raw.zip"
            files = mac_files(); approved = archive(path, files)
            files["Documentation/extra.md"] = FILE, b"extra"
            replacement = archive(path, files)
            with self.assertRaisesRegex(ValueError, "archive differs"):
                policy.verify_inventory(path, "macos", approved)
            approved["archiveSha256"] = replacement["archiveSha256"]
            with self.assertRaisesRegex(ValueError, "raw ZIP inventory differs"):
                policy.verify_inventory(path, "macos", approved)
            target = f"{policy.STUDIO}/Contents/Resources/alias"
            files = mac_files(); files[target] = LINK, b"../Frameworks/libpython.dylib"
            approved = archive(path, files)
            _, links = policy.verify_inventory(path, "macos", approved)
            self.assertEqual(links[f"{policy.ROOT}/{target}"], "../Frameworks/libpython.dylib")
            for bad in (b"../../../outside", b"/absolute"):
                files[target] = LINK, bad
                approved = archive(path, files)
                with self.assertRaisesRegex(ValueError, "symlink"):
                    policy.verify_inventory(path, "macos", approved)
            files = mac_files(); files[f"__MACOSX/{policy.ROOT}/._orphan"] = FILE, APPLEDOUBLE
            approved = archive(path, files)
            with self.assertRaisesRegex(ValueError, "Unpaired AppleDouble"):
                policy.verify_inventory(path, "macos", approved)

    def test_preview12_dispatch_refuses_before_any_release_mutation(self):
        args = argparse.Namespace(commit=SOURCE, package="qualified", plan=False)
        with patch.object(release, "plan", return_value=(policy.TAG, [])), patch.object(release, "package_files", return_value=[Path("mac"), Path("win"), Path("notes"), Path("receipt")]), \
             patch.object(release, "verify_checks"), patch.object(release, "verify_macos") as historical, patch.object(policy, "verify", side_effect=ValueError("approval refused")), \
             patch.object(release, "create_draft") as draft, contextlib.redirect_stdout(io.StringIO()), self.assertRaisesRegex(ValueError, "approval refused"):
            release.publish(args)
        historical.assert_not_called(); draft.assert_not_called()

    def test_preview12_approval_is_source_bound_and_receipt_preserves_only_its_digest(self):
        with tempfile.TemporaryDirectory() as folder:
            parent = Path(folder)
            mac, win, notes = (parent / name for name in release.PACKAGE_FILES)
            for path in (mac, win, notes):
                path.write_bytes(b"qualified")
            receipt = parent / "release-receipt.json"
            sidecar = parent / policy.APPROVAL
            candidate = {"archiveSha256": "f" * 64, "compiledSource": COMPILED, "members": [{}]}
            record = {"schema": 1, "source": SOURCE, "tree": TREE, "platforms": {"macos": candidate, "windows": candidate}}
            sidecar.write_bytes(encoded(record))
            self.assertEqual(policy.load_approval(sidecar, SOURCE, run)[0], record)
            with self.assertRaisesRegex(ValueError, "approval source"):
                policy.load_approval(sidecar, "e" * 40, run)
            with self.assertRaisesRegex(ValueError, "compiler source"):
                policy.load_approval(sidecar, SOURCE, lambda *args: TREE if SOURCE in args[-1] else "e" * 40)
            hashes = {path.name: digest(path.read_bytes()) for path in (mac, win)}
            approval_sha = "e" * 64
            args = argparse.Namespace(commit=SOURCE, package="qualified", plan=False)
            with patch.object(release, "plan", return_value=(policy.TAG, [])), patch.object(release, "package_files", return_value=[mac, win, notes, receipt]), \
                 patch.object(release, "verify_checks"), patch.object(release, "verify_macos") as historical, patch.object(policy, "verify", return_value=(approval_sha, hashes)), \
                 patch.object(release, "create_draft") as draft, patch.object(release, "verify_downloads") as downloads, patch.object(release, "run"), \
                 patch.object(release, "api", return_value={"html_url": "https://example.test/release"}), patch.object(release, "releases", return_value=[]), contextlib.redirect_stdout(io.StringIO()):
                release.publish(args)
            historical.assert_not_called()
            self.assertEqual(draft.call_args.args[2], [mac, win])
            self.assertEqual(set(downloads.call_args.args[1]), {mac.name, win.name, "SHA256SUMS.txt", "preview.json"})
            self.assertEqual(json.loads(receipt.read_text())["preview12ApprovalSHA256"], approval_sha)
            mac.write_bytes(b"changed after approval")
            with patch.object(release, "plan", return_value=(policy.TAG, [])), patch.object(release, "package_files", return_value=[mac, win, notes, parent / "unused-receipt.json"]), \
                 patch.object(release, "verify_checks"), patch.object(policy, "verify", return_value=(approval_sha, hashes)), patch.object(release, "create_draft") as draft, \
                 contextlib.redirect_stdout(io.StringIO()), self.assertRaisesRegex(ValueError, "archive changed"):
                release.publish(args)
            draft.assert_not_called()


if __name__ == "__main__":
    unittest.main()
