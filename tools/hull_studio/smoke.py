"""Release smoke for the real local Studio and an already-built Rust validator.

Run from a joined checkout with --validator PATH. No build, browser, graphical
session, or mock validator is used. Temporary exports and server logs are isolated.
"""

import argparse
from contextlib import contextmanager
from copy import deepcopy
import hashlib
import http.client
import json
from math import isclose
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time

from definitions import ROOT, encode, parse, reference_bytes


STUDIO = Path(__file__).with_name("studio.py")


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def file_digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


@contextmanager
def server(validator, folder, timeout, definition=None):
    argv = [sys.executable, "-B", str(STUDIO), "--validator", str(validator), "--no-browser"]
    if definition is not None:
        argv.extend(["--definition", str(definition)])
    with (folder / "stdout.log").open("w+b") as stdout, (folder / "stderr.log").open("w+b") as stderr:
        process = subprocess.Popen(argv, cwd=ROOT, stdout=stdout, stderr=stderr)
        try:
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                require(process.poll() is None, f"Studio exited during startup: {(folder / 'stderr.log').read_text(errors='replace')}")
                output = (folder / "stdout.log").read_text(errors="replace")
                match = re.search(r"http://127\.0\.0\.1:(\d+)/", output)
                if match:
                    yield int(match.group(1))
                    return
                time.sleep(0.05)
            raise RuntimeError("Studio did not announce a loopback port before the timeout")
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=5)


def request(port, method, route, timeout, definition=None, headers=None):
    address = f"127.0.0.1:{port}"
    supplied = {"Host": address, "Origin": f"http://{address}"}
    supplied.update(headers or {})
    body = None
    if definition is not None:
        supplied["Content-Type"] = "application/json"
        body = encode({"definition": definition})
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=timeout)
    try:
        connection.request(method, route, body=body, headers=supplied)
        response = connection.getresponse()
        content = response.read()
        require(response.getheader("Cache-Control") == "no-store", f"Missing no-store for {route}")
        return response.status, content, response.getheader("Content-Disposition")
    finally:
        connection.close()


def success(response, label):
    status, content, _ = response
    require(status == 200, f"{label} returned {status}: {content[:500]!r}")
    return parse(content)


def rejected(response, expected, label):
    status, content, attachment = response
    require(status == expected, f"{label} returned {status}, expected {expected}")
    body = parse(content)
    require(isinstance(body, dict) and bool(body.get("error")), f"{label} had no error message")
    require("package" not in body and "schema" not in body and not attachment,
            f"{label} leaked an export or stale package")
    return body


def validate_export(validator, folder, name, data, timeout):
    path = folder / f"{name}.json"
    report_path = folder / f"{name}-report.json"
    path.write_bytes(data)
    result = subprocess.run([str(validator), "validate", str(path), "--report", str(report_path)],
                            cwd=ROOT, capture_output=True, timeout=timeout, check=False)
    require(result.returncode == 0 and report_path.is_file(),
            f"Direct validator rejected exported {name}: {result.stderr.decode(errors='replace')}")
    report = parse(report_path.read_bytes())
    require(report.get("valid") is True and not report.get("diagnostics"), f"Invalid direct report for {name}")
    return report


def check_report(package, report):
    require(report.get("valid") is True and report.get("diagnostics") == [], "Expected clean validator report")
    expected = {route["name"]: True for route in package["clearances"]}
    require(report.get("clearances") == [{"name": name, "passed": passed} for name, passed in expected.items()],
            "Clearance results differ from declared successful routes")
    # The authoritative query reports metre-space support inputs, not display triangles.
    groups = {}
    for domain in ("detail", "native"):
        mesh = package[domain]
        for point, group in zip(mesh["vertices"], mesh["groups"], strict=True):
            groups.setdefault(group, []).append([value * package["metresPerUnit"] for value in point])
    actual = report.get("supportGroups")
    require(isinstance(actual, list) and len(actual) == len(groups), "Missing or stale runtime support group query")
    require([group["id"] for group in actual] == sorted(groups), "Runtime support group IDs differ from config")
    for group in actual:
        require(group["roundingMetres"] == package["roundingMetres"], "Runtime rounding differs from config")
        expected_points = groups[group["id"]]
        points = group["points"]
        require(len(points) == len(expected_points) and all(
            len(observed) == 3 and all(isclose(a, b, rel_tol=1e-14, abs_tol=1e-13)
                                       for a, b in zip(observed, expected_point, strict=True))
            for observed, expected_point in zip(sorted(points), sorted(expected_points), strict=True)
        ), "Runtime support points differ from scaled config")


