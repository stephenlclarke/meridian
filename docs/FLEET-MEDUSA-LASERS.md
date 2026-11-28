# Fleet Medusa laser fitting and controls

The Fleet Medusa has four laser mounts. Its first XYZZY issue fits a Naval Laser to each one. The Peregrine has one front laser mount.

## Fit a laser

At a station, open **5 Equipment** and choose **Front laser**, **Port laser**, **Starboard laser** or **Aft laser**. Each mount shows its installed class. Select a compatible class to buy at that station, or select an item in storage to fit. Review the mount, item and price, then confirm. The displaced gun goes into storage with its original identity and purchase record. Other mounts keep their fits.

To move an installed gun to another mount, store it first, then fit it from the destination mount. An empty mount cannot fire. Ordinary purchases use the station’s technology level, the listed price, available credits and ownership limits. An issued Naval Laser can be stored and fitted again. [Laser classes, prices and firing intervals](FAST-LASERS.md#original-profiles-and-paid-custody)

## Fire the selected mount

Select **0** for front, **1** for aft, **2** for port or **3** for starboard. Hold **Z** to fire along the selected view. The installed class determines damage and the interval between shots. All four mounts draw from the same energy supply and share heat and cooldown. Switching views during cooldown keeps the remaining delay.

The Mining Laser can fire from a combat mount. Asteroid extraction uses the front tool. **Continue** restores the fitted and stored items. Reissuing the Medusa or selecting its maximum outfit refits its owned Naval Lasers and stores the guns they replace.

The frozen hull design names the mounts, and the original maximum outfit grants a Naval Laser at each supported mount. The original mount enumeration supplies their names. [SH171 design](https://github.com/stephenlclarke/l337/blob/a11fe16924d87d06021816f9037952afcbac08db/docs/fleet-design-v2/ships/SH171.toml) · [Original outfit grant](https://github.com/stephenlclarke/l337/blob/a11fe16924d87d06021816f9037952afcbac08db/Sources/L337Core/EquipmentMutations.swift#L76) · [Original mounts](https://github.com/stephenlclarke/l337/blob/a11fe16924d87d06021816f9037952afcbac08db/Sources/L337Core/LaserSpecification.swift#L38)
