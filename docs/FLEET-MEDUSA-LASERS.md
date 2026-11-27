# Fleet Medusa laser controls

In the ordinary Fleet app, open the XYZZY menu with **Control+Shift+X** at the main orbital. Review and confirm the Medusa issue. Close the menu and launch. Select **0** for front, **1** for rear, **2** for port (left) or **3** for starboard (right). Hold **Z** to fire the laser on that view. The energy, heat and cooldown readouts apply to all four mounts. Switching view during cooldown does not ready another shot.

The front mount fires the currently installed modular laser. Storing it leaves the front mount empty. The XYZZY hull issue supplies three integral Naval Lasers for the other facings. Each has a distinct item identity tied to the issuing commander and the first issue revision. Repeating the issue or continuing a save retains those identities. Front-mounted mining and salvage tools continue to operate through the front view. The standalone Medusa preview keeps its own flight fixture.

## Integration and verification

The release base is `a2959bd36951917066caec5279684de5f0fba659`. Apply G3's Fleet issue and save changes through `a387b30a9ad1206bb15b08d5e40f26422c223bfd` first. The laser changes use `World::xyzzy_medusa_issue()` for provenance and `World::xyzzy_medusa_active()` for flight input. The combined branch contains both layers. Inspect the laser delta with `git diff a387b30a..HEAD`. Its protected-source inverse covers both layers against the release base.

On the assigned compiler lane run:

```sh
cargo test --locked -p meridian-core sh171_lasers
cargo test --locked -p meridian-desktop sh171_lasers
cargo clippy --locked --workspace --all-targets -- -D warnings
```

For the native gameplay check, use a disposable ordinary Fleet commander. Issue the Medusa, launch, select each view and fire. Check the shot direction, installed front item, three distinct side/rear items and shared cooldown. Save in a side view and Continue. Check that the view, fitted item, energy and cooldown return. Use a separate Peregrine commander to check its existing front fire.

The frozen hull design names the mounts, and the original maximum outfit grants a Naval Laser at each supported mount. The original mount enumeration supplies their names. [SH171 design](https://github.com/stephenlclarke/l337/blob/a11fe16924d87d06021816f9037952afcbac08db/docs/fleet-design-v2/ships/SH171.toml) · [Original outfit grant](https://github.com/stephenlclarke/l337/blob/a11fe16924d87d06021816f9037952afcbac08db/Sources/L337Core/EquipmentMutations.swift#L76) · [Original mounts](https://github.com/stephenlclarke/l337/blob/a11fe16924d87d06021816f9037952afcbac08db/Sources/L337Core/LaserSpecification.swift#L38)
