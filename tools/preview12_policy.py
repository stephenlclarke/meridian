"""Strict, independently inventoried Preview 12 archive gate (not a packager)."""
import hashlib
import json
import os
import plistlib
import re
import stat
import struct
import subprocess
import tempfile
import unicodedata
import uuid
import zipfile
from pathlib import Path, PurePosixPath


TAG = "v0.12.0-preview.1"
APPROVAL = "PREVIEW12-APPROVED-INVENTORY.json"
ROOT = "Meridian Preview 12"
FLEET = "Meridian Fleet Preview.app"
MEDUSA = "Meridian Medusa Preview.app"
STUDIO = "Meridian Hull Studio.app"
SHOWROOM = f"{STUDIO}/Contents/Helpers/Meridian Hull Showroom.app"
MAC_VALIDATOR = f"{STUDIO}/Contents/Frameworks/meridian-hull-tool"
CODESIGN = "/usr/bin/codesign"
STAPLER = "/usr/bin/xcrun"
GATEKEEPER = "/usr/sbin/spctl"
TEAM = "4MEB7MUTAV"
AUTHORITY = "Developer ID Application: Steve Clarke (4MEB7MUTAV)"
MAC_TOP = {FLEET, MEDUSA, STUDIO, "Read Me.txt", "BUILD-IDENTITY.json", "Documentation",
           "Source", "STUDIO-RUNTIME-MANIFEST.json", "SOURCE-NOTICE-INVENTORY.json"}
WINDOWS_STUDIO = "Meridian Hull Studio"
WINDOWS_GAME = "bin/meridian-desktop.exe"
WINDOWS_STUDIO_EXE = f"{WINDOWS_STUDIO}/Meridian Hull Studio.exe"
WINDOWS_VALIDATOR = f"{WINDOWS_STUDIO}/_internal/meridian-hull-tool.exe"
WINDOWS_SHOWROOM = f"{WINDOWS_STUDIO}/_internal/meridian-desktop.exe"
WINDOWS_EXECUTABLES = {WINDOWS_GAME, WINDOWS_STUDIO_EXE, WINDOWS_VALIDATOR, WINDOWS_SHOWROOM}
# Exact measured CPython 3.13 / PyInstaller onedir native runtime from Dell's
# Preview 12 candidate freeze. VCRUNTIME140.dll is external, not redistributed.
WINDOWS_NATIVE_RUNTIME = {
    f"{WINDOWS_STUDIO}/_internal/{name}" for name in (
        "_bz2.pyd", "_ctypes.pyd", "_decimal.pyd", "_hashlib.pyd", "_lzma.pyd",
        "_socket.pyd", "_ssl.pyd", "libcrypto-3-x64.dll", "libffi-8.dll",
        "libssl-3-x64.dll", "python313.dll", "select.pyd", "unicodedata.pyd",
    )
}
MAX_ENTRIES = 20_000
MAX_EXPANDED_BYTES = 2 * 1024**3
MAX_ENTRY_BYTES = 512 * 1024**2
MAX_APPROVAL_BYTES = 10 * 1024**2


def _hex(value, length):
    if not isinstance(value, str) or not re.fullmatch(rf"[0-9a-f]{{{length}}}", value):
        raise ValueError("Invalid Preview 12 source or digest")
    return value


def _digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def _unique_object(pairs):
    item = {}
    for key, value in pairs:
        if key in item:
            raise ValueError(f"Duplicate Preview 12 JSON key: {key}")
        item[key] = value
    return item


def _json(data):
    return json.loads(data, object_pairs_hook=_unique_object)


def load_approval(path, commit, run):
    if path.is_symlink() or not path.is_file() or path.stat().st_size > MAX_APPROVAL_BYTES:
        raise ValueError("Preview 12 requires its independent approved inventory")
    data = path.read_bytes()
    record = _json(data)
    if not isinstance(record, dict) or set(record) != {"schema", "source", "tree", "platforms"} or record["schema"] != 1:
        raise ValueError("Invalid Preview 12 approval schema")
    if _hex(record["source"], 40) != commit:
        raise ValueError("Preview 12 approval source differs from release source")
    tree = _hex(record["tree"], 40)
    if run("git", "rev-parse", f"{commit}^{{tree}}") != tree:
        raise ValueError("Preview 12 approval tree differs from release tree")
    if not isinstance(record["platforms"], dict) or set(record["platforms"]) != {"macos", "windows"}:
        raise ValueError("Preview 12 approval requires both platforms")
    for platform, entry in record["platforms"].items():
        if not isinstance(entry, dict) or set(entry) != {"archiveSha256", "compiledSource", "members"}:
            raise ValueError(f"Invalid Preview 12 {platform} approval schema")
        _hex(entry["archiveSha256"], 64)
        compiled = _hex(entry["compiledSource"], 40)
        if run("git", "rev-parse", f"{compiled}^{{tree}}") != tree:
            raise ValueError(f"Preview 12 {platform} compiler source differs from release tree")
        if not isinstance(entry["members"], list) or not 0 < len(entry["members"]) <= MAX_ENTRIES:
            raise ValueError(f"Invalid Preview 12 {platform} member approval")
    return record, hashlib.sha256(data).hexdigest()


