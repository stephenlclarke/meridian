# Hull Studio

Hull Studio is a separate local ship editor. Open it with `make hull-studio`
from the Meridian checkout using Python 3.11 or later. It starts with
Peregrine Mk III, including both original meshes, its palette and its 69
collision pieces. The browser connects only to the loopback server. Stop the
server with Quit Studio or Ctrl+C in its terminal.

Studio uses local HTTP on `127.0.0.1` and does not serve network clients. It checks the local Host and, when sent, Origin. The server serves only its shipped editor assets and the JSON API. Browser uploads do not select paths on the server; use the local CLI to open a split definition or write to an explicit export path.

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
5. In the standalone app, choose **Open in Showroom** to inspect your ship in
   the game. This opens a disposable commander profile and removes it when
   the game exits. In development builds the window title includes the inspected
   ship’s name and local ID. Use those to find its window when several are open.
   You can also pass a new ship to the Meridian desktop executable with
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
Build `meridian-desktop` in release mode as well to include the Showroom.
In a Python environment with PyInstaller installed, run:

```sh
python tools/hull_studio/package_app.py --validator target/release/meridian-hull-tool --game target/release/meridian-desktop --dist OUTPUT
```

On Windows use `meridian-hull-tool.exe`. On macOS pass an existing Developer ID
Application identity with `--signing-identity` for a signed bundle. The output
includes the Rust validator, Python runtime, editor assets, original reference
definition and notices. The installed app opens the editor in the default
browser and can run without a checkout or an internet connection.

With `--game`, the macOS bundle contains a separately identifiable **Meridian
Hull Showroom** helper application. Packaging signs the helper first, then
the outer Studio, so its game window is distinguishable from the editor and
ordinary Meridian. Windows and checkout-based development continue to use the
direct game executable. One Showroom can be open per Studio instance; close
the game normally before inspecting another hull. The first-frame readiness
check and private disposable profile are unchanged.

## Schema 1 limits

The validator accepts a JSON package up to 64 MiB. Each mesh supports up to
100,000 vertices. The detailed mesh supports 200,000 faces and the native
mesh supports 512. A package may have up to 1,024 local groups with 5,344
points per group, counting both meshes, and 256 clearance routes. Rounding
must be positive and at most 20 mm. The gap measured at concave surfaces can
exceed that rounding distance wherever a convex group bridges a recess.

## Archived mesh inspection

Schema 2 accepts archived low-detail artwork for local inspection. Each mesh
can hold up to 200,000 faces. The 64 MiB file limit, 100,000-point limit per
mesh and support-group limits still apply. A schema-2 package can carry native
face material indices. It carries neither a flight specification nor an
engineering qualification.

Build the archive in Studio, then choose **Open in Showroom** if you want to
see it in the disposable game session. The Studio export command refuses
schema 2. To inspect an external schema-2 file in the standalone game, use
`--inspect-hull-package`. The station Showroom can also display a bundled,
authenticated archive for a stock catalogue entry. The [Vulture Mk II archive](../integration/sh038-vulture-mk-ii/ORIGINAL-INSPECTION.json)
retains the original detailed and native artwork and both face-material tables.
Schema 1 retains the 512-face native limit for flight export.

To check this archive against a local clone of the original L337 sources, run
the following command from the Meridian checkout.

```sh
python3 tools/verify_original_stock_inspection.py \
  integration/sh038-vulture-mk-ii/ORIGINAL-INSPECTION.json /path/to/l337
```
