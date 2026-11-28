"""Validate local asset destinations and atomically replace regular file entries."""
import os
from pathlib import Path
import stat
import tempfile


def temporary_path(path):
    """Normalize the system temporary directory alias, without resolving user links."""
    temporary = Path(tempfile.gettempdir()).absolute()
    scratch = temporary.resolve(strict=True)
    if path.is_relative_to(temporary):
        path = scratch / path.relative_to(temporary)
    return path, scratch


def checked_directory(path):
    if not path.is_dir() or path.resolve(strict=True) != path:
        raise ValueError('Asset directory is missing or redirects')
    return path


def checked_file(path):
    if path.is_symlink() or path.resolve() != path or (path.exists() and not path.is_file()):
        raise ValueError('Output must be a regular file without redirects: ' + path.name)
    return path


def replace_output(path, data):
    """Replace the destination entry without writing through existing file links."""
    try:
        mode = stat.S_IMODE(path.stat().st_mode) & 0o777
    except FileNotFoundError:
        mode = None
    sibling = None
    try:
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix='.asset-output-', delete=False) as stream:
            sibling = Path(stream.name)
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        if mode is not None:
            sibling.chmod(mode)
        sibling.replace(path)
        sibling = None
    finally:
        if sibling is not None:
            try:
                sibling.unlink(missing_ok=True)
            except PermissionError:
                # Windows cannot remove a read-only file after a failed replacement.
                sibling.chmod(0o600)
                sibling.unlink(missing_ok=True)
