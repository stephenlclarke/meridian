# Medusa player flight preview

The explicit SH171 flight preview uses the authenticated 512-triangle hull and 70 rounded supports from the [Medusa station adapter](../sh171-medusa-flight/README.md). It gives the local pilot the original stock speed, acceleration and turn rates. It uses the same World launch, fixed-tick flight and physical SH028 docking commands as the ordinary game.

Launch the built desktop executable with a dedicated preview option. The directory can be new or can already contain an ordinary commander; the preview uses its own file and independent exclusive lease.

```sh
meridian-desktop --medusa-preview-dir /path/to/preview-slot
```

The preview starts docked or continues its own existing save. L launches. W/S adjust throttle, A/D yaw, up/down arrows pitch, Q/E roll and Space brakes. 0–3 change the flight view. P pauses. F5 saves, F9 continues and Escape saves and quits.

While docked, B fits or stores the preview cargo expansion. Brackets reduce or increase the number of fitted preview cabins. The fit is limited to four cabins, 5 t and one equipment unit per cabin, and 22 t of stock hold with a 10 t cargo expansion. The preview also checks the stock limits of 13 equipment units and four utility mounts. Fit changes publish the prospective checked save before altering the live fixture. The pilot cannot fit while launching or in flight.

`medusa-preview.meridian` stores the checked SH171 flight pose, flight view, fixture fit and exact pinned content fingerprint. It cannot open as a commander save; standard commander bytes cannot open as a preview. Reload rejects malformed, over-capacity, invalid-position and corrupt files. An existing ordinary `commander.meridian` is not read or replaced by preview operations.

This mode is an isolated test entitlement, not PLUGH fabrication, a paid stock hull, a maximum upgrade, or a mission actor. It has no ordinary commander identity, market, wallet, quest receipts, passengers, or service authority. It does not grant a source-unsupported intrinsic attachment or a paid upgrade. The ordinary Showroom and progression keep their existing acquisition refusals.

In flight, the active World hull identity selects the original 162,548-face artwork. The GPU retains the detailed mesh and uses a closer chase camera in the shared flight projection. Near the station, the existing first-person view remains in use. The 512-face flight contact geometry is independent, and the archived 729-piece inspection shield is not rendered in flight.

Native acceptance must check the displayed SH171 identity, the actual geometry on launch and return, fit limit refusal and cold Continue. The source tests do not substitute for signed installed preview acceptance.