def _member_name(name):
    if not isinstance(name, str) or "\\" in name or "\0" in name or ":" in name:
        raise ValueError(f"Unsafe Preview 12 archive path: {name!r}")
    text = name.removesuffix("/")
    parts = text.split("/")
    if any(part in ("", ".", "..") or any(ord(c) < 32 for c in part) for part in parts):
        raise ValueError(f"Unsafe Preview 12 archive path: {name!r}")
    return parts


def _member_kind(name, mode, size):
    file_type = stat.S_IFMT(mode)
    if name.endswith("/") and file_type in (0, stat.S_IFDIR) and size == 0:
        return "directory"
    if not name.endswith("/") and file_type in (0, stat.S_IFREG):
        return "file"
    if not name.endswith("/") and file_type == stat.S_IFLNK:
        return "symlink"
    raise ValueError(f"Unsupported Preview 12 ZIP member type: {name}")


def _approved_members(rows):
    expected = {}
    for row in rows:
        if not isinstance(row, dict) or set(row) != {"path", "kind", "size", "mode", "sha256"}:
            raise ValueError("Invalid Preview 12 approved member fields")
        name = row["path"]
        _member_name(name)
        size, mode = row["size"], row["mode"]
        if type(size) is not int or type(mode) is not int or not 0 <= size <= MAX_ENTRY_BYTES or not 0 <= mode <= 0xFFFF:
            raise ValueError(f"Invalid Preview 12 approved member size/mode: {name}")
        kind = _member_kind(name, mode, size)
        if kind != row["kind"] or (kind == "directory" and row["sha256"] is not None):
            raise ValueError(f"Invalid Preview 12 approved member type: {name}")
        if kind != "directory":
            _hex(row["sha256"], 64)
        if name in expected:
            raise ValueError(f"Duplicate Preview 12 approved member: {name}")
        expected[name] = row
    return expected


def _check_logical_path(name, platform):
    parts = _member_name(name)
    if parts[0] == "__MACOSX":
        if platform != "macos" or (len(parts) > 1 and parts[1] not in {ROOT, "._" + ROOT}):
            raise ValueError(f"Unexpected AppleDouble location: {name}")
        return
    if parts[0] != ROOT:
        raise ValueError(f"Unexpected Preview 12 wrapper: {name}")
    if platform == "windows" and any(part.endswith((".", " ")) or re.fullmatch(r"(?:con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\..*)?", part, re.I) for part in parts):
        raise ValueError(f"Ambiguous Windows Preview 12 path: {name}")
    lower = parts[-1].casefold()
    if lower.endswith((".meridian", ".profraw")) or lower in {"session.lock", "medusa-preview.lock"}:
        raise ValueError(f"Private save/build data in Preview 12: {name}")


def _check_links(members, links):
    studio_root = [ROOT, STUDIO]
    logical = {name.removesuffix("/") for name in members if not name.startswith("__MACOSX/")}
    non_directories = {name for name, row in members.items() if row["kind"] != "directory"}
    for name in logical:
        for parent in PurePosixPath(name).parents:
            if parent.as_posix() in non_directories:
                raise ValueError(f"Archive member traverses a non-directory: {name}")
    for name, target in links.items():
        if name.split("/")[:2] != studio_root:
            raise ValueError(f"Preview 12 symlinks are only allowed inside Studio: {name}")
        pending = target.split("/")
        prefix = name.split("/")[:-1]
        expansions = 0
        while pending:
            part = pending.pop(0)
            if part in ("", "."):
                continue
            if part == "..":
                if len(prefix) <= len(studio_root):
                    raise ValueError(f"Preview 12 symlink escapes Studio: {name}")
                prefix.pop()
                continue
            prefix.append(part)
            current = "/".join(prefix)
            if current in links:
                prefix.pop()
                pending = links[current].split("/") + pending
                expansions += 1
                if expansions > 40:
                    raise ValueError(f"Preview 12 symlink cycle: {name}")
        resolved = "/".join(prefix)
        if resolved not in logical and not any(path.startswith(resolved + "/") for path in logical):
            raise ValueError(f"Preview 12 symlink has no packaged target: {name}")


