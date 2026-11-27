"""Measure SH098 local supports against the three calibrated meshes."""

import prepare_anoa
from fleet_hull_measurement import HullMeasurement, fidelity


measurement = HullMeasurement(
    prepare_anoa,
    {"detailed": 2600, "archivedNative": 2600, "candidateNative": 3000},
    "Anoa support face spans groups",
)
load = measurement.load
fit = measurement.fit
directed = measurement.directed
directions = measurement.directions


if __name__ == "__main__":
    measurement.main(__doc__, fidelity)
