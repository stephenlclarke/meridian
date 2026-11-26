# Medusa inspection

SH171 uses the original detailed artwork in the station Showroom. The source package preserves the original detailed mesh (162,548 faces), the archived native mesh (76,228 faces) and all seven original materials. Each mesh carries 510 separate original emissive projection dots, which form the dotted arcs outside the solid lobes. The archived native mesh exceeds the 512-face flight limit. The two source files are stored as gzip streams. Their decompressed bytes match the Git blobs in the frozen source receipt.

The inspection uses the archive's adopted scale of 5/41 metres per source unit. The 729 local shield supports use both original point streams and 20 mm of rounding. The original continuous crown is covered by 16 overlapping sectors. The proof covers every original face in both meshes. It gives a 1,394 mm upper bound from the rounded supports to their corresponding detailed parts, including internal caps. Each crown sector clears its own axial reference line by at least 5,806 mm. This is a check of the crown sectors. It grants no flight route through the complete ship.

Run `python -B tools/prepare_medusa_inspection.py --check` to compare the packed artwork and source points with the frozen inputs. Run `python -B tools/verify_medusa_shield.py` to check the fan geometry and recompute the fit. The fan is pinned from the original authoring host. Check mode uses its recorded triangles so a different convex hull implementation cannot silently change the evidence.

Run `python -B tools/prepare_medusa_inspection.py --studio-definition /path/to/new-medusa.json` to write the archived inspection definition for the companion Hull Studio change. The definition retains the source position, winding and material of every triangle, including native material indices. Boundary points shared by supports are repeated at the same position for Studio's one-group-per-vertex format. The archive carries no flight clearance or engineering claims in Studio.

## Outstanding integration

- Flight needs a separately qualified low-detail mesh within 512 faces and an admitted finite-dry engineering record. The archived engineering budget records 40,712 kg at the adopted scale.
- Player control and purchase need explicit admission, crew and equipment checks, and save/continue qualification.
- The PLUGH fabrication quest, mission encounter, EMP disable, Salvage Beam lattice recovery and transfer of fabrication rights remain separate gameplay work.
