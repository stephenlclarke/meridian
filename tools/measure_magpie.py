"""Measure SH107 local supports against the three calibrated meshes."""

import prepare_magpie
from fleet_hull_measurement import HullMeasurement, fidelity


measurement = HullMeasurement(
    prepare_magpie,
    {"detailed": 2000, "archivedNative": 2800, "candidateNative": 3300},
    "Magpie support face spans groups",
)
load = measurement.load
fit = measurement.fit
directed = measurement.directed
directions = measurement.directions


if __name__ == "__main__":
    measurement.main(__doc__, fidelity)
