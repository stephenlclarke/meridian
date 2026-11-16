"""Publish qualified platform previews; version, verify downloads, then retain three."""
import argparse
import hashlib
import json
import os
import re
import subprocess
import tempfile
import tomllib
from pathlib import Path

REPOSITORY = "stephenlclarke/meridian"
STAGING_ROOT = Path.home() / ".local/share/meridian-preview-releases"
PACKAGE_FILES = ("Meridian-macos-arm64.zip", "Meridian-windows-x64.zip", "RELEASE-NOTES.md")
TAG = re.compile(r"v(\d+)\.(\d+)\.(\d+)-preview\.([1-9]\d*)\Z")
COMMIT = re.compile(r"([a-z]+)(?:\([^\n)]+\))?(!)?: ")
REQUIRED_CHECKS = {"Rust (ubuntu-latest)", "Rust (windows-latest)", "Rust (macos-latest)",
                   "Documentation", "Coverage reports", "SonarQube (main)"}


def run(executable, *args):
    if executable not in {"git", "gh", "/usr/bin/ditto", "/usr/bin/codesign"}:
        raise ValueError("Only the fixed release tools may run")
    return subprocess.check_output([executable, *args], text=True).strip()


def source_commit(value):
    if not re.fullmatch(r"[0-9a-f]{40}", value):
        raise ValueError("Use an exact 40-character committed-main SHA")
    return bytes.fromhex(value).hex()


def api(path, *fields):
    return json.loads(run("gh", "api", path, *fields))


def previews(items):
    return sorted(
        (r for r in items if not r["draft"] and r["prerelease"] and TAG.fullmatch(r["tag_name"])),
        key=lambda r: (r["published_at"], r["id"]), reverse=True,
    )


def initial_tag(value):
    match = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)", value)
    if not match:
        raise ValueError("Initial workspace version must be numeric SemVer")
    major, minor, patch = map(int, match.groups())
    return f"v{major}.{minor}.{patch}-preview.1"


def next_tag(previous, messages, initial):
    """Only a preview's tag is its version; never infer from a branch name."""
    if previous is None:
        return initial_tag(initial)
    match = TAG.fullmatch(previous)
    if not match:
        raise ValueError("Not a managed preview tag")
    major, minor, patch, serial = map(int, match.groups())
    bump = 0
    for message in messages:
        header = COMMIT.match(message.splitlines()[0])
        if (header and header[2]) or re.search(r"^BREAKING[ -]CHANGE: ", message, re.M):
            bump = max(bump, 3)
        elif header and header[1] == "feat":
            bump = max(bump, 2)
        elif header and header[1] in {"fix", "perf"}:
            bump = max(bump, 1)
    if bump == 3:
        major, minor, patch, serial = major + 1, 0, 0, 1
    elif bump == 2:
        minor, patch, serial = minor + 1, 0, 1
    elif bump == 1:
        patch, serial = patch + 1, 1
    else:
        serial += 1
    return f"v{major}.{minor}.{patch}-preview.{serial}"


def releases():
    pages = json.loads(run("gh", "api", "--paginate", "--slurp",
                           f"repos/{REPOSITORY}/releases?per_page=100"))
    return [release for page in pages for release in page]


def plan(commit):
    comparison = api(f"repos/{REPOSITORY}/compare/{commit}...main")
    if comparison["status"] not in {"ahead", "identical"}:
        raise ValueError("Preview source must be on remote main")
    latest = previews(releases())
    previous = latest[0]["tag_name"] if latest else None
    revision = commit
    if previous:
        baseline = source_commit(api(f"repos/{REPOSITORY}/commits/{previous}")["sha"])
        run("git", "merge-base", "--is-ancestor", baseline, commit)
        if baseline == commit:
            raise ValueError("This source already has a published preview")
        revision = f"{baseline}..{commit}"
    messages = [m.strip() for m in run("git", "log", "--format=%B%x00", revision).split("\0") if m.strip()]
    cargo = tomllib.loads(run("git", "show", f"{commit}:Cargo.toml"))
    tag = next_tag(previous, messages, cargo["workspace"]["package"]["version"])
    return tag, [r["tag_name"] for r in latest[2:]]