def _check_appledouble(members):
    logical = {name.removesuffix("/") for name in members if not name.startswith("__MACOSX/")}
    for name, row in members.items():
        if not name.startswith("__MACOSX/") or row["kind"] == "directory":
            continue
        parts = _member_name(name)[1:]
        if not parts[-1].startswith("._") or row["kind"] != "file":
            raise ValueError(f"Unexpected AppleDouble payload: {name}")
        counterpart = "/".join([*parts[:-1], parts[-1][2:]])
        if counterpart not in logical and not any(path.startswith(counterpart + "/") for path in logical):
            raise ValueError(f"Unpaired AppleDouble payload: {name}")


def verify_inventory(archive, platform, approval):
    if _digest(archive) != approval["archiveSha256"]:
        raise ValueError(f"Preview 12 {platform} archive differs from independent approval")
    expected = _approved_members(approval["members"])
    actual, links, folded, folded_prefixes = {}, {}, set(), {}
    with zipfile.ZipFile(archive) as bundle:
        infos = bundle.infolist()
        if len(infos) != len(expected) or sum(info.file_size for info in infos) > MAX_EXPANDED_BYTES:
            raise ValueError(f"Preview 12 {platform} raw ZIP inventory differs or exceeds limit")
        for info in infos:
            name = info.filename
            if info.orig_filename != name or info.flag_bits & 1:
                raise ValueError(f"Truncated or encrypted Preview 12 ZIP member: {name}")
            _check_logical_path(name, platform)
            key = unicodedata.normalize("NFC", name.removesuffix("/")).casefold()
            if key in folded:
                raise ValueError(f"Duplicate or case-colliding Preview 12 ZIP path: {name}")
            folded.add(key)
            leaf = PurePosixPath(name.removesuffix("/"))
            for parent in (leaf, *leaf.parents):
                if parent.as_posix() == ".":
                    continue
                parent_key = unicodedata.normalize("NFC", parent.as_posix()).casefold()
                if folded_prefixes.setdefault(parent_key, parent.as_posix()) != parent.as_posix():
                    raise ValueError(f"Case-colliding Preview 12 ZIP parent: {name}")
            mode = info.external_attr >> 16
            kind = _member_kind(name, mode, info.file_size)
            row = expected.get(name)
            if row is None or (kind, mode, info.file_size) != (row["kind"], row["mode"], row["size"]):
                raise ValueError(f"Preview 12 unapproved ZIP member: {name}")
            if kind == "symlink":
                if platform != "macos" or info.file_size > 4096:
                    raise ValueError(f"Unexpected Preview 12 symlink: {name}")
                data = bundle.read(info)
                target = data.decode("utf-8")
                if not target or target.startswith("/") or "\\" in target or "\0" in target or ":" in target:
                    raise ValueError(f"Unsafe Preview 12 symlink target: {name}")
                links[name] = target
                sha = hashlib.sha256(data).hexdigest()
            elif kind == "file":
                with bundle.open(info) as stream:
                    sha = hashlib.file_digest(stream, "sha256").hexdigest()
                if name.startswith("__MACOSX/"):
                    with bundle.open(info) as stream:
                        if stream.read(8) != b"\x00\x05\x16\x07\x00\x02\x00\x00" or info.file_size < 26:
                            raise ValueError(f"Invalid Preview 12 AppleDouble header: {name}")
            else:
                sha = None
            if sha != row["sha256"]:
                raise ValueError(f"Preview 12 ZIP member digest differs: {name}")
            actual[name] = row
    if actual.keys() != expected.keys():
        raise ValueError(f"Preview 12 {platform} missing approved ZIP members")
    _check_links(actual, links)
    if platform == "macos":
        _check_appledouble(actual)
    return actual, links


def _files(members):
    return {name.removeprefix(ROOT + "/"): row for name, row in members.items()
            if name.startswith(ROOT + "/") and row["kind"] != "directory"}


def _required_file(files, path):
    if path not in files or files[path]["kind"] != "file":
        raise ValueError(f"Missing required Preview 12 file: {path}")
    return files[path]


