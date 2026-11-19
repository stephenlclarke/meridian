"""Edit, validate and export Meridian ship definitions on this computer."""

import argparse
import json
import mimetypes
import os
import subprocess
import sys
import tempfile
import threading
import webbrowser
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit

from definitions import ROOT, encode, local_definition, parse, prepare, reference_bytes
from fit import fit_report
from showroom_bundle import showroom_executable
from surface import envelope_view


MAX_UPLOAD_BYTES = 64 * 1024 * 1024 + 32


class InvalidHull(ValueError):
    def __init__(self, message, report=None, package=None):
        super().__init__(message)
        self.report = report
        self.package = package


def build(definition, validator):
    package = prepare(definition)
    encoded = encode(package)
    with tempfile.TemporaryDirectory(prefix="meridian-hull-") as folder:
        path = Path(folder) / "package.json"
        report_path = path.with_name("report.json")
        path.write_bytes(encoded)
        result = subprocess.run(
            [str(validator), "validate", str(path), "--report", str(report_path)],
            capture_output=True,
            check=False,
        )
        report = parse(report_path.read_bytes()) if report_path.is_file() else {}
    if result.returncode not in (0, 1):
        raise RuntimeError(
            result.stderr.decode(errors="replace").strip() or "Could not run validator"
        )
    if not isinstance(report, dict):
        raise RuntimeError("The collision validator returned an invalid report")
    if result.returncode or report.get("valid") is not True:
        detail = (
            result.stderr.decode(errors="replace").strip() or "Hull validation failed"
        )
        raise InvalidHull(detail, report, package)
    return package, report


def inspect_surface(package, report):
    try:
        report["envelope"] = envelope_view(package)
        report["fit"] = fit_report(package, report["envelope"])
    except (ArithmeticError, ValueError) as error:
        report["viewWarning"] = str(error)
        report["supportPoints"] = [
            p for g in report.get("supportGroups", []) for p in g["points"]
        ]


class GamePreview:
    """Keep the disposable profile alive until its game process exits."""

    def __init__(self, executable):
        self.executable = executable
        self.lock = threading.Lock()
        self.running = False

    def launch(self, package):
        if self.executable is None:
            raise RuntimeError("The game inspector is unavailable in this installation")
        with self.lock:
            if self.running:
                raise RuntimeError(
                    "Close the open Showroom window before opening another ship"
                )
            workspace = tempfile.TemporaryDirectory(prefix="meridian-hull-showroom-")
            folder = Path(workspace.name)
            try:
                hull = folder / "ship.hull.json"
                hull.write_bytes(encode(package))
                profile = folder / "profile"
                profile.mkdir()
                child = subprocess.Popen(
                    [
                        str(self.executable),
                        "--save-dir",
                        str(profile),
                        "--inspect-hull-package",
                        str(hull),
                        "--open-authored-showroom",
                    ],
                    cwd=folder,
                    stdin=subprocess.DEVNULL,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                )
            except BaseException:
                workspace.cleanup()
                raise
            self.running = True

        startup = threading.Event()
        rendered = threading.Event()
        failure = []

        def monitor():
            last_line = ""
            try:
                with child.stdout as messages:
                    for raw in messages:
                        line = raw.decode(errors="replace").strip()
                        if line == "MERIDIAN_HULL_STUDIO_READY":
                            rendered.set()
                            startup.set()
                        elif line:
                            last_line = line[-1000:]
                status = child.wait()
                if not rendered.is_set():
                    failure.append(
                        last_line
                        or f"Game exited with status {status} before rendering"
                    )
            finally:
                try:
                    workspace.cleanup()
                finally:
                    with self.lock:
                        self.running = False
                    startup.set()

        threading.Thread(target=monitor, daemon=False).start()
        if not startup.wait(45):
            raise RuntimeError(
                "The game is still starting; its Showroom window has not confirmed a frame"
            )
        if not rendered.is_set():
            raise RuntimeError(
                f"Showroom did not open: {failure[0] if failure else 'game startup interrupted'}"
            )


