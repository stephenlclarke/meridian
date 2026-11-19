# SH033 Panther Scout hull definition

The original Panther Scout data is bound to immutable L337 revision `a11fe16924d87d06021816f9037952afcbac08db`, tree `68fec574df12ddf74acb14747e0dd6839f478642`. The Scout resource, native export and contract are independent of SH032 Panther Explorer; the original family registration and physical-contract references distinguish the two variants. Their checksums and the original MIT terms are retained in the authored definition. The shared original texture bytes are reused from the existing SH001 assets without duplicating them.

`PANTHER-SCOUT-STUDIO.json` is a schema-1 data-defined hull. It preserves the original metre-space detailed and native triangle positions and ordering, forest-ivory palette, per-face material indices and UVs. The original 2,672 detailed vertices / 5,212 faces and 188 native vertices / 320 faces become 2,744 and 212 support-indexed vertices respectively, with unchanged faces. Only arch-band seam vertices are duplicated. Thirty-five local rounded convex support groups include the Scout's own axial survey spine and paired sails. The arch bands keep the side openings clear. Engineering is unassigned (`null`); no new hull admission, combat configuration, or player ownership is claimed.

`CLEARANCE-FIXTURES.json` records four positively separated open paths, four solid convex-support midpoint witnesses, and aligned and offset original SH028 aperture projection measurements. The conservative lower gaps for the arch openings are 0.58 m and for the upper Scout openings approximately 0.753 m, after rounding and a 0.10 m probe radius. The rectangular station aperture is 124.64 by 101.6 m. Its aligned projection margins are approximately 52.471 m horizontally and 43.22 m vertically; a 60 m lateral offset exceeds the horizontal aperture by 7.529 m. These are projection checks, not actual station mesh sweep acceptance.

## Reproduction

```sh
python3 -B tools/prepare_panther_scout_studio.py --check
python3 -B -m unittest discover -s tools -p 'test_panther_scout_studio.py' -v
python3 -B tools/prepare_panther_scout_studio.py --check --validator /absolute/path/to/meridian-hull-tool
```

The optional validator argument uses the already-built actual executable, not a mock or a compiler. `VALIDATOR-RECEIPT.json` records the actual author-stage run using the release binary from the earlier Hull Studio source `1789030980e0d2a7897c2dc48518fad728c9bbf2` (tree `fb22a072408e94b4553ea9c5e586916ff7e6ecae`): four open paths passed together with exit zero, and each of the four solid paths independently refused with exit one. The validator and input hashes are retained. Exact joined-head platform execution, independent source review, native Showroom inspection, physical station sweep and runtime integration remain separately owned and pending.
