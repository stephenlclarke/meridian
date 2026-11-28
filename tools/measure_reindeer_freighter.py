"""Measure SH150 local supports against the three calibrated meshes."""

import prepare_reindeer_freighter
from fleet_hull_measurement import HullMeasurement, fidelity


measurement = HullMeasurement(
    prepare_reindeer_freighter,
    {"detailed": 2800, "archivedNative": 3800, "candidateNative": 4400},
    "Reindeer Freighter support face spans groups",
)
load = measurement.load
fit = measurement.fit
directed = measurement.directed
directions = measurement.directions


if __name__ == "__main__":
    measurement.main(__doc__, fidelity)
