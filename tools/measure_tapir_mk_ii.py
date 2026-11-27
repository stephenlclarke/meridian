"""Measure SH106 local supports against the three calibrated meshes."""

import prepare_tapir_mk_ii
from fleet_hull_measurement import HullMeasurement, fidelity


measurement = HullMeasurement(
    prepare_tapir_mk_ii,
    {"detailed": 4750, "archivedNative": 4750, "candidateNative": 5000},
    "Tapir Mk II support face spans groups",
)
load = measurement.load
fit = measurement.fit
directed = measurement.directed
directions = measurement.directions


if __name__ == "__main__":
    measurement.main(__doc__, fidelity)
