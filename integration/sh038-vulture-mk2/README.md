# SH038 Vulture Mk II source and separate derived candidate

Immutable L337 revision `a11fe16924d87d06021816f9037952afcbac08db`, tree `68fec574df12ddf74acb14747e0dd6839f478642`. The original supplied geometry and physical source remain unmodified in `assets/fleet/sh038`. The archived copy hashes were independently matched to both the original revision and its Git blob identities after workspace recovery:

| Archived file | Original Git blob |
| --- | --- |
| LICENSE | `b27f022cea044295412849dec93512dad638f08e` |
| contract.toml | `1896a2b05a638780f12f362acc681e48b772f9d5` |
| engineering.json | `5ab98f14fa4f3d145085a6adcee47de498407f0e` |
| native-export.json | `42678a49ee0366f7acd147be47f41e054bb2ada6` |
| resource.toml | `41b36b5bde2ca4b1fee9aac1ca44af9a42139b52` |

The original relative pending-source registration is distinct from admission. The original native archive has 1,812 vertices and 3,016 faces, whereas its contract and Studio cap are 512 faces. `ORIGINAL-ARCHIVE-CANDIDATE.json` retains both exact original meshes and their topology, Copper / graphite / sand palette and face-material assignments. `ORIGINAL-REFUSAL.json` records the real validator exit one with the exact 512-face cardinality refusal. No cap or importer was changed.

The original authored dry engineering budget is retained without modification: 57,625 kg, original centre of mass and inertia tensor and the adopted dimensional calibration of `6000/611` metres per source unit (30 m nominal detailed length). It is a source reference, not measured physical engineering, loaded actor, or operational authority.

## Separate derived native approximation

`DERIVED-NATIVE-CANDIDATE.json` is a distinct newly authored experiment, not a replacement for the original native archive. It preserves all 8,272 detailed source faces with unchanged positions and ordering, duplicates only collar sector seam indices, and retains the actual face-material assignments. The new native proxy uses 324 selected original native vertices and exactly 512 faces across separate major supports without joining opposing parts. The 360-degree viewing belt and two hollow rails are split into eight local sectors apiece; the smaller original detail components remain distinct. There are 522 local rounded collision supports in the combined package. The reduced native mesh omits smaller original surface details by design. It is not a recovered or approved per-ID native successor.

Actual validator evidence accepts four open paths with lower independent gaps of 0.151 m and 0.726 m, and refuses four separate solid support traversals. `CONTACT-RESULT.json` independently certifies every rounded local support against all 488 original SH028 station triangles: the aligned sweep has a 43.865 m lower separation; the offset opening has 0.979 m lower separation; the offset sweep crosses the strict interior of station lip face 482 with an actual original source vertex. The crossing parameter near 0.5 is a witness, not the time of first contact. The validator was the existing release binary from Hull Studio source `1789030980e0d2a7897c2dc48518fad728c9bbf2`; neither a compiler nor a game session was started.

`SURFACE-FIT.json` records separate bounded rounded-envelope comparison to both original meshes: detailed 0.178–1.017 m, native 0.343–1.590 m. The upper bounds conservatively include all rounded pieces, even overlaps. For the new native visual proxy itself, the full-surface upper bounds to original detailed and native meshes are 1.819 m and 1.816 m; the reverse original-to-proxy bound is 2.300 m in both domains, with strict lower witnesses of 2.046 m. The difference is material and explicitly preserved. These bounds are not proof of source equivalence or a request to waive the original source policy.

## Reproduction and limits

```sh
python3 -B tools/prepare_vulture_mk2_source.py --check --validator /path/to/meridian-hull-tool
python3 -B tools/prepare_vulture_mk2_candidate.py --check
python3 -B tools/verify_vulture_mk2_candidate.py --validator /path/to/meridian-hull-tool
python3 -B tools/measure_vulture_mk2_candidate.py
python3 -B -m unittest discover -s tools -p 'test_vulture_mk2_candidate.py' -v
```

The approximation and fit computations use the existing pinned scientific requirements and must keep Python assertions enabled. The game/Showroom has not been changed to load either candidate; the original registration identifies the exact 3,016-face pending native and the derived proxy has no original approval. A future independently reviewed reduced-native admission would be a separate policy decision. Native platform and actual-game qualification remain unrun. No weapons, combat, acquisition, or player ownership behavior has been introduced.
