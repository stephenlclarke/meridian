"""Bind Wombat to its original length-derived calibration and local groups."""

from fractions import Fraction
from banteng_art_mesh import assemble

SCALE = Fraction(23_000, 1_211)

__all__ = ["SCALE", "assemble"]