def _bundle_specs(compiled):
    short = compiled[:8]
    return {
        FLEET: (f"io.xyzzy.meridian.preview12.{short}", "LaunchMeridian"),
        MEDUSA: (f"io.xyzzy.meridian.medusa-preview.{short}", "LaunchMedusa"),
        STUDIO: ("io.xyzzy.meridian.hullstudio", "Meridian Hull Studio"),
        SHOWROOM: ("io.xyzzy.meridian.hullstudio.showroom", "meridian-desktop"),
    }


def _namespace(app, compiled):
    prefix = "saves-v17-" if app == FLEET else "medusa-preview-v17-"
    return f"{ROOT}/{prefix}{compiled[:8]}"


def _verify_mac_layout(members, compiled):
    files = _files(members)
    top = {name.split("/", 1)[0] for name in files}
    for name in members:
        if name.startswith(ROOT + "/") and name != ROOT + "/":
            top.add(name.removeprefix(ROOT + "/").split("/", 1)[0])
    if top != MAC_TOP:
        raise ValueError("Preview 12 Mac wrapper inventory differs from approved layout")
    actual_bundles = set()
    for name in members:
        if not name.startswith(ROOT + "/"):
            continue
        parts = _member_name(name)[1:]
        for index, part in enumerate(parts):
            if part.casefold().endswith(".app"):
                actual_bundles.add("/".join(parts[:index + 1]))
    if actual_bundles != set(_bundle_specs(compiled)):
        raise ValueError("Preview 12 Mac bundle set differs from approved layout")
    for path, (_, executable) in _bundle_specs(compiled).items():
        for suffix in ("Contents/Info.plist", f"Contents/MacOS/{executable}", "Contents/_CodeSignature/CodeResources"):
            _required_file(files, f"{path}/{suffix}")
    for app in (FLEET, MEDUSA):
        _required_file(files, f"{app}/Contents/MacOS/Meridian")
    _required_file(files, MAC_VALIDATOR)
    for name in ("Read Me.txt", "BUILD-IDENTITY.json", "STUDIO-RUNTIME-MANIFEST.json", "SOURCE-NOTICE-INVENTORY.json"):
        _required_file(files, name)


def _extract_inventory(scratch, members, links):
    expected = {name.removesuffix("/"): row for name, row in members.items()
                if not name.startswith("__MACOSX/")}
    expected_dirs = {name for name, row in expected.items() if row["kind"] == "directory"}
    for name in expected:
        expected_dirs.update(parent.as_posix() for parent in PurePosixPath(name).parents if parent.as_posix() != ".")
    expected_payload = {name: row for name, row in expected.items() if row["kind"] != "directory"}
    actual_dirs, actual_payload = set(), set()
    for directory, dirs, files in os.walk(scratch, followlinks=False):
        for path in (Path(directory) / entry for entry in dirs + files):
            name = path.relative_to(scratch).as_posix()
            mode = path.lstat().st_mode
            if stat.S_ISDIR(mode):
                actual_dirs.add(name)
                continue
            if name not in expected_payload:
                raise ValueError(f"Extracted Preview 12 Mac member was not approved: {name}")
            row = expected_payload[name]
            if stat.S_ISLNK(mode) and row["kind"] == "symlink":
                target = os.readlink(path)
                if target != links[name] or not path.resolve(strict=True).is_relative_to((scratch / ROOT / STUDIO).resolve(strict=True)):
                    raise ValueError(f"Extracted Preview 12 Mac symlink differs: {name}")
                data = os.fsencode(target)
                actual_size, actual_sha = len(data), hashlib.sha256(data).hexdigest()
            elif stat.S_ISREG(mode) and row["kind"] == "file":
                actual_size, actual_sha = path.stat().st_size, _digest(path)
            else:
                raise ValueError(f"Extracted Preview 12 Mac member type differs: {name}")
            if (actual_size, actual_sha) != (row["size"], row["sha256"]) or (row["mode"] and stat.S_IMODE(mode) != stat.S_IMODE(row["mode"])):
                raise ValueError(f"Extracted Preview 12 Mac member bytes/mode differ: {name}")
            actual_payload.add(name)
    if actual_dirs != expected_dirs or actual_payload != set(expected_payload):
        raise ValueError("Extracted Preview 12 Mac inventory differs from approved raw inventory")


def _checked_output(executable, *args):
    result = subprocess.run([executable, *args], check=True, text=True, capture_output=True)
    return result.stdout + result.stderr


