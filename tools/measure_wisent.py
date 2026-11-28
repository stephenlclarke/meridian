"""Measure SH096 local supports against the three calibrated meshes."""

import prepare_wisent
from fleet_hull_measurement import HullMeasurement


measurement = HullMeasurement(
    prepare_wisent,
    {"detailed": 2600, "archivedNative": 2600, "candidateNative": 3000},
    "Wisent support face spans groups",
)
load = measurement.load
fit = measurement.fit
directed = measurement.directed
directions = measurement.directions


if __name__ == "__main__":
    measurement.main(__doc__)
