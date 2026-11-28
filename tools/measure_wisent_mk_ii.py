"""Measure SH097 local supports against the three calibrated meshes."""

import prepare_wisent_mk_ii
from fleet_hull_measurement import HullMeasurement


measurement = HullMeasurement(
    prepare_wisent_mk_ii,
    {"detailed": 2600, "archivedNative": 2600, "candidateNative": 3000},
    "Wisent Mk II support face spans groups",
)
load = measurement.load
fit = measurement.fit
directed = measurement.directed
directions = measurement.directions


if __name__ == "__main__":
    measurement.main(__doc__)
