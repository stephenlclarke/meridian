"""Edit, validate and export Meridian ship definitions on this computer."""
import argparse
import json
import mimetypes
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import webbrowser
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlsplit

from definitions import ROOT, encode, local_definition, parse, prepare, reference_bytes


class InvalidHull(ValueError):
    def __init__(self, message, report=None):
        super().__init__(message)
        self.report = report


def build(definition, validator):
    package = prepare(definition)
    encoded = encode(package)
    with tempfile.TemporaryDirectory(prefix="meridian-hull-") as folder:
        path = Path(folder) / "package.json"
        report_path = path.with_name("report.json")
        path.write_bytes(encoded)
        result = subprocess.run([str(validator), "validate", str(path), "--report", str(report_path)],
                                capture_output=True, check=False)
        report = parse(report_path.read_bytes()) if report_path.is_file() else {}
    if not isinstance(report, dict):
        raise RuntimeError("The collision validator returned an invalid report")
    if result.returncode or report.get("valid") is not True:
        detail = result.stderr.decode(errors="replace").strip() or "Hull validation failed"
        raise InvalidHull(detail, report)
    return package, report


class StudioRequest(BaseHTTPRequestHandler):
    def respond(self, status, content, mime="application/json", attachment=False):
        self.send_response(status)
        self.send_header("Content-Type", mime)
        self.send_header("Content-Length", str(len(content)))
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("Referrer-Policy", "no-referrer")
        if attachment:
            self.send_header("Content-Disposition", 'attachment; filename="ship.hull.json"')
        self.end_headers()
        self.wfile.write(content)

    def local(self):
        address = f"127.0.0.1:{self.server.server_port}"
        return (self.headers.get("Host") == address
                and self.headers.get("Origin", f"http://{address}") == f"http://{address}")

    def do_GET(self):
        if not self.local():
            return self.respond(403, encode({"error":"Use the local Studio address"}))
        route = urlsplit(self.path).path
        if route == "/api/example":
            return self.respond(200, self.server.example)
        root = Path(__file__).with_name("web").resolve()
        path = (root / ("index.html" if route == "/" else route.lstrip("/"))).resolve()
        if not path.is_relative_to(root) or not path.is_file():
            return self.respond(404, encode({"error":"Unknown resource"}))
        self.respond(200, path.read_bytes(), mimetypes.guess_type(path.name)[0] or "application/octet-stream")

    def do_POST(self):
        if not self.local():
            return self.respond(403, encode({"error":"Use the local Studio address"}))
        route = urlsplit(self.path).path
        if route not in ("/api/build", "/api/export"):
            return self.respond(404, encode({"error":"Unknown operation"}))
        if self.headers.get_content_type() != "application/json":
            return self.respond(415, encode({"error":"Send a JSON definition"}))
        try:
            length = int(self.headers.get("Content-Length", "-1"))
            if not 0 < length <= 32 * 1024 * 1024:
                return self.respond(413, encode({"error":"Expected a JSON upload of at most 32 MiB"}))
            body = parse(self.rfile.read(length))
            if not isinstance(body, dict) or set(body) != {"definition"}:
                raise ValueError("Expected an object with a definition field")
            package, report = build(body["definition"], self.server.validator)
        except InvalidHull as error:
            return self.respond(422, encode({"error":str(error), "report":error.report}))
        except (ValueError, RecursionError, OverflowError) as error:
            return self.respond(422, encode({"error":str(error)}))
        except (OSError, RuntimeError) as error:
            return self.respond(503, encode({"error":str(error)}))
        if route == "/api/export":
            return self.respond(200, encode(package), attachment=True)
        from surface import envelope_view
        from fit import fit_report
        try:
            report["envelope"] = envelope_view(package)
            report["fit"] = fit_report(package, report["envelope"])
        except (ArithmeticError, ValueError) as error:
            report["viewWarning"] = str(error)
            report["supportPoints"] = [p for g in report.get("supportGroups", []) for p in g["points"]]
        self.respond(200, encode({"package":package, "report":report}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    binary = "meridian-hull-tool" + (".exe" if os.name == "nt" else "")
    parser.add_argument("--validator", type=Path, default=ROOT / "target/release" / binary)
    parser.add_argument("--definition", type=Path, help="Open a local definition in place of Peregrine Mk III")
    parser.add_argument("--export", type=Path, help="Validate and export without opening the editor")
    parser.add_argument("--port", type=int, default=0)
    parser.add_argument("--no-browser", action="store_true")
    args = parser.parse_args()
    validator = args.validator.resolve()
    if not validator.is_file():
        parser.error(f"Build meridian-hull-tool or select it with --validator ({validator})")
    sample = encode(local_definition(args.definition)) if args.definition else reference_bytes()
    if args.export:
        package, report = build(parse(sample), validator)
        with tempfile.NamedTemporaryFile(dir=args.export.parent, delete=False) as stream:
            temporary = Path(stream.name)
            try:
                stream.write(encode(package))
                stream.flush()
                os.fsync(stream.fileno())
            except BaseException:
                temporary.unlink(missing_ok=True)
                raise
        try:
            os.replace(temporary, args.export)
        finally:
            temporary.unlink(missing_ok=True)
        print(json.dumps(report))
        return
    with ThreadingHTTPServer(("127.0.0.1", args.port), StudioRequest) as server:
        server.validator, server.example = validator, sample
        address = f"http://127.0.0.1:{server.server_port}/"
        print(f"Meridian Hull Studio is open at {address}", flush=True)
        if not args.no_browser:
            webbrowser.open(address)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, RuntimeError) as exc:
        sys.exit(str(exc))
