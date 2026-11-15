# Reduce ship importer conversion complexity

Sonar reports `python:S3776` on `tools/import_ship.py` at main `e20ec88265700354610c667768886cb00bc36903`: `convert` has cognitive complexity 19 against the configured limit of 15. Its nested triangle/vertex expansion also performs index and finite-position validation inline.

Extract that validation into a small helper. Preserve the immutable source hash gate, notice checks, validation order and errors, source triangle/material/vertex order, colour conversion, binary format and command-line behaviour. Do not suppress the rule, modify source assets or change their provenance claims.

The existing 17 importer tests and a byte comparison of the pinned source's base/candidate/committed mesh provide the relevant verification. Actual Sonar closure belongs to the final reviewed commit analysis.
