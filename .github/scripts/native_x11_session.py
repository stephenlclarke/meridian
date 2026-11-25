"""Keep an authenticated X11 connection alive around the native coverage command."""

import re
import select
import subprocess
import sys

from coverage_workspace import workspace_path


X11_CLIENT = ("xdotool", "-")


def run_session(command, log):
    with log.open("xb") as diagnostics, subprocess.Popen(
        X11_CLIENT, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=diagnostics,
        bufsize=0,
    ) as client:
        try:
            # Script mode keeps the same display connection open while stdin remains open.
            try:
                client.stdin.write(b"getdisplaygeometry\n")
            except BrokenPipeError:
                raise RuntimeError("X11 client exited before readiness") from None
            if not select.select([client.stdout], [], [], 10)[0]:
                raise RuntimeError("Timed out waiting for an authenticated X11 connection")
            geometry = client.stdout.read(128)
            if not re.fullmatch(rb"[1-9][0-9]* [1-9][0-9]*\n", geometry) or client.poll() is not None:
                raise RuntimeError(f"X11 client did not establish a live display: {geometry!r}")
            diagnostics.write(b"Authenticated X11 display geometry: " + geometry)
            diagnostics.flush()

            result = subprocess.run(command, check=False)
            diagnostics.write(f"Native coverage exit status: {result.returncode}\n".encode())
            if result.returncode:
                # A signal must also fail the job, even if no application receipt was written.
                return result.returncode if result.returncode > 0 else 128 - result.returncode
            if client.poll() is not None:
                raise RuntimeError("X11 client exited during native coverage")
            return 0
        except Exception as error:
            diagnostics.write(f"{type(error).__name__}: {error}\n".encode())
            raise
        finally:
            # Do not let an Xlib failure during display teardown replace the command's exit status.
            if client.poll() is None:
                client.kill()
            client.wait(timeout=10)


def main():
    if len(sys.argv) != 1:
        raise SystemExit("The native X11 coverage session does not accept arguments")
    command = [sys.executable, "-B", "-m", "coverage", "run", "--parallel-mode",
               str(workspace_path(".github/scripts/native_coverage_smoke.py"))]
    return run_session(command, workspace_path("coverage/x11-client.log"))


if __name__ == "__main__":
    raise SystemExit(main())
