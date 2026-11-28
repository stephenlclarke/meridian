"""The game executable location shared by packaging and Studio startup."""

from pathlib import Path

MAC_SHOWROOM_NAME = "Meridian Hull Showroom"
MAC_SHOWROOM_ID = "io.xyzzy.meridian.hullstudio.showroom"
MAC_SHOWROOM_RELATIVE = Path("Helpers") / f"{MAC_SHOWROOM_NAME}.app"
MAC_EXECUTABLE = Path("Contents/MacOS/meridian-desktop")


def showroom_executable(validator, executable, platform, frozen):
    if frozen and platform == "darwin":
        return executable.parent.parent / MAC_SHOWROOM_RELATIVE / MAC_EXECUTABLE
    name = "meridian-desktop.exe" if platform == "win32" else "meridian-desktop"
    return validator.with_name(name)