def export(port, definition, timeout):
    status, data, attachment = request(port, "POST", "/api/export", timeout, definition)
    require(status == 200 and attachment == 'attachment; filename="ship.hull.json"',
            f"Export failed: {status} {data[:500]!r}")
    return data


def build(port, definition, timeout):
    result = success(request(port, "POST", "/api/build", timeout, definition), "build")
    require(set(result) == {"package", "report"}, "Unexpected build response fields")
    check_report(result["package"], result["report"])
    mesh = result["report"].get("envelope")
    require(isinstance(mesh, dict) and bool(mesh.get("vertices")) and bool(mesh.get("triangles")),
            f"Build omitted illustrative envelope: {result['report'].get('viewWarning')}")
    return result


def authored_definition():
    # Two disjoint tetrahedra, each an independent original group. No catalogue
    # asset, engine state, save or runtime hull is borrowed for this definition.
    def vertices(x):
        return [[x-1, -1, -1], [x+1, -1, -1], [x, 1, -1], [x, 0, 1]]
    faces = [[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]]
    mesh = {"vertices": vertices(-3) + vertices(3),
            "triangles": faces + [[i+4 for i in face] for face in faces]}
    return {"schema": 1, "id": "SMOKE-TEST-HULL", "name": "Studio Smoke Hull",
            "metresPerUnit": 1, "roundingMetres": 0.02,
            "detail": deepcopy(mesh), "native": deepcopy(mesh),
            "materials": [{"name": "Grey", "rgb": [100, 110, 120]}], "faceMaterials": [0]*8,
            "engineering": {"massKg": 100, "centerOfMass": [0, 0, 0],
                            "inertia": [[10, 0, 0], [0, 10, 0], [0, 0, 10]]},
            "flight": {"maxSpeed": 10},
            "clearances": [{"name": "Central gap", "start": [0, 0, -4], "end": [0, 0, 4], "radiusMetres": 0.1}],
            "provenance": {"source": "Independent release smoke fixture"}}


def check_rejected_export(validator, folder, invalid, timeout):
    definition = folder / "invalid-definition.json"
    definition.write_bytes(encode(invalid))
    target = folder / "invalid-export.json"
    args = [sys.executable, "-B", str(STUDIO), "--validator", str(validator),
            "--definition", str(definition), "--export", str(target), "--no-browser"]
    result = subprocess.run(args, cwd=ROOT, capture_output=True, timeout=timeout, check=False)
    require(result.returncode != 0 and not target.exists(), "Invalid standalone export created a package")
    preserved = b"existing content must survive a rejected export\n"
    target.write_bytes(preserved)
    result = subprocess.run(args, cwd=ROOT, capture_output=True, timeout=timeout, check=False)
    require(result.returncode != 0 and target.read_bytes() == preserved, "Invalid standalone export overwrote existing content")


