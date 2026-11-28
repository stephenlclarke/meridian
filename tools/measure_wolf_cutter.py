"""Measure SH160 local supports against the three calibrated meshes."""

import prepare_wolf_cutter
from fleet_hull_measurement import HullMeasurement, fidelity


measurement = HullMeasurement(
    prepare_wolf_cutter,
    {"detailed": 2800, "archivedNative": 3800, "candidateNative": 4400},
    "Wolf Cutter support face spans groups",
)
load = measurement.load
fit = measurement.fit
directed = measurement.directed
directions = measurement.directions


if __name__ == "__main__":
    measurement.main(__doc__, fidelity)
