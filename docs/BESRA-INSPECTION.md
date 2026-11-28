# Besra stock inspection

SH108 is Besra. Its original detailed mesh has 2,078 vertices and 4,044 faces. Its selected native mesh has 122 vertices and 208 faces. Both mesh streams, the palette and both sets of material indices remain in original order. The detailed renderer uses the authored texture indices and UVs. The original native UVs remain available in the native export.

The archived native coordinates are in metres. At L337 revision `0d8cc9ebe0018bf74b0d9fa10ef23b70339dd6b0`, `OriginalHullPhysicalContract.calibrationCase13` checks Besra’s identity, contract and native geometry hash before applying one metre per source unit. The package groups the 28 detailed components and nine native components by their connected indices. The engineering field is empty.

The MBA rebuilt the package from four pinned inputs. Its Studio export matches the checked-in package up to the final line feed. The validator returned all 2,200 original points across 37 groups and rejected a 513-face control. The ordinary station Showroom has a stock entry and a desktop regression for stock and local selection. Live station rendering and platform testing remain with their owners.

## Reproduce

```sh
python3 tools/prepare_besra.py --check
cargo run --locked -p meridian-hull-tool -- validate \
  integration/sh108-besra/BESRA-STUDIO.json --report /tmp/besra-report.json
```

Use a new report path for each run. [Source inputs and MIT notice](../assets/fleet/sh108) · [Stock package](../integration/sh108-besra/BESRA-STUDIO.json)
