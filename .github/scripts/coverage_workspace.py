"""Resolve fixed coverage job paths from the checked-in script location."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def workspace_path(relative):
    """Reject traversal and symlinks before opening a CI input or output."""
    relative = Path(relative)
    if relative.is_absolute() or ".." in relative.parts:
        raise ValueError("Coverage path must be a relative path inside the checkout")
    path = ROOT / relative
    if path.resolve() != path:
        raise ValueError(f"Coverage path must not use symlinks: {relative}")
    return path
