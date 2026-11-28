"""Validate authored triangulations for deterministic collar and ring measurements."""

import json

import numpy as np

from vulture_mk_i_exact_fit import bound_fan


def load_span_fans(path, supports):
    records = json.loads(path.read_bytes())
    if records.keys() != set(supports) or any(row.keys() != {"4", "8"} for row in records.values()):
        raise ValueError("Unexpected authored span-fan inventory")
    return records


def validate_span_fans(vertices, spans, records):
    fans = []
    for span, faces in zip(spans, records, strict=True):
        ids = np.asarray(span)
        fan = np.asarray(faces)
        if fan.ndim != 2 or fan.shape[1] != 3 or not len(fan) or fan.dtype.kind not in "iu":
            raise ValueError("Expected authored span-fan triangles")
        local = np.searchsorted(ids, fan)
        if np.any(local >= len(ids)) or not np.array_equal(ids[local], fan):
            raise ValueError("Fan crosses the original support span")
        bound_fan(vertices[ids], local)
        fans.append(fan)
    return fans
