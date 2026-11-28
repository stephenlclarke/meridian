# SH040 Newt Escape Pod source geometry

SH040 is the original cryostasis capsule, not an ordinary purchasable hull. Its current frozen contract permits player flight but explicitly forbids purchase. No acquisition, occupant, rescue, flight or native game authority is introduced by this package. Identity is bound to L337 revision `a11fe16924d87d06021816f9037952afcbac08db` and tree `68fec574df12ddf74acb14747e0dd6839f478642`; all five archived file hashes and Git blob identities match this tree. Its current geometry successor is `docs/fleet-design-v2/successors/SH040-cryostasis.toml` with SHA-256 `5b52208ecf64228a843dc792ff4614af3ff9109034de4bedb6ddb0b7918072d9`.

The original detailed geometry contains 256 vertices and 415 faces (414 triangles and one glass quad). Its Studio representation triangulates only that quad in the original face order, preserving the glass material on both triangles, for 416 detailed triangles. The native shell remains all 48 original vertices and 92 exact triangles. The two shell triangle sets agree exactly after the glass fan. The original `newt-cryostasis` palette is retained. The glass source has opacity 0.25, represented as alpha 64 in the 8-bit Studio schema and retained exactly as source metadata. Optical rendering is not native-qualified.

The adopted scale remains `60/19` metres per original source unit, with the original origin and axes. The original 248 kg dry source budget and centre of mass are unchanged. The source inertia has one mirrored off-diagonal binary64 round-off discrepancy. The exact original tensor in `NEWT-ORIGINAL-TENSOR.json` was refused by the real validator's unchanged symmetry rule. The SH040-specific approved representation in `NEWT-STUDIO.json` sets both entries to their arithmetic mean `-0.4166111063290541` kg·m², from original values `-0.41661110632905385` and `-0.4166111063290544`, respectively changing them by `-2.7755575615628914e-16` and `+2.7755575615628914e-16` kg·m². The original record is byte-identical, every other tensor and physical field is unchanged, both meshes are identical between the original-tensor and normalized packages, and no validator waiver was introduced.

## Source contact and fit

The closed native vessel forms one 0.02 m rounded convex support. Exact support-plane checks prove all 256 detailed points, including the internal decorative occupant, lie inside the native support. The rounded envelope fits both original surface domains with certified 19 mm exposed lower witnesses and 29 mm full upper bounds. These are geometry facts, not claims about a living occupant or transparent material response.

Four independent exterior bypasses have conservative gaps of at least 0.111 m and actual validator acceptance. The four separate solid probes traverse the closed side, lid and both ends; all were refused by the actual validator with exit one. `CONTACT-RESULT.json` binds every exact validator input and raw report by SHA-256, including the original-tensor refusal and normalized acceptance. The returned single support group and all 304 original metre-space points were independently compared for every case.

The exact rational station witness covers all 488 authentic SH028 triangles: the aligned crossing has at least 50.273 m separation, the offset opening 0.978 m, and actual source native vertex 36 crosses strict interior of station face 482 at approximately half of the inward stroke. The fraction is an intersection witness, not a first-contact claim. No game or station entry admission is asserted.

## Reproduction

```sh
python3 -B tools/prepare_newt_escape_pod_studio.py --check
python3 -B tools/verify_newt_escape_pod.py --validator /path/to/meridian-hull-tool
python3 -B tools/measure_newt_escape_pod_fit.py
python3 -B -m unittest discover -s tools -p 'test_newt_escape_pod.py' -v
```

The actual validator receipt is bound to the existing release binary built from Hull Studio source `1789030980e0d2a7897c2dc48518fad728c9bbf2`; fit uses the pinned scientific requirements. The stock Showroom includes the capsule in searches. It keeps the catalogue ship count at 162 and treats Newt as inspection only. The original dry budget is 248 kg. Capsule selection cannot request a ship exchange.

The portable certificate checks all 58 exact supporting planes of the original native point cloud against every detailed vertex. The shared forward vertex supplies a 20 mm exposed witness. Integer subdivision bounds the complete rounded support at 29 mm against both original meshes. Recompute it with `python3 -B tools/measure_newt_escape_pod.py`. The runtime geometry test uses the same stock collision package for the aligned and offset station crossings.
