# Medusa player flight preview

The explicit SH171 flight preview uses the authenticated 512-triangle hull and 70 rounded supports from the [Medusa station adapter](../sh171-medusa-flight/README.md). It gives the local pilot the original stock speed, acceleration and turn rates. It uses the same World launch, fixed-tick flight and physical SH028 docking commands as the ordinary game.

Launch the built desktop executable with a dedicated preview option. The directory can be new or can already contain an ordinary commander; the preview uses its own file and independent exclusive lease.

For the packaged follow-up, open the separate **Meridian Medusa Preview.app** on macOS or **Launch Medusa Preview.cmd** on Windows. These launchers must pass only the Medusa preview option and bind a new release-specific preview directory. They must not invoke the ordinary launcher's hard-coded profile arguments. The exact installed names and paths are recorded by the release packagers.

```sh
meridian-desktop --medusa-preview-dir /path/to/preview-slot
```

The preview starts docked or continues its own existing save. L launches. W/S adjust throttle, A/D yaw, up/down arrows pitch, Q/E roll and Space brakes. 0–3 change the flight view. P pauses. Ctrl+S saves, Ctrl+R continues and Escape saves and quits.

While docked, B fits or stores the preview cargo expansion. Brackets reduce or increase the number of fitted preview cabins. The fit is limited to four cabins, 5 t and one equipment unit per cabin, and 22 t of stock hold with a 10 t cargo expansion. The preview also checks the stock limits of 13 equipment units and four utility mounts. Fit changes publish the prospective checked save before altering the live fixture. The pilot cannot fit while launching or in flight.

`medusa-preview.meridian` stores the checked SH171 flight pose, flight view, fixture fit and exact pinned content fingerprint. It cannot open as a commander save; standard commander bytes cannot open as a preview. Reload rejects malformed, over-capacity, invalid-position and corrupt files. An existing ordinary `commander.meridian` is not read or replaced by preview operations.

An actual flight can pass around and behind the station port, not only in front of it. The save gate accepts any clear flight pose and requires the exact station launch pose when saved in Launching mode. The pinned content fingerprint covers the station geometry and port, Medusa collider/supports, original configuration record and fitted capacity and flight rate rules.

This mode is an isolated test entitlement, not PLUGH fabrication, a paid stock hull, a maximum upgrade, or a mission actor. It has no ordinary commander identity, market, wallet, quest receipts, passengers, or service authority. It does not grant a source-unsupported intrinsic attachment or a paid upgrade. The ordinary Showroom and progression keep their existing acquisition refusals.

In flight, the active World hull identity selects the original 162,548-face artwork. The GPU retains the detailed mesh and uses a closer chase camera in the shared flight projection. Near the station, the existing first-person view remains in use. The 512-face flight contact geometry is independent, and the archived 729-piece inspection shield is not rendered in flight.

## Native validation

On 26 November 2026, G1 built and signed exact source `5a9243153b7d05b3d268fa9a82d30df5261a0e9a` (tree `61214e3e28f6b2830cfd5c71d52482e049f29ef3`). The Orchestrator observed the native Mac process using an isolated profile. From a docked start, B fitted the cargo expansion, L launched, W accelerated and F5 saved. The chase view showed the original crown, lobes and cyan dotted artwork. The HUD showed FLIGHT, 50.30/50.30, 0/32 t and MEDUSA PREVIEW SAVED.

The exact core decoder read SH171, FLIGHT, speed 50.3, cargo expansion fitted and no ordinary commander identity or wallet. It re-encoded the 69-byte save without changing a byte; the ordinary decoder refused the save. The first process closed, and a separate cold process reopened the same isolated slot. The Orchestrator confirmed the same Medusa model, flight mode, 50.30 speed and 0/32 t hold. Both process exits preserved the save bytes. The signed build passed the post-run strict signature check.

Core regressions at that exact source also passed the stock-rate launch and physical docking return, legal rear-side save and fit-limit checks. Native fit-limit refusal and physical docking return were not observed in the interactive run. Acceptance of those native cases and final packaged release qualification remain open. No real commander save was used, and this evidence grants no ordinary ownership or progression.
