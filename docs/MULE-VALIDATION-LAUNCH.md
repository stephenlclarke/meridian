# Internal World-owned Mule validation entry

The desktop has an explicit internal launch route for G1's actual World/render/contact qualification. Ordinary startup and `--save-dir PATH` retain the commander lifecycle. They do not create a Mule. Only the exact argument pair `--internal-mule-validation front|rear|far|side|dock|dock-obstructed` selects a fresh internal World through the committed World API. Missing/unknown modes, extra flags, mixed save directories and duplicate flags refuse. There is no environment activation, real-profile lookup, fallback or retry.

The internal host has no `Store`, directory, save lease or commander. It does not resolve the default profile directory. New, Continue, Save, XYZZY, equipment and ordinary session commands refuse before doing work. Quit succeeds without saving. The existing World constructor creates the scoped SH001 NPC, validates the physical stationary SH025 observer and owns all simulation state. The renderer consumes that actual immutable publication through the current scope/generation/tick adapter. This is not population, acquisition, paid effects, a player Mule or hull acceptance.

## Source-derived observer presets

The four flight presets start from the authenticated station's actual Mule launch placement and the same revision-2, 34-piece source. Observer radius and lateral extent are read from the exact bundled SH025 native point data, not a display-mesh substitute. Initial axial separation uses both source radii, the existing observer padding and a one-metre margin. The World constructor independently validates the resulting placement; refusal is terminal rather than a request to move it elsewhere.

| Mode | Actual stationary pilot and view |
| --- | --- |
| `front` | Ahead of the source launch, looking back through the normal front camera; Mule advances toward the physical observer. |
| `rear` | Behind the source launch, looking forward at the Mule's rear as it moves away. |
| `far` | Ahead with another 128 metres of separation for the initial beyond-fade view. |
| `side` | Ahead and laterally offset by the source Mule right support plus the native observer left extent, with half a metre of side-plane overlap. Uses the existing read-only Left presentation view from the exact frozen pilot position/orientation. Real World sweeps decide contact; this setup does not declare a clear or contacting trajectory. |

The native SH025 axial point extents are about -60.55 to +59.45 metres and its source-point radius is about 72.55 metres. These are offline encoded-coordinate extrema, not contact or fit results. Axial camera-origin views may therefore reach physical contact with very little visible shimmer inside the 64-metre fade. The side preset provides a source-derived closer grazing view without putting a camera outside the actual physical observer or shrinking either body. Its initial fore/aft separation exceeds the enclosing-sphere requirement; only actual World advancement establishes later clearance/contact. No exact contact tick, minimum camera gap, rendered intensity or native result is claimed.

The side camera selection is presentation-only. It does not call the commander-only view command, rotate the physical observer, change aiming or grant a commander. Station, Mule and other supplied scene primitives use the same World observer frame; sky uses the same actual pilot pose and selected presentation view. The ordinary view selection remains the current World setting.

## Inbound crossing views

`dock` starts the aligned inbound approach at speed 4. `dock-obstructed` starts 69 metres toward +X at speed 33.2 to meet the original SH028 front lip. The +100 metre lane clears the wall and remains an entry-conditions refusal in the World regression. Both use the actual SH028 port centre and begin at world Z 260. The physical SH025 observer sits to the side of each axial trajectory. Its lateral separation is the sum of the Mule origin radius, the padded SH025 radius and a two-metre reserve. The camera looks from that observer between the bearings to the opening pose and the port plane along each inbound lane. The projection regression checks the detailed hull through the approach and terminal pose at the default 1280 by 720 aspect. World checks the actual observer pose and determines the crossing or contact.

The HUD reports the actual Mule phase, speed, tick and any World refusal. At accepted crossing it holds the `Complete` frame. A stopped approach holds the `Approach` frame and displays the actual refusal. The host also holds at 160 ticks if World has published neither outcome. The tick-limit message is distinct from both terminal outcomes. The earlier close presets remain available for shimmer inspection.

## Actual lifecycle and controls

The internal window starts paused. Its HUD and title say internal Mule validation with no commander/save. It shows actual World tick, publication generation and any actual motion refusal. It does not display an invented commander cockpit or resources.

- `P` runs or pauses the existing bounded fixed-clock path.
- `.` advances exactly one real World tick while paused. Key repeat is ignored.
- `Escape` or `Q`, or normal window close, quits without a save attempt.

Movement, fire, EMP/ECM, profile, equipment and help/menu shortcuts are ignored. Held keys cannot enter the internal simulation path: it always calls the real World reducer with default observer input and no fire. Focus loss, occlusion and pause stop host advancement; they do not modify the frozen World observer or create a domain pause authority. No renderer-owned simulation, scripted actor movement or shader state override is introduced.

## Qualification on the internal build

On `f811f128`, the Mac desktop and importer suites passed 310 fresh Rust cases. The 453 unchanged core and compatibility results carry forward from the preceding build. Strict Clippy, the release build and the separate offscreen Metal test passed. Linux and native Windows each passed 313 fresh Rust cases including the three observer cases, and 118 Python tests.

Three Mac launches exited normally. In the far view the Mule grew from a small silhouette inside the station ring to a detailed frontal view. Single-stepping the side view to tick 93 showed the pods broadside, with visible gaps and no prominent halo. These observations cover the sampled frames. The offscreen Metal fixture checks blending, fade and depletion separately. Native contact, station entry and cost measurements remain open.

The docking adapter tests and native crossing views are pending qualification. The World docking test records simulation tick cost separately. On normal internal exit, stderr emits one `MULE_PRESENTED_DRAW_WALL` JSON summary for the lifetime of the process. The count, total, min and max cover wall time from entry to return of `Renderer::draw` for calls that return `Presented`. This covers CPU frame preparation inside the renderer, surface acquisition, submission and return from the present call. It excludes World ticks, skipped or failed draws, GPU execution duration, display completion and the interval between displayed frames. Empty runs report a count and total of zero with null extrema.

## Observed docking results

Mac revision `4288969` passed 767 fresh Rust tests, strict Clippy and the release build. The aligned native run showed the full ship from its opening pose through the accepted crossing. It stopped at tick 104, generation 105, with phase Complete and speed zero. The obstructed run stopped alongside the lip at tick 11, generation 12. It remained in Approach with speed zero and displayed StationContact. Both windows closed normally with Q.

The separate serial release test measured 95th-percentile approach steps of 2.172 ms aligned, 1.832 ms overspeed, 1.610 ms outside the aperture at maximum speed, 1.612 ms outside it at low speed and 1.575 ms approaching the lip. Native draw timings measure wall time inside calls that return Presented. GPU execution time and display cadence remain unmeasured.