def smoke(validator, timeout):
    with tempfile.TemporaryDirectory(prefix="meridian-hull-release-smoke-") as temp:
        folder = Path(temp)
        reference_path = folder / "reference-server"
        reference_path.mkdir()
        with server(validator, reference_path, timeout) as port:
            baseline = request(port, "GET", "/api/example", timeout)
            original = success(baseline, "SH023 example")
            require(original["id"] == "SH023" and baseline[1] == reference_bytes(), "SH023 example is not the exact pinned reference")
            original_build = build(port, original, timeout)
            require(original_build["package"] == original, "SH023 build changed original definition")
            original_export = export(port, original, timeout)
            require(original_export == baseline[1], "SH023 export changed exact source bytes")
            direct = validate_export(validator, folder, "sh023-export", original_export, timeout)
            check_report(original, direct)
            require(all(original_build["report"].get(k) == v for k, v in direct.items()), "SH023 direct and HTTP runtime reports differ")
            require(request(port, "GET", "/api/example", timeout)[1] == baseline[1], "Reference example changed after build/export")
            rejected(request(port, "GET", "/api/example", timeout, headers={"Host": "invalid.local"}), 403, "wrong Host")
            rejected(request(port, "POST", "/api/build", timeout, original, {"Origin": "http://invalid.local"}), 403, "wrong Origin")

        authored = authored_definition()
        authored_path = folder / "authored-definition.json"
        authored_path.write_bytes(encode(authored))
        authored_server = folder / "authored-server"
        authored_server.mkdir()
        with server(validator, authored_server, timeout, authored_path) as port:
            require(success(request(port, "GET", "/api/example", timeout), "authored example") == authored,
                    "Config-only import changed authored data")
            initial = build(port, authored, timeout)
            package = initial["package"]
            require(package["id"] == authored["id"] and package["detail"]["groups"] == [0]*4+[1]*4
                    and package["native"]["groups"] == [2]*4+[3]*4, "Automatic local groups differ from the independent twin-tetra fixture")
            initial_export = export(port, authored, timeout)
            require(parse(initial_export) == package, "Initial export differs from validated package")
            require(export(port, authored, timeout) == initial_export, "Repeat authored export changed content")

            changed = deepcopy(package)
            changed["metresPerUnit"] = 2
            for domain in ("detail", "native"):
                for point in changed[domain]["vertices"][:4]:
                    point[0] -= 0.5
                changed[domain]["groups"] = [17]*4 + [41]*4
            changed_build = build(port, changed, timeout)
            require(changed_build["package"] == changed, "Edited config ignored coordinate, scale or group changes")
            require(changed_build["report"]["supportGroups"] != initial["report"]["supportGroups"], "Edited config returned stale support geometry")
            changed_export = export(port, changed, timeout)
            require(changed_export != initial_export and parse(changed_export) == changed, "Edited export is stale")
            direct_edited = validate_export(validator, folder, "authored-edited-export", changed_export, timeout)
            check_report(changed, direct_edited)
            require(all(changed_build["report"].get(k) == v for k, v in direct_edited.items()),
                    "Edited direct and HTTP runtime reports differ")
            require(build(port, changed, timeout) == changed_build, "Repeated edited build changed content")

            invalid = deepcopy(changed)
            invalid["clearances"][0].update(start=[-12, 0, 0], end=[12, 0, 0])
            for route in ("/api/build", "/api/export"):
                failure = rejected(request(port, "POST", route, timeout, invalid), 422, f"blocked probe {route}")
                report = failure.get("report", {})
                require(report.get("valid") is False and report.get("clearances") == [{"name": "Central gap", "passed": False}]
                        and bool(report.get("diagnostics")), "Blocked probe lacks actual failed runtime witness")
            require(export(port, changed, timeout) == changed_export, "Rejected probe changed the last valid export")
            require(success(request(port, "GET", "/api/example", timeout), "authored example after refusal") == authored,
                    "Failed build changed the imported source")
            check_rejected_export(validator, folder, invalid, timeout)

        return {"passed": True, "validatorSha256": file_digest(validator),
                "sh023ExportSha256": digest(original_export), "authoredExportSha256": digest(initial_export),
                "editedExportSha256": digest(changed_export), "referenceGroups": len(direct["supportGroups"]),
                "authoredGroups": len(initial["report"]["supportGroups"]), "editedGroups": len(changed_build["report"]["supportGroups"])}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--validator", required=True, type=Path, help="Path to the built real meridian-hull-tool")
    parser.add_argument("--timeout", type=float, default=180, help="Per HTTP/subprocess timeout in seconds")
    args = parser.parse_args()
    validator = args.validator.resolve()
    if not validator.is_file() or not 0 < args.timeout <= 600:
        parser.error("Select a real validator binary and a timeout in (0, 600] seconds")
    try:
        print(json.dumps(smoke(validator, args.timeout), sort_keys=True))
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError, http.client.HTTPException) as error:
        parser.exit(1, f"Hull Studio smoke failed: {error}\n")


if __name__ == "__main__":
    main()