def _verify_macho_header(header, name, filetypes):
    # A thin 64-bit Mach-O with CPU_TYPE_ARM64 and the expected native type. A matching
    # textual target, universal binary or signed x86_64 executable is not enough.
    if len(header) != 32 or struct.unpack_from("<II", header) != (0xFEEDFACF, 0x0100000C) or struct.unpack_from("<I", header, 12)[0] not in filetypes:
        raise ValueError(f"Preview 12 Mac native member is not thin arm64 Mach-O: {name}")


def _verify_macho_arm64(path):
    with path.open("rb") as stream:
        _verify_macho_header(stream.read(32), path.name, {2})


def _verify_mac_bundle(app, identifier, executable, top_level):
    with (app / "Contents/Info.plist").open("rb") as stream:
        info = plistlib.load(stream)
    if (info.get("CFBundleIdentifier"), info.get("CFBundlePackageType"), info.get("CFBundleExecutable")) != (identifier, "APPL", executable):
        raise ValueError(f"Preview 12 Mac bundle plist identity differs: {app.name}")
    native = app / "Contents/MacOS" / executable
    if not native.is_file():
        raise ValueError(f"Preview 12 Mac bundle executable missing: {app.name}")
    _verify_macho_arm64(native)
    _checked_output(CODESIGN, "--verify", "--deep", "--strict", str(app))
    signature = _checked_output(CODESIGN, "-dvv", str(app))
    lines = signature.splitlines()
    if (f"Identifier={identifier}" not in lines or f"TeamIdentifier={TEAM}" not in lines
            or f"Authority={AUTHORITY}" not in lines or "(runtime)" not in signature):
        raise ValueError(f"Preview 12 Mac bundle is not the approved hardened Developer ID: {app.name}")
    if top_level:
        _checked_output(STAPLER, "stapler", "validate", str(app))
        gate = _checked_output(GATEKEEPER, "--assess", "--type", "execute", "--verbose=4", str(app))
        if "accepted" not in gate or "Notarized Developer ID" not in gate:
            raise ValueError(f"Preview 12 Mac bundle lacks accepted notarized Gatekeeper identity: {app.name}")
    return info


def _verify_mac_identity(scratch, commit, tree, compiled):
    wrapper = scratch / ROOT
    identity = _json((wrapper / "BUILD-IDENTITY.json").read_bytes())
    if (identity.get("protectedReleaseSource"), identity.get("protectedSourceTree"), identity.get("compiledSource")) != (commit, tree, compiled) or identity.get("version") != "0.12.0" or identity.get("build") != 12:
        raise ValueError("Preview 12 Mac wrapper identity differs from release source")
    apps = identity.get("apps", {})
    if set(apps) != {FLEET, MEDUSA, STUDIO}:
        raise ValueError("Preview 12 Mac requires three individual accepted submission receipts")
    submission_ids = set()
    for name, (identifier, executable) in _bundle_specs(compiled).items():
        top = name != SHOWROOM
        info = _verify_mac_bundle(wrapper / name, identifier, executable, top)
        if top:
            record = apps[name]
            if record.get("bundleIdentifier") != identifier or record.get("appleNotaryStatus") != "Accepted":
                raise ValueError(f"Preview 12 Mac notarization receipt differs: {name}")
            try:
                submission = uuid.UUID(record.get("appleNotarySubmissionID", ""))
            except (ValueError, TypeError, AttributeError) as error:
                raise ValueError(f"Invalid Preview 12 Mac notarization submission ID: {name}") from error
            if submission in submission_ids:
                raise ValueError("Preview 12 Mac requires distinct notarization submissions")
            submission_ids.add(submission)
        if name in (FLEET, MEDUSA):
            if (info.get("MeridianSourceRevision"), info.get("MeridianSourceTree"), info.get("MeridianCompiledRevision")) != (commit, tree, compiled) or info.get("MeridianSaveNamespace") != _namespace(name, compiled):
                raise ValueError(f"Preview 12 Mac game source/save binding differs: {name}")
            _verify_macho_arm64(wrapper / name / "Contents/MacOS/Meridian")
    _verify_macho_arm64(wrapper / MAC_VALIDATOR)


def verify_macos(archive, approval, commit, tree, run, source_notices=None):
    members, links = verify_inventory(archive, "macos", approval)
    compiled = approval["compiledSource"]
    _verify_mac_layout(members, compiled)
    with zipfile.ZipFile(archive) as bundle:
        _verify_studio_runtime(bundle, members, "macos", commit, tree, compiled)
        _verify_source_notices(bundle, members, "macos", commit, tree, source_notices)
    with tempfile.TemporaryDirectory(prefix="meridian-preview12-signature-") as folder:
        run("/usr/bin/ditto", "-x", "-k", str(archive), folder)
        scratch = Path(folder)
        _extract_inventory(scratch, members, links)
        _verify_mac_identity(scratch, commit, tree, compiled)


