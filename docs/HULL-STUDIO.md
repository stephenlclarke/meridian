# Hull Studio

Hull Studio is a separate local ship editor. Open it with `make hull-studio`
from the Meridian checkout using Python 3.11 or later. It starts with
Peregrine Mk III, including both original meshes, its palette and its 69
collision pieces. The browser connects only to the loopback server. Stop the
server with Quit Studio or Ctrl+C in its terminal.

## Author a ship

1. Open a ship definition or start with the reference. Give a new ship its own
   ID and name. Save your definition to keep work in progress.
2. Set the scale in metres per model unit. Use matching group numbers on the
   detailed and native meshes to enclose those vertices in one smooth piece.
   A convex piece bridges everything between its vertices. Use separate groups
   on opposite sides of an opening you want to keep. Each triangle belongs to
   one group. Omitting the groups lets Studio group each mesh by connected
   components, with separate groups for the two meshes.
3. Add named clearance probes. Their start, end and radius are in metres in
   ship coordinates. A probe represents the space a moving sphere needs.
   Inspect the visible mesh alongside the envelope, rotate the view, and check
   each opening and each reported clearance.
4. Validate and export. Export checks the same definition with the Rust
   collision engine and produces a self-contained JSON package. Editing the
   definition invalidates the preceding result. A failed clearance blocks
   export. A package may carry null engineering data where mass or inertia is
   unknown.
5. Pass a new ship to the Meridian desktop executable with
   `--hull-package PATH`. For a variant sharing a catalogue identity, use
   `--inspect-hull-package PATH` to compare it with the original in the
   Ship Showroom. This workflow adds an inspectable hull. Encounters, purchase
   eligibility and ownership are governed by the campaign.

## Inputs and measurements

Schema 1 includes identity, two indexed triangle meshes, group membership,
scale, rounding, RGB materials, optional alpha and emission, per-face materials,
optional per-corner UVs and original atlas selection, flight speed, optional
mass/centre of mass/inertia, probe routes and provenance. The original fleet
example carries its source hashes and MIT notice. Keep the attribution when
deriving a ship from those assets. The editor's mesh overlay approximates the
analytic collision surface; the runtime tests use the analytic pieces.

The fit readout measures distance to the detailed visible mesh. For the exact
Peregrine reference, the project also has a verified full upper bound. For
edited shapes the readout samples the drawn piece surfaces, including portions
where pieces overlap. The largest sample identifies a location to inspect.
It does not bound the entire envelope.

For split source files, pass `--definition path/to/ship.json` when starting
the editor. For each mesh, materials, face material list or face texture list,
the local definition can use an object such as `{"file":"models/detail.json"}`.
Paths are relative to the definition's directory. Saving from the editor or
exporting in the terminal produces inline data. To export in the terminal:

```sh
python3 tools/hull_studio/studio.py --definition ship.json --export ship.hull.json
```

To select an already built validator, pass `--validator PATH`.

## Standalone desktop build

Build the validator for the host with `cargo build --release --locked -p meridian-hull-tool`.
In a Python environment with PyInstaller installed, run:

```sh
python tools/hull_studio/package_app.py --validator target/release/meridian-hull-tool --dist OUTPUT
```

On Windows use `meridian-hull-tool.exe`. On macOS pass an existing Developer ID
Application identity with `--signing-identity` for a signed bundle. The output
includes the Rust validator, Python runtime, editor assets, original reference
definition and notices. The installed app opens the editor in the default
browser and can run without a checkout or an internet connection.