class StudioRequest(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        if sys.stderr is not None:
            super().log_message(fmt, *args)

    def respond(self, status, content, mime="application/json", attachment=False):
        self.send_response(status)
        self.send_header("Content-Type", mime)
        self.send_header("Content-Length", str(len(content)))
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("Referrer-Policy", "no-referrer")
        if attachment:
            self.send_header(
                "Content-Disposition", 'attachment; filename="ship.hull.json"'
            )
        self.end_headers()
        self.wfile.write(content)

    def local(self):
        address = f"127.0.0.1:{self.server.server_port}"
        return (
            self.headers.get("Host") == address
            and self.headers.get("Origin", f"http://{address}") == f"http://{address}"
        )

    def do_GET(self):
        if not self.local():
            return self.respond(403, encode({"error": "Use the local Studio address"}))
        route = urlsplit(self.path).path
        if route == "/api/capabilities":
            return self.respond(
                200, encode({"showroom": self.server.preview.executable is not None})
            )
        if route == "/api/example":
            return self.respond(200, self.server.example)
        root = Path(__file__).with_name("web").resolve()
        path = (root / ("index.html" if route == "/" else route.lstrip("/"))).resolve()
        if not path.is_relative_to(root) or not path.is_file():
            return self.respond(404, encode({"error": "Unknown resource"}))
        self.respond(
            200,
            path.read_bytes(),
            mimetypes.guess_type(path.name)[0] or "application/octet-stream",
        )

    def do_POST(self):
        if not self.local():
            # Winsock can reset a closed socket with unread request data,
            # discarding its 403 response. Drain bounded input without parsing it.
            try:
                remaining = int(self.headers.get("Content-Length", "0"))
            except ValueError:
                remaining = 0
            if 0 < remaining <= MAX_UPLOAD_BYTES:
                while remaining:
                    block = self.rfile.read(min(65536, remaining))
                    if not block:
                        break
                    remaining -= len(block)
            return self.respond(403, encode({"error": "Use the local Studio address"}))
        route = urlsplit(self.path).path
        if route not in ("/api/build", "/api/export", "/api/quit", "/api/showroom"):
            return self.respond(404, encode({"error": "Unknown operation"}))
        if self.headers.get_content_type() != "application/json":
            return self.respond(415, encode({"error": "Send a JSON definition"}))
        try:
            length = int(self.headers.get("Content-Length", "-1"))
            if not 0 < length <= MAX_UPLOAD_BYTES:
                return self.respond(
                    413, encode({"error": "Expected a JSON upload of at most 64 MiB"})
                )
            body = parse(self.rfile.read(length))
            if route == "/api/quit":
                if body != {}:
                    raise ValueError("Expected an empty object")
                self.respond(200, encode({"closed": True}))
                threading.Thread(target=self.server.shutdown, daemon=True).start()
                return
            if not isinstance(body, dict) or set(body) != {"definition"}:
                raise ValueError("Expected an object with a definition field")
            package, report = build(body["definition"], self.server.validator)
            if route == "/api/showroom":
                self.server.preview.launch(package)
                return self.respond(200, encode({"opened": True}))
        except InvalidHull as error:
            if route == "/api/build" and error.report.get("supportGroups"):
                inspect_surface(error.package, error.report)
            return self.respond(
                422, encode({"error": str(error), "report": error.report})
            )
        except (ValueError, RecursionError, OverflowError) as error:
            return self.respond(422, encode({"error": str(error)}))
        except (OSError, RuntimeError) as error:
            return self.respond(503, encode({"error": str(error)}))
        if route == "/api/export":
            return self.respond(200, encode(package), attachment=True)
        inspect_surface(package, report)
        self.respond(200, encode({"package": package, "report": report}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    binary = "meridian-hull-tool" + (".exe" if os.name == "nt" else "")
    default_validator = (
        Path(__file__).with_name(binary)
        if getattr(sys, "frozen", False)
        else ROOT / "target/release" / binary
    )
    parser.add_argument("--validator", type=Path, default=default_validator)
    parser.add_argument(
        "--game",
        type=Path,
        help="Meridian desktop executable for opening validated ships",
    )
    parser.add_argument(
        "--definition",
        type=Path,
        help="Open a local definition in place of Peregrine Mk III",
    )
    parser.add_argument(
        "--export", type=Path, help="Validate and export without opening the editor"
    )
    parser.add_argument("--port", type=int, default=0)
    parser.add_argument("--no-browser", action="store_true")
    args = parser.parse_args()
    validator = args.validator.resolve()
    if not validator.is_file():
        parser.error(
            f"Build meridian-hull-tool or select it with --validator ({validator})"
        )
    sample = (
        encode(local_definition(args.definition))
        if args.definition
        else reference_bytes()
    )
    if args.export:
        package, report = build(parse(sample), validator)
        with tempfile.NamedTemporaryFile(
            dir=args.export.parent, delete=False
        ) as stream:
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
        game = (
            args.game.resolve()
            if args.game
            else showroom_executable(
                default_validator,
                Path(sys.executable),
                sys.platform,
                getattr(sys, "frozen", False),
            )
        )
        if args.game and not game.is_file():
            parser.error(f"Game executable is missing: {game}")
        server.preview = GamePreview(game if game.is_file() else None)
        server.validator, server.example = validator, sample
        address = f"http://127.0.0.1:{server.server_port}/"
        if sys.stdout is not None:
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
