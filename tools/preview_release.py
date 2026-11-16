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
TAG = re.compile(r"v(\d+)\.(\d+)\.(\d+)-preview\.([1-9]\d*)\Z")
COMMIT = re.compile(r"([a-z]+)(?:\([^\n)]+\))?(!)?: ")


def run(*args):
    return subprocess.check_output(args, text=True).strip()


def api(path, *fields):
    return json.loads(run("gh", "api", path, *fields))


def previews(releases):
    return sorted(
        (r for r in releases if not r["draft"] and r["prerelease"]
         and TAG.fullmatch(r["tag_name"])),
        key=lambda r: (r["published_at"], r["id"]), reverse=True,
    )


def next_tag(previous, messages, initial):
    """Only a preview's tag is its version; never infer from a branch name."""
    if previous is None:
        return f"v{initial}-preview.1"
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
            if digest(Path(temporary) / name) != sha:
                raise ValueError(f"Downloaded asset differs: {name}")


def publish(args):
    if not re.fullmatch(r"[0-9a-f]{40}", args.commit):
        raise ValueError("Use an exact 40-character committed-main SHA")
    comparison = api(f"repos/{REPOSITORY}/compare/{args.commit}...main")
    if comparison["status"] not in {"ahead", "identical"}:
        raise ValueError("Preview source must be on remote main")
    latest = previews(releases())
    previous = latest[0]["tag_name"] if latest else None
    revision = args.commit
    if previous:
        baseline = api(f"repos/{REPOSITORY}/commits/{previous}")["sha"]
        run("git", "merge-base", "--is-ancestor", baseline, args.commit)
        if baseline == args.commit:
            raise ValueError("This source already has a published preview")
        revision = f"{baseline}..{args.commit}"
    messages = run("git", "log", "--format=%B%x00", revision).split("\0")
    messages = [m.strip() for m in messages if m.strip()]
    cargo = tomllib.loads(run("git", "show", f"{args.commit}:Cargo.toml"))
    tag = next_tag(previous, messages, cargo["workspace"]["package"]["version"])
    print(json.dumps({"tag": tag, "source": args.commit,
                      "remove_after_verified_publication": [r["tag_name"] for r in latest[2:]]}))
    if args.plan:
        return
    if not all([args.macos, args.windows, args.notes, args.receipt]):
        raise ValueError("Publication needs both platform archives, release notes and receipt path")
    archives = [args.macos.resolve(), args.windows.resolve()]
    if any(not p.is_file() or p.suffix.lower() != ".zip" for p in archives):
        raise ValueError("Provide the qualified macOS and Windows ZIP files")
    if archives[0].name == archives[1].name:
        raise ValueError("Platform archives must have distinct filenames")
    if args.receipt.exists() or not args.receipt.parent.is_dir():
        raise ValueError("Receipt must name a new file in an existing external directory")
    required = {"Rust (ubuntu-latest)", "Rust (windows-latest)",
                "Rust (macos-latest)", "Documentation", "Coverage reports", "SonarQube (main)"}
    checks = api(f"repos/{REPOSITORY}/commits/{args.commit}/check-runs?per_page=100")["check_runs"]
    latest_checks = {}
    for check in sorted(checks, key=lambda c: c["id"]):
        latest_checks[check["name"]] = check["conclusion"]
    if any(latest_checks.get(name) != "success" for name in required):
        raise ValueError("Exact-main CI and Sonar must pass before publication")
    verify_macos(archives[0])
    expected = {p.name: digest(p) for p in archives}
    notes = args.notes.read_text()
    with tempfile.TemporaryDirectory(prefix="meridian-release-upload-") as temporary:
        root = Path(temporary)
        checksums = root / "SHA256SUMS.txt"
        checksums.write_text("".join(f"{sha}  {name}\n" for name, sha in expected.items()))
        metadata = root / "preview.json"
        metadata.write_text(json.dumps({"version": tag[1:], "source": args.commit,
                                        "assets": expected}, indent=2) + "\n")
        full_notes = root / "release-notes.md"
        full_notes.write_text(notes + f"\n\nVersion: `{tag}`. Source: `{args.commit}`.\n")
        # Tag/ref creation refuses conflicts. Published tags are never moved.
        obj = api(f"repos/{REPOSITORY}/git/tags", "-f", f"tag={tag}",
                  "-f", f"message=Meridian {tag}\nSource: {args.commit}",
                  "-f", f"object={args.commit}", "-f", "type=commit")
        api(f"repos/{REPOSITORY}/git/refs", "-f", f"ref=refs/tags/{tag}",
            "-f", f"sha={obj['sha']}")
        run("gh", "release", "create", tag, "--repo", REPOSITORY,
            "--verify-tag", "--draft", "--prerelease", "--title", f"Meridian {tag}",
            "--notes-file", str(full_notes), *(str(p) for p in archives),
            str(checksums), str(metadata))
        expected.update({p.name: digest(p) for p in [checksums, metadata]})
        verify_downloads(tag, expected)
        run("gh", "release", "edit", tag, "--repo", REPOSITORY,
            "--draft=false", "--prerelease", "--latest=false")
        verify_downloads(tag, expected)
        receipt = api(f"repos/{REPOSITORY}/releases/tags/{tag}")
        with args.receipt.open("x") as stream:
            json.dump({"release": receipt, "verifiedDownloads": expected}, stream, indent=2)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        # Only managed published previews qualify, never stable or unrelated tags.
        for old in previews(releases())[3:]:
            run("gh", "release", "delete", old["tag_name"], "--repo", REPOSITORY,
                "--yes", "--cleanup-tag")
        print(receipt["html_url"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--macos", type=Path)
    parser.add_argument("--windows", type=Path)
    parser.add_argument("--notes", type=Path)
    parser.add_argument("--receipt", type=Path)
    parser.add_argument("--plan", action="store_true", help="Read-only version and retention plan")
    publish(parser.parse_args())


if __name__ == "__main__":
    main()
