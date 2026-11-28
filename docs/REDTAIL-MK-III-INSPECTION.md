# Redtail Mk III stock inspection

SH103 is the Redtail Mk III. Its original L337 resource has 1,216 vertices and 2,328 faces in the detailed mesh. The selected native archive has 84 vertices and 140 faces. The package preserves both meshes, the original palette and per-face materials. The detailed renderer uses the original three surface maps and UV coordinates. The native UV tables remain in the archived source.

The L337 archive identifies the native coordinates as metres. At revision `d03f1b4ecfd7588bb6b27763e31c230ebe1fbf5a`, `OriginalHullPhysicalContract.calibrationCase12` matches the SH103 source path and geometry hash and applies one metre per source unit. The detailed bounds are 22 m wide, 14.04 m high and 37 m long. The package retains the authored source speed and leaves engineering unspecified.

The MBA Studio export matched the checked-in definition except its final line feed. The Rust validator returned all 1,300 scaled support points exactly across 26 detailed groups and seven native groups. These groups follow original index-connected components. The stock entry provides a station Showroom preview. Native rendering, in-game collision and player acquisition have separate acceptance criteria.

## Reproduce

```sh
python3 tools/prepare_redtail_mk_iii.py --check
cargo run --locked -p meridian-hull-tool -- validate \
  integration/sh103-redtail-mk-iii/REDTAIL-STUDIO.json --report /tmp/redtail-report.json
```

The validator writes to a new report path. [Original assets and MIT notice](../assets/fleet/sh103) · [Studio package](../integration/sh103-redtail-mk-iii/REDTAIL-STUDIO.json)
