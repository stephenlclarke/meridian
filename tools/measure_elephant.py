"""Measure SH090 local supports against the three calibrated meshes."""

import prepare_elephant
from fleet_hull_measurement import HullMeasurement


measurement = HullMeasurement(
    prepare_elephant,
    {"detailed": 3120, "archivedNative": 3120, "candidateNative": 3000},
    "Elephant support face spans groups",
)
load = measurement.load
fit = measurement.fit
directed = measurement.directed
directions = measurement.directions


if __name__ == "__main__":
    measurement.main(__doc__)