def package_files(identifier):
    """Accept a package identifier, never agent-supplied filesystem paths."""
    if not identifier or not re.fullmatch(r"[a-z0-9][a-z0-9-]{0,63}", identifier):
        raise ValueError("Package identifier must contain only lowercase letters, digits and hyphens")
    if STAGING_ROOT.is_symlink():
        raise ValueError("Release staging root must not be a symlink")
    root = STAGING_ROOT.resolve(strict=True)
    directory = root / identifier
    if directory.is_symlink() or directory.resolve(strict=True).parent != root:
        raise ValueError("Package directory must stay inside release staging")
    paths = [directory / name for name in (*PACKAGE_FILES, "release-receipt.json")]
    if any(p.is_symlink() or p.resolve().parent != directory for p in paths):
        raise ValueError("Package files must not be symlinks or escape release staging")
    if not all(p.is_file() for p in paths[:3]) or paths[3].exists():
        raise ValueError("Require both ZIPs, RELEASE-NOTES.md and a new receipt filename")
    return paths


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def verify_macos(archive):
    # ditto preserves the signed app's modes, symlinks and bundle metadata.
    with tempfile.TemporaryDirectory(prefix="meridian-release-signature-") as temporary:
        run("/usr/bin/ditto", "-x", "-k", str(archive), temporary)
        apps = list(Path(temporary).rglob("*.app"))
        if len(apps) != 1:
            raise ValueError("macOS archive must contain one .app bundle")
        run("/usr/bin/codesign", "--verify", "--deep", "--strict", str(apps[0]))
        result = subprocess.run(
            ["/usr/bin/codesign", "-dvv", str(apps[0])], check=True,
            text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )
        if "Authority=Developer ID Application:" not in result.stderr:
            raise ValueError("macOS preview requires a Developer ID Application signature")


def verify_downloads(tag, expected):
    with tempfile.TemporaryDirectory(prefix="meridian-release-download-") as temporary:
        run("gh", "release", "download", tag, "--repo", REPOSITORY, "--dir", temporary)
        for name, sha in expected.items():
            if name not in {*PACKAGE_FILES[:2], "SHA256SUMS.txt", "preview.json"}:
                raise ValueError("Unexpected downloadable asset filename")
            if digest(Path(temporary) / name) != sha:
                raise ValueError(f"Downloaded asset differs: {name}")


def verify_checks(commit):
    checks = api(f"repos/{REPOSITORY}/commits/{commit}/check-runs?per_page=100")["check_runs"]
    latest = {}
    for check in sorted(checks, key=lambda c: c["id"]):
        latest[check["name"]] = check["conclusion"]
    if any(latest.get(name) != "success" for name in REQUIRED_CHECKS):
        raise ValueError("Exact-main CI and Sonar must pass before publication")


def write_metadata(root, tag, commit, expected, notes):
    checksums = root / "SHA256SUMS.txt"
    metadata = root / "preview.json"
    full_notes = root / "release-notes.md"
    with checksums.open("x") as stream:
        stream.writelines(f"{sha}  {name}\n" for name, sha in expected.items())
    with metadata.open("x") as stream:
        json.dump({"version": tag[1:], "source": commit, "assets": expected}, stream, indent=2)
    with full_notes.open("x") as stream:
        stream.write(notes)
        stream.write(f"\n\nVersion: `{tag}`. Source: `{commit}`.\n")
    return checksums, metadata, full_notes


def create_draft(tag, commit, archives, checksums, metadata, notes):
    # Tag/ref creation refuses conflicts. Published tags are never moved.
    obj = api(f"repos/{REPOSITORY}/git/tags", "-f", f"tag={tag}",
              "-f", f"message=Meridian {tag}\nSource: {commit}",
              "-f", f"object={commit}", "-f", "type=commit")
    api(f"repos/{REPOSITORY}/git/refs", "-f", f"ref=refs/tags/{tag}",
        "-f", f"sha={source_commit(obj['sha'])}")
    run("gh", "release", "create", tag, "--repo", REPOSITORY,
        "--verify-tag", "--draft", "--prerelease", "--title", f"Meridian {tag}",
        "--notes-file", str(notes), *(str(p) for p in archives), str(checksums), str(metadata))


def publish(args):
    commit = source_commit(args.commit)
    tag, removal = plan(commit)
    print(json.dumps({"tag": tag, "source": commit, "remove_after_verified_publication": removal}))
    if args.plan:
        return
    macos, windows, notes, receipt_path = package_files(args.package)
    verify_checks(commit)
    verify_macos(macos)
    archives = [macos, windows]
    expected = {p.name: digest(p) for p in archives}
    with notes.open() as stream:
        content = stream.read()
    with tempfile.TemporaryDirectory(prefix="meridian-release-upload-") as temporary:
        checksums, metadata, full_notes = write_metadata(Path(temporary), tag, commit, expected, content)
        create_draft(tag, commit, archives, checksums, metadata, full_notes)
        expected.update({p.name: digest(p) for p in [checksums, metadata]})
        verify_downloads(tag, expected)
        run("gh", "release", "edit", tag, "--repo", REPOSITORY,
            "--draft=false", "--prerelease", "--latest=false")
        verify_downloads(tag, expected)
        receipt = api(f"repos/{REPOSITORY}/releases/tags/{tag}")
        with receipt_path.open("x") as stream:
            json.dump({"release": receipt, "verifiedDownloads": expected}, stream, indent=2)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        for old in previews(releases())[3:]:
            run("gh", "release", "delete", old["tag_name"], "--repo", REPOSITORY,
                "--yes", "--cleanup-tag")
        print(receipt["html_url"])


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--package", help="Identifier of a prepared package inside release staging")
    parser.add_argument("--plan", action="store_true", help="Read-only version and retention plan")
    publish(parser.parse_args(argv))


if __name__ == "__main__":
    main()
