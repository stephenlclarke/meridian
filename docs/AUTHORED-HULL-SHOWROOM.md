# Inspect a local hull package in Meridian

The development desktop game accepts self-contained schema-1 JSON exported by Hull Studio or the shared validator. This is an explicit, read-only session asset; opening it never buys, spawns or replaces a ship in World and never writes the package or data into a save.

Schema-2 archived artwork is also accepted through the explicit `--inspect-hull-package` option only. Its native mesh can exceed the schema-1 flight limit of 512 faces. It has no engineering or flight qualification, cannot be exported for flight from Studio and cannot acquire a hull or authorise flight. See [archived mesh inspection](HULL-STUDIO.md#archived-mesh-inspection).

Run from the repository root with its pinned toolchain:

```bash
cargo run --locked -p meridian-desktop -- --save-dir /path/to/isolated/profile --hull-package integration/authored-hulls/DEMO.json
```

On Commander Details at a station, press **3** to open Ship Showroom, **/** to search `TEST901`, then Enter. Compare initially shows the current owned ship beside the package; use C to see one, the ordinary Pin/turn controls to compare or rotate, and Specifications to inspect the model's actual source dimensions, face and support counts, engineering/flight source values and collision results. Exchange remains unavailable. The pack is absent on an ordinary launch without these flags.

Repeat `--hull-package PATH` or `--inspect-hull-package PATH` to load multiple distinct definitions. A regular package must have a new ID and name; conflicting original or session identities refuse explicitly before a profile opens. For example, `--inspect-hull-package /path/to/SH023.json` makes the local copy searchable as `LOCAL:SH023`, separately from built-in `SH023`. The package source ID remains visible in its specifications. Duplicate loaded source IDs cannot be combined, even across the two flags. The flags cannot be combined with internal Mule validation.

Hull Studio's one-click Open in Showroom uses `--open-authored-showroom` together with exactly one `--inspect-hull-package PATH`, normally in a fresh disposable save directory. It opens the ordinary station Showroom with that isolated `LOCAL:` preview selected on the first frame. It refuses if there is no docked commander, does not change World or bypass save validation, and never repeats selection on later visits. Without the explicit opt-in, Commander Details remains the startup screen.

The explicit launch prints and flushes `MERIDIAN_HULL_STUDIO_READY` once, only after the first successful presentation of that authenticated local preview. Ordinary launches do not print it. Studio and the desktop share the same preview precision check; a detailed triangle that collapses in the fitted `f32` representation refuses preflight. A source ID beginning with `LOCAL:` (case-insensitive, after trimming whitespace) is reserved for inspection and cannot be authored.

Only a regular local file within the shared 64 MiB validator bound is accepted. The shared core validates all fields, material/texture indices, finite UVs, support groups, engineering and native topology limits; any declared blocked or indeterminate clearance refuses the launch. Accepted clearance readouts come from the actual conservative continuous collision kernel, with the same package's rounded pieces also consumed for the local-origin contact readout. Source engineering values and positive clearance results are not acquired mass or flight admission.

The generic renderer submits the detailed face order, winding, exact tint channels including alpha, emissive flags and optional corner UVs against the three shared original atlases. The richer stock display palette is not applied to local assets. Untextured non-emissive surfaces use restrained directional relief; original maps retain baked relief. Partial alpha uses face-centre depth sorting over opaque depth, not order-independent translucency. Presentation uses a uniform fit independent of unscaled collision coordinates and reports physical metres separately. Device and shared core geometry limits still apply; authored assets do not modify built-ins or request external textures.

## Owned verification

The initial and changed fixtures have the same new ID and independent session identity keys. The changed mesh is 5×2×2 m instead of 2×2×2 m, alters tint, partial alpha, emission and texture attributes, and requires moving the previous clear route farther out. Keeping the original route refuses in the actual core validator. The fixtures are independent of the original SH fleet.

```bash
cargo test --locked -p meridian-desktop authored_package
cargo test --locked -p meridian-desktop normal_game_entry_searches_authored_package
cargo test --locked -p meridian-desktop conflicts_and_invalid_inputs_refuse
cargo test --locked -p meridian-desktop authored_shader_and_corner_material_attributes
cargo test --locked -p meridian-desktop one_click_authored_showroom
```

For the Mac validation owner, use a fresh absolute evidence directory:

```bash
MERIDIAN_AUTHORED_HULL_EVIDENCE_DIR=/path/to/new/evidence \
  cargo test --locked -p meridian-desktop metal_authored_package_geometry_and_material_changes_render_in_actual_showroom -- --ignored --nocapture
```

The native test uses actual station frames and the production GPU pass, captures each same-ID package variant and requires distinct nonblack RGB, own-pane confinement and exact repeatability. Portable tests also verify changed geometry/color bytes, identity refusal, normal app entry, unchanged World save bytes and shared-kernel blocked-route denial. On the Mac, the original two authored models yielded 1,795 and 1,348 visible pixels respectively in the production Metal pass. Editing the model changed 2,428 pixels. The save comparison during ordinary navigation uses a matched built-in Showroom control, and entering the package directly also verifies the fresh World stays byte-identical.
