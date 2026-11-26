# Medusa inspection

SH171 uses the original detailed artwork in the station Showroom. The source package preserves the original detailed mesh (162,548 faces), the archived native mesh (76,228 faces) and all seven original materials. Each mesh carries 510 separate original emissive projection dots, which form the dotted arcs outside the solid lobes. The archived native mesh exceeds the 512-face flight limit. The two source files are stored as gzip streams. Their decompressed bytes match the Git blobs in the frozen source receipt.

The inspection uses the archive's adopted scale of 5/41 metres per source unit. The 729 local shield supports use both original point streams and 20 mm of rounding. The original continuous crown is covered by 16 overlapping sectors. The proof covers every original face in both meshes. It gives a 1,394 mm upper bound from the rounded supports to their corresponding detailed parts, including internal caps. Each crown sector clears its own axial reference line by at least 5,806 mm. This is a check of the crown sectors. It grants no flight route through the complete ship.

Run `python -B tools/prepare_medusa_inspection.py --check` to compare the packed artwork and source points with the frozen inputs. Run `python -B tools/verify_medusa_shield.py` to check the fan geometry and recompute the fit. The fan is pinned from the original authoring host. Check mode uses its recorded triangles so a different convex hull implementation cannot silently change the evidence.

Run `python -B tools/prepare_medusa_inspection.py --studio-definition /path/to/new-medusa.json` to write the archived inspection definition for the companion Hull Studio change. The definition retains the source position, winding and material of every triangle, including native material indices. Boundary points shared by supports are repeated at the same position for Studio's one-group-per-vertex format. The archive carries no flight clearance or engineering claims in Studio.

## Empty stock reference

The station Showroom lists the empty stock dry mass of 40,712 kg. The finite-equivalent model retains the original 8,640 kg intrinsic hardware allocation and the surface-stock budget. Its centre of mass and full inertia tensor use the verified 5/41 metre calibration. Fourteen closed, separate distributions fit their original owners in both LODs. This model describes mass distribution. It grants no flight authority, occupied space, material qualification or equipment ownership.

The frozen configuration record separates catalogue stock, the limited mission instance, the paid PLUGH stock purchase and the legal maximum player fit. The 32-unit expansion and higher target statistics remain proposals. The current inspection does not issue a quote, change the player's hull or grant equipment.

Run `python -B tools/prepare_medusa_stock.py --check` to verify the Rust reference. To replay the original geometry and independent finite proof on the MacBook Air with the pinned scientific Python, stage a new directory first.

```sh
python -B tools/prepare_medusa_stock.py --stage-legacy /tmp/medusa-stock-replay
cd /tmp/medusa-stock-replay
python -B build-medusa-finite-source.py . finite-stock.json
python -B test-medusa-finite-source.py
```

The original builder and tests retain the 30 CPU-second and 512 MiB limits. The checked source is from archive commit `a11fe16924d87d06021816f9037952afcbac08db` under `docs/milestones/fleet-physical/medusa-finite-proposal`. The replayed `FINITE-STOCK.json` records the exact inputs, plan and builder identities.

### Native preview criteria

1. Open a fresh isolated commander and select Medusa in the station Showroom.
2. In single-ship view, confirm the original artwork and the `Empty stock dry mass: 40712 kg` line. Confirm that the stock line excludes paid upgrades and the flight and PLUGH refusals remain visible.
3. Attempt exchange and confirm refusal. Check that the commander still has SH025, the same balance, and the same equipment. Quit normally, reopen the same isolated slot, and confirm those values persist.
4. Record the signed preview hash, exact Git commit and result. Keep existing player profile directories separate.

## Outstanding integration

- The [flight geometry adapter](../sh171-medusa-flight/README.md) uses the separate 512-face proxy and original SH028 port. Player admission still needs the finite-dry reference and owned-hull integration.
- Player control and purchase need explicit admission, crew and equipment checks, and save/continue qualification.
- The PLUGH fabrication quest, mission encounter, EMP disable, Salvage Beam lattice recovery and transfer of fabrication rights remain separate gameplay work.
