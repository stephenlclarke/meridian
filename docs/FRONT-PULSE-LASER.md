# SH025 front Pulse Laser

Source baseline `d5308bf757ae1324a0c995d3b9004e1059620d1d` retains the
paid Large Cargo Bay. The front Pulse Laser uses the actual standard birth
item in that same account and hull. The command changes laser resources and
returns the beam endpoint for presentation.

The reference is immutable L337
`a11fe16924d87d06021816f9037952afcbac08db`, tree
`68fec574df12ddf74acb14747e0dd6839f478642`.

| Source | Rule |
| --- | --- |
| `CommanderEquipment.swift:149–157` | Front Pulse Laser is standard, at `.laser(.front)`. |
| `FlightWorkTools.swift:42–54`, `FlightControlInput.swift:100–110`, `Game.swift:6659,6920–6942` | Flight admits the trigger and fires the fitted selected tool. |
| `GameplayWeapons.swift:54–81,134–139` | Front ray starts at the ship origin, points along local +Z, has range 6000 and radius zero. Discharge costs one energy and adds ten heat. Pre-shot bounds require energy over one and heat below 240. |
| `LaserSpecification.swift:21–26` | Pulse cadence is 0.3 seconds. |
| `OriginalFleetR3Data.swift:1036–1053`, `EquipmentModuleTemplates.swift:57`, `Game.swift:611,6935–6942` | Stock SH025 energy 380 and recharge 4.2 per second with the Basic Energy multiplier of one. Laser heat cools by 12 per second. |
| `Game.swift:6160–6177` | Physical dock fills energy and clears heat. Laser cooldown is retained. |
| `MissionNativeBridge.swift:414–437`, `GameplayRaycast.swift:389–431` | Use the first qualified scene obstruction and the finite, two-sided triangle intersection. |

`World::request_fire_front_pulse(expected_tick)` checks current tick, pause,
active Flight mode and exact owner, hull, standard birth item ID and fitted
front slot. The desktop can sample a held trigger after each successful
simulation step. A discharge applies its resource changes once, even when the
beam misses. Cooldown prevents a second discharge at the same tick. Meridian's
Launching publication is a pre-step state, so the first firing opportunity is
its first successful step into Flight. There is one front mount in this
slice.

At 60 Hz the cadence is 18 ticks, the energy increment is seven hundredths
per flight tick, and cooling is one fifth of a heat unit per flight tick.
These exact integer conversions keep readiness and heat thresholds stable.
The source's strict pre-shot heat check allows a shot at 239.8 to reach
249.8, and forbids firing at 240. Docked steps and pause do not advance the
weapon clock. A flight step that physically docks consumes that tick then
applies the source reset.

The ray uses the exact SH028 native station collision triangles already bound
to the World scene. The pinned `sh028.physics` SHA-256 is
`cd54d30cab638f6b33feacf1fa78c9136f8ce346e8cc6133259bfc0472c259d5`.
The existing station manifest describes this 488-triangle collision
representation and its source-frame calibration. A ray along the center
passes through the aperture and axial void. A ray toward the front wall at
x=70, y=0 intersects the surface at z=136.32. The beam ends at the nearest
triangle within range. Reported outcomes are a clear ray or a station
obstruction in this bounded collision mesh.

The current World lacks a live combat target roster with physical identity,
qualified transformed target geometry, and health or impact ownership.
Current station contacts are dock-service actors. A combat hit and its
effects require those missing scene and damage bindings. The firing event
therefore carries station contact geometry and no impact transaction,
station damage, legal consequence, death or payout.

Version 4 stores energy, heat and cooldown as five bytes following the
existing market block. The original account, paid purchase receipt and
validation remain authoritative. The candidate loader validates bounded
resources, the feasible energy/cooldown relationship, heat and docking
constraints, and launch/tick chronology. Version 3 and earlier saves are
refused atomically. The desktop holds the short-lived beam presentation
outside the save.

Hold **Z** during flight to fire. The desktop samples the held trigger after
successful fixed steps, clears it with the existing focus and pause input
reset, and suppresses it during jump spool. The flight HUD displays energy,
heat and cooldown. The existing depth-tested scene draws a short ribbon from
the committed origin to the committed endpoint. A centered pulse can appear
foreshortened in the pilot view, so the HUD also reports a clear shot or a
station obstruction. The desktop never chooses a contact or changes a
discharge's resource cost.

Six authored regressions cover fitted public discharge and refusal, cadence
and thermal recovery, real jump/aim with clear and blocked rays, hot Continue
and resealed save refusal, physical docking resource behavior, and the
bounded native geometry including range edge. A desktop regression covers a
held trigger across fixed ticks and release. The whole joined source needs
independent review and one coordinated Dell execution of format, compiler,
tests and strict Clippy. Desktop acceptance should exercise trigger sampling,
beam endpoint, feedback, and Continue with a hot laser. Rust execution and
desktop acceptance are pending.
