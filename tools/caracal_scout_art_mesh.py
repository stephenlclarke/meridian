"""Bind the calibrated Caracal Scout coordinate scale and shared local grouping."""

from fractions import Fraction

from banteng_art_mesh import assemble

SCALE = Fraction(1_500_000, 171_707)

__all__ = ["SCALE", "assemble"]