def _read_member(bundle, name):
    return bundle.read(f"{ROOT}/{name}")


def _git_bytes(*args):
    return subprocess.check_output(["git", *args])


def _notice_location(path):
    item = PurePosixPath(path)
    parts = item.parts
    if parts[:2] == ("assets", "fleet") and len(parts) == 4 and (item.name == "LICENSE" or item.name.endswith((".json", ".toml", ".json.gz", ".toml.gz"))):
        return f"Fleet provenance/{parts[2].upper()}/{item.name}"
    if parts[:2] == ("assets", "legacy") and len(parts) == 3:
        if item.name == "Elite-L337-MIT.txt":
            return item.name
        if item.suffix == ".json":
            return f"Asset provenance/{item.name}"
    if path == "LICENSE":
        return "LICENSE"
    if path == "docs/LEGACY-REUSE.md":
        return "LEGACY-REUSE.md"
    if path in {"integration/unwritten-route/LICENSE", "integration/unwritten-route/SOURCE.json"}:
        return f"Original Unwritten Route provenance/{item.name}"
    if path == "crates/meridian-desktop/src/audio/source-provenance.json":
        return "Original audio provenance/SOURCE-PROVENANCE.json"
    if path == "crates/meridian-desktop/src/audio/fixtures/manifest.json":
        return "Original audio provenance/REFERENCE-WAVEFORMS.json"
    return None


