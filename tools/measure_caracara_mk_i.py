"""Measure SH169 local supports against the three calibrated meshes."""

import prepare_caracara_mk_i
from fleet_hull_measurement import HullMeasurement, fidelity


measurement = HullMeasurement(
    prepare_caracara_mk_i,
    {"detailed": 2800, "archivedNative": 3800, "candidateNative": 4400},
    "Caracara Mk I support face spans groups",
)
load = measurement.load
fit = measurement.fit
directed = measurement.directed
directions = measurement.directions


if __name__ == "__main__":
    measurement.main(__doc__, fidelity)