def _expected_source_notices(commit):
    """Derive every current notice/provenance path and byte pin from protected Git blobs."""
    _hex(commit, 40)
    result = {}
    for entry in filter(None, _git_bytes("ls-tree", "-r", "-z", commit).split(b"\0")):
        header, raw_path = entry.split(b"\t", 1)
        mode, kind, blob = header.decode().split()
        path = raw_path.decode()
        destination = _notice_location(path)
        if destination is None:
            continue
        if kind != "blob" or mode != "100644" or destination in result:
            raise ValueError(f"Invalid or duplicated source notice location: {path}")
        data = _git_bytes("cat-file", "blob", _hex(blob, 40))
        result[destination] = {"sourcePath": path, "commit": commit, "blob": blob,
                               "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(), "mode": 0o644}
    hulls = {name.split("/")[1] for name in result if name.startswith("Fleet provenance/")}
    if not result or any(f"Fleet provenance/{hull}/LICENSE" not in result for hull in hulls):
        raise ValueError("Incomplete current source notice inventory in protected Git tree")
    return result


def _verify_source_notices(bundle, members, platform, commit, tree, source_notices=None):
    files = _files(members)
    name = "SOURCE-NOTICE-INVENTORY.json"
    _required_file(files, name)
    inventory = _json(_read_member(bundle, name))
    if inventory.get("status") != "EXACT_SOURCE_INVENTORY" or inventory.get("sourceTrees") != {commit: tree}:
        raise ValueError(f"Preview 12 {platform} source notice inventory differs from release source")
    notices = inventory.get("files", {})
    expected = _expected_source_notices(commit) if source_notices is None else source_notices
    hulls = sorted({name.split("/")[1] for name in expected if name.startswith("Fleet provenance/")})
    if notices != expected or inventory.get("noticeCount") != len(expected) or inventory.get("fleetHulls") != hulls or inventory.get("fleetHullCount") != len(hulls):
        raise ValueError(f"Preview 12 {platform} incomplete or changed current Git source notice inventory")
    prefixes = [f"{app}/Contents/Resources/Notices/" for app in (FLEET, MEDUSA, SHOWROOM)] if platform == "macos" else ["notices/"]
    for relative, pin in notices.items():
        _member_name(relative)
        for prefix in prefixes:
            row = _required_file(files, prefix + relative)
            if (row["size"], row["sha256"], stat.S_IMODE(row["mode"])) != (pin.get("bytes"), pin.get("sha256"), pin.get("mode")):
                raise ValueError(f"Preview 12 {platform} source notice differs: {prefix}{relative}")


def _runtime_mode(value):
    if isinstance(value, str) and re.fullmatch(r"(?:0o)?[0-7]+", value):
        return int(value.removeprefix("0o"), 8)
    if type(value) is int and 0 <= value <= 0xFFFF:
        return value
    raise ValueError("Invalid Preview 12 Studio runtime mode")


def _verify_studio_runtime(bundle, members, platform, commit, tree, compiled):
    files = _files(members)
    name = "STUDIO-RUNTIME-MANIFEST.json"
    _required_file(files, name)
    manifest = _json(_read_member(bundle, name))
    studio = STUDIO if platform == "macos" else WINDOWS_STUDIO
    target = "aarch64-apple-darwin" if platform == "macos" else "x86_64-pc-windows-msvc"
    if (manifest.get("schema"), manifest.get("protectedSource"), manifest.get("sourceTree"), manifest.get("compiledSource"), manifest.get("platform"), manifest.get("target"), manifest.get("studioRoot")) != (1, commit, tree, compiled, platform, target, studio):
        raise ValueError(f"Preview 12 {platform} Studio runtime source or root differs")
    if not manifest.get("runtimeFiles") or not manifest.get("components"):
        raise ValueError(f"Preview 12 {platform} Studio runtime inventory is empty")
    _hex(manifest.get("buildReceiptSHA256"), 64)
    runtime = {}
    for item in manifest["runtimeFiles"]:
        path = item.get("path")
        parts = _member_name(path)
        if parts[0] != studio or path in runtime:
            raise ValueError(f"Preview 12 {platform} unapproved Studio runtime path: {path}")
        runtime[path] = item
        kind = item.get("kind")
        raw_name = f"{ROOT}/{path.removesuffix('/')}" + ("/" if kind == "directory" else "")
        row = members.get(raw_name)
        if kind == "directory" and row is None and any(p.startswith(raw_name) for p in members):
            if item.get("bytes") == 0 and item.get("sha256") is None:
                continue  # implicit ZIP directory; actual contained paths are all approved
        if row is None or (row["kind"], row["size"], row["sha256"]) != (kind, item.get("bytes"), item.get("sha256")):
            raise ValueError(f"Preview 12 {platform} Studio runtime differs from approved archive: {path}")
        if stat.S_IMODE(_runtime_mode(item.get("mode"))) != stat.S_IMODE(row["mode"]):
            raise ValueError(f"Preview 12 {platform} Studio runtime mode differs: {path}")
        if kind == "symlink" and item.get("target") != bundle.read(raw_name).decode("utf-8"):
            raise ValueError(f"Preview 12 {platform} Studio runtime symlink target differs: {path}")
        if platform == "macos" and kind == "file":
            with bundle.open(raw_name) as stream:
                header = stream.read(32)
            macho_magic = {b"\xcf\xfa\xed\xfe", b"\xfe\xed\xfa\xcf", b"\xce\xfa\xed\xfe", b"\xfe\xed\xfa\xce", b"\xca\xfe\xba\xbe", b"\xbe\xba\xfe\xca", b"\xca\xfe\xba\xbf", b"\xbf\xba\xfe\xca"}
            if header[:4] in macho_magic or path.endswith((".dylib", ".so")) or (".framework/" in path and PurePosixPath(path).name == "Python"):
                _verify_macho_header(header, path, {2, 6, 8})
    if platform == "windows":
        actual = {path for path in files if path.casefold().endswith((".dll", ".pyd"))}
        declared = {path for path in runtime if path.casefold().endswith((".dll", ".pyd"))}
        if actual != WINDOWS_NATIVE_RUNTIME or declared != WINDOWS_NATIVE_RUNTIME:
            raise ValueError("Preview 12 Windows DLL/PYD inventory differs from approved Studio list")
        for path in actual:
            item = runtime[path]
            if not path.startswith(WINDOWS_STUDIO + "/_internal/") or not item.get("componentId") or not item.get("sourceArtifact") or not item.get("noticePaths"):
                raise ValueError(f"Preview 12 Windows runtime lacks exact Studio source/notice binding: {path}")
            for notice in item["noticePaths"]:
                _member_name(notice)
                _required_file(files, notice)
            _verify_pe(bundle, path)
    return runtime


def _verify_pe(bundle, path):
    with bundle.open(f"{ROOT}/{path}") as stream:
        first = stream.read(64)
        if len(first) != 64 or first[:2] != b"MZ":
            raise ValueError(f"Preview 12 Windows invalid PE: {path}")
        offset = struct.unpack_from("<I", first, 0x3C)[0]
        if not 64 <= offset <= 1024 * 1024:
            raise ValueError(f"Preview 12 Windows invalid PE offset: {path}")
        header = stream.read(offset - 64 + 26)[offset - 64:]
    if len(header) != 26 or header[:4] != b"PE\0\0" or struct.unpack_from("<H", header, 4)[0] != 0x8664 or struct.unpack_from("<H", header, 24)[0] != 0x20B:
        raise ValueError(f"Preview 12 Windows member is not x64 PE32+: {path}")


def _launcher_bytes(lines):
    return ("\r\n".join(lines) + "\r\n").encode("utf-8")


def _windows_launcher(command):
    return _launcher_bytes([
        "@echo off", "setlocal", 'pushd "%~dp0"', "if errorlevel 1 exit /b 1", command,
        'set "MERIDIAN_EXIT_CODE=%ERRORLEVEL%"', "popd", "exit /b %MERIDIAN_EXIT_CODE%",
    ])


def _verify_windows_identity(bundle, members, commit, tree, compiled, run):
    files = _files(members)
    for name in ("Launch Meridian.cmd", "Launch Medusa Preview.cmd", "Launch Hull Studio.cmd", "SOURCE-IDENTITY.json", "SOURCE-NOTICE-INVENTORY.json", "STUDIO-RUNTIME-MANIFEST.json"):
        _required_file(files, name)
    exe_paths = {name for name in files if name.casefold().endswith(".exe")}
    if exe_paths != WINDOWS_EXECUTABLES:
        raise ValueError("Preview 12 Windows executable inventory differs from approved layout")
    for name in WINDOWS_EXECUTABLES:
        _verify_pe(bundle, name)
    if files[WINDOWS_GAME]["sha256"] != files[WINDOWS_SHOWROOM]["sha256"]:
        raise ValueError("Preview 12 Windows Studio Showroom must use the same qualified game binary")
    identity = _json(_read_member(bundle, "SOURCE-IDENTITY.json"))
    ordinary, medusa = f"saves-preview12-{commit[:8]}", "saves-medusa-preview"
    if (identity.get("protectedMain"), identity.get("gitTree"), identity.get("compiledSource"), identity.get("profileDirectory"), identity.get("medusaProfileDirectory")) != (commit, tree, compiled, ordinary, medusa):
        raise ValueError("Preview 12 Windows source or isolated profile binding differs")
    for directory in (ordinary, medusa):
        name = f"{ROOT}/{directory}/"
        if name not in members or any(path.startswith(name) and path != name for path in members):
            raise ValueError(f"Preview 12 Windows requires an empty isolated profile: {directory}")
    expected_command = f'"%~dp0bin\\meridian-desktop.exe" --save-dir "%~dp0{ordinary}"'
    if _read_member(bundle, "Launch Meridian.cmd") != _windows_launcher(expected_command):
        raise ValueError("Preview 12 Windows ordinary launcher differs from isolated profile command")
    tracked_medusa = run("git", "show", f"{commit}:integration/sh171-medusa-preview/Launch Medusa Preview.cmd")
    if _read_member(bundle, "Launch Medusa Preview.cmd") != _launcher_bytes(tracked_medusa.splitlines()):
        raise ValueError("Preview 12 Windows Medusa launcher differs from exact committed preview command")
    if _read_member(bundle, "Launch Hull Studio.cmd") != _windows_launcher(f'"%~dp0{WINDOWS_STUDIO}\\Meridian Hull Studio.exe"'):
        raise ValueError("Preview 12 Windows Studio launcher differs from standalone command")


def verify_windows(archive, approval, commit, tree, run, source_notices=None):
    members, _ = verify_inventory(archive, "windows", approval)
    compiled = approval["compiledSource"]
    with zipfile.ZipFile(archive) as bundle:
        _verify_windows_identity(bundle, members, commit, tree, compiled, run)
        _verify_studio_runtime(bundle, members, "windows", commit, tree, compiled)
        _verify_source_notices(bundle, members, "windows", commit, tree, source_notices)


def verify(macos, windows, commit, run):
    """Verify both archives before publication and return the independently approved sidecar digest."""
    if macos.parent != windows.parent:
        raise ValueError("Preview 12 approved archives must share their fixed staging directory")
    path = macos.parent / APPROVAL
    approval, approval_sha = load_approval(path, commit, run)
    source_notices = _expected_source_notices(commit)
    verify_macos(macos, approval["platforms"]["macos"], commit, approval["tree"], run, source_notices)
    verify_windows(windows, approval["platforms"]["windows"], commit, approval["tree"], run, source_notices)
    return approval_sha, {
        macos.name: approval["platforms"]["macos"]["archiveSha256"],
        windows.name: approval["platforms"]["windows"]["archiveSha256"],
    }
