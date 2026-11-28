# Meridian

[Download Meridian Preview 12 for macOS arm64 or Windows x64](https://github.com/stephenlclarke/meridian/releases/tag/v0.12.0-preview.1). Published 28 November 2026 from protected main `83ceae733d6e4f9cffc81c517fabb3eb7b39fba8`. The archive contains Fleet, the standalone Medusa and Hull Studio. [Installation and platform checks](docs/PREVIEW-RELEASES.md#fleet-medusa-and-hull-studio)

Start **Fleet** to fly your commander. At a station, press **3** to inspect 115 Showroom models. To fly a fully fitted SH171 Medusa in your Fleet save, use the XYZZY refit described below. The standalone **Medusa Preview** uses a separate save. Fleet and Medusa require macOS 14 or later on Apple silicon. Hull Studio requires macOS 27 or later. The Windows x64 package is unsigned. Interactive Windows gameplay and Showroom rendering remain unverified.

At a station, open **5 Equipment** to buy, store or refit the Fleet Medusa’s front, aft, port and starboard lasers independently. In flight, use **0–3** to select a view and hold **Z** to fire its fitted weapon. [Fleet Medusa laser controls](docs/FLEET-MEDUSA-LASERS.md)

The centre sight identifies the active laser or work tool. Press **H**, then **I**, for Flight Status. Press **N** on that card to choose where notices appear. [Cockpit instruments](docs/COCKPIT-HUD.md)

The ordinary-drive development candidate adds the Basic, Intermediate and Advanced hyperspace drive slot to Equipment. Paid upgrades change the actual jump countdown and station mass lock; explicit XYZZY maximum fitting uses the Advanced drive. The [equipment implementation and testing ledger](docs/EQUIPMENT-PLAYABLE.md) records all catalogue grades, remaining gameplay paths and Medusa compatibility constraints. This candidate is not part of Preview 10 and awaits owned validation.

The stock-shield development candidate adds seven independently testable shield grades, an actual finite bank capacity for each grade, four ordinary paid upgrades and a dedicated XYZZY test selector. It preserves the original maximum outfit and the distinct unresolved Elite-rank and PLUGH-only acquisition paths. [Shield source, fitting and test controls](docs/SHIELD-GRADES.md). Owned validation is pending.

The [fuel storage development candidate](docs/FUEL-STORAGE.md) adds four separately owned repeatable units, actual paid refuelling and capacity changes. SH025 has three shared normal utility mounts; the explicitly labelled four-tank test maximum uses a guarded exception. Owned validation is pending.

The [solar fuel development candidate](docs/SOLAR-FUEL-SCOOPS.md) adds actual near-sun fuel collection with fitted scoops, independent cabin-temperature danger and retained collection receipts. Physical cargo and ore intake remain separate. Owned validation is pending.

The passenger-cabin development candidate offers all six original paid cabin layouts, with real five-tonne fit costs and separate Avenor-to-Renas G04 passenger consent, boarding and reception. See [passenger cabins and G04 controls](docs/PASSENGER-CABINS.md). Owned validation is pending.

In Preview 12, press **Tab** to mark the nearest contact. Press again to move through the others by distance. **F** switches between hand flight and the assistance your equipment provides. You can aim and fire at ships, fly a docking approach, mine or salvage by hand. [Flight controls and required equipment](docs/AUTOMATIC-WORK.md)

In a normal Fleet commander slot, dock and press **Ctrl+Shift+X**. Choose **Activate / refit SH171 Medusa**, press **Enter** to review, then **Enter** to confirm. Launch from that berth to fly the issued ship. Its 32-tonne expanded hold, five loaded missile positions and fitted equipment travel with the commander through Save and Continue. Existing CLARKE commanders use the same row and keep their current credits. [Medusa test outfit and limits](docs/EQUIPMENT-PLAYABLE.md#fleet-xyzzy-medusa)

## Goshawk Explorer in the Ship Showroom

In Preview 10, press 3 at a station and search for Goshawk Explorer. Compare shows the Peregrine Mk V beside the Goshawk for a new commander. Press C for a single-ship view. Mac testing verified the original dark green hull, silver cross-member, open wishbone and refusal of an unavailable exchange. Preview 9 predates this ship. [Goshawk geometry and Showroom controls](docs/GOSHAWK-GAMEPLAY-ENVELOPE.md)

The development source also provides a separate Goshawk Mk II Custom Showroom and collision candidate. Its original detailed appearance, four lower mounts and SH028 fit evidence are in the [SH010 hull guide](docs/GOSHAWK-MK-II-CUSTOM.md). Owned native validation is pending.

Medusa (SH171) can be inspected in the station Showroom. The source package preserves both original meshes and the inspection shield. The Showroom lists the verified empty stock dry mass. [Medusa inspection and flight limits](integration/sh171-medusa/README.md)

Preview 12 offers 115 selectable stock previews in the ordinary station Showroom. This includes 109 validated Hull Studio schema-1 fleet packages and six original stock models. That release excluded Cheetah (SH082) and 22 entries in the final fleet intake. These are catalogue previews, not new ships for purchase or flight. [Hull Studio](docs/HULL-STUDIO.md) also opens a local or archived preview in a disposable Showroom session without changing the player's ship. The earlier 37-preview Mac build passed the stock loader and station Showroom tests; the final Mac package displayed a native nested Showroom frame. Interactive Windows Showroom rendering remains unverified. [Outstanding fleet scope](docs/IMPLEMENTATION-STATUS.md#outstanding-fleet-inspection-scope)

The development source adds the [Serval Fighter](docs/SERVAL-GAMEPLAY-ENVELOPE.md), [Vulture Mk I](docs/VULTURE-MK-I-GAMEPLAY-ENVELOPE.md), [Newt Escape Pod](integration/sh040-newt-escape-pod/README.md), [Zebu](docs/ZEBU-GAMEPLAY-ENVELOPE.md), [Panther Scout](integration/sh033-panther-scout/README.md), [Panther Explorer](docs/PANTHER-EXPLORER-GAMEPLAY-ENVELOPE.md), [Okapi](docs/OKAPI-GAMEPLAY-ENVELOPE.md), [Ibex](docs/IBEX-GAMEPLAY-ENVELOPE.md) and [Harrier](docs/HARRIER-GAMEPLAY-ENVELOPE.md) for stock inspection.

Stock inspection also includes Vulture Mk II (SH038) and Vulture Mk III (SH039). Each archive keeps all 8,272 detailed faces and 3,016 original native faces.

The development build adds [Cheetah (SH082)](docs/CHEETAH-SHIELD.md) to the station Showroom. Its 512-face native candidate retains the 24 measured original contacts, including the four service-well contacts lost in the earlier candidate.

The development build adds [Redtail Mk III (SH103)](docs/REDTAIL-MK-III-INSPECTION.md) to the station Showroom. Its original 2,328-face detailed mesh and 140-face native mesh use the source calibration of one metre per unit.

The development catalogue also includes [Besra (SH108)](docs/BESRA-INSPECTION.md), [Salamander Station (SH141)](docs/SALAMANDER-STATION-INSPECTION.md), [Lion Bastion (SH156)](docs/LION-BASTION-INSPECTION.md) and [Wolf Citadel (SH161)](docs/WOLF-CITADEL-INSPECTION.md). The source Showroom now has 132 previews.

The separate [Medusa player flight preview](integration/sh171-medusa-preview/README.md) uses a dedicated profile and the admitted reduced flight geometry. On signed Mac build `5a924315`, the original SH171 artwork was visible in actual flight at 50.30/50.30. A cold restart restored the same ship, speed and expanded 32 t hold. Native docking return and fit-limit refusal were not observed in that run. Ordinary progression, fabrication and exchange do not grant the Medusa. See the [Preview 12 validation record](docs/IMPLEMENTATION-STATUS.md#published-preview-12) for final-package checks.

## Internal Mule docking validation

The internal SH001 Mule scene uses the original SH028 station. On Mac build `ffb7112`, Mule entered the clear opening at tick 104. On the obstructed approach it stopped at the station lip at tick 11. In the development build, completing all eight G02 freight handovers unlocks a single Mule encounter on the next launch from Renas. A returning pilot whose saved ship overlaps the Mule’s berth can fly clear before the Mule appears. Preview 8 predates this encounter. The reviewed renderer build `1cdac5e` also passed the aligned crossing on Mac and the docking regressions on Linux and native Windows. [Controls and test scope](docs/MULE-VALIDATION-LAUNCH.md)

[![CI](https://github.com/stephenlclarke/meridian/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/stephenlclarke/meridian/actions/workflows/ci.yml)
[![Quality Gate](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=alert_status)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=coverage)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Bugs](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=bugs)](https://sonarcloud.io/project/issues?id=stephenlclarke_Meridian&types=BUG)
[![Vulnerabilities](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=vulnerabilities)](https://sonarcloud.io/project/issues?id=stephenlclarke_Meridian&types=VULNERABILITY)
[![Code smells](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=code_smells)](https://sonarcloud.io/project/issues?id=stephenlclarke_Meridian&types=CODE_SMELL)
[![Maintainability](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=sqale_rating)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Duplicated lines](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=duplicated_lines_density)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Lines of Code](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=ncloc)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Reliability Rating](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=reliability_rating)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Security Rating](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=security_rating)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Technical Debt](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=sqale_index)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)

[![Latest preview](https://img.shields.io/github/v/release/stephenlclarke/meridian?include_prereleases&label=preview)](https://github.com/stephenlclarke/meridian/releases)
[![Downloads](https://img.shields.io/github/downloads/stephenlclarke/meridian/total)](https://github.com/stephenlclarke/meridian/releases)
[![Open issues](https://img.shields.io/github/issues/stephenlclarke/meridian)](https://github.com/stephenlclarke/meridian/issues)
[![Open pull requests](https://img.shields.io/github/issues-pr/stephenlclarke/meridian)](https://github.com/stephenlclarke/meridian/pulls)
[![Last commit](https://img.shields.io/github/last-commit/stephenlclarke/meridian/main)](https://github.com/stephenlclarke/meridian/commits/main)

[![Rust 1.99](https://img.shields.io/badge/Rust-1.99.0-000000?logo=rust)](rust-toolchain.toml)
[![Preview platforms](https://img.shields.io/badge/preview%20platforms-macOS%20%7C%20Windows-blue)](docs/PREVIEW-RELEASES.md)
[![macOS signing](https://img.shields.io/badge/macOS%20signing-Developer%20ID-green)](docs/PREVIEW-RELEASES.md)
[![Preview 12 notarisation](https://img.shields.io/badge/Preview%2012%20Mac-notarized%20%2B%20stapled-green)](docs/PREVIEW-RELEASES.md)
[![Source-visible license](https://img.shields.io/badge/license-proprietary%20%2F%20source--visible-blue)](LICENSE)

Meridian is a Rust reimplementation of the original L337 project, with direct wgpu graphics, one authoritative World and traceable original content. New code is [proprietary/source-visible](LICENSE); imported original content retains its own notices. Original Elite/Oolite assets are excluded.

The earlier [Preview 8 EMP utility](docs/EMP.md) is published for macOS arm64 and Windows x64. It adds paid purchase, fitting, storage and power/capacity/recharge upgrades, a held-U pulse and electrical disablement that preserves hull, crew and cargo custody. Mac, Linux and native Windows qualification passed 631 Rust tests, 99 Python tests and 49 Markdown checks per platform, strict Clippy and locked release builds. The release uses corrected source `2d5f4a5`; the full earlier EMP native journey retains its separate `5e3e98a` identity. See [implementation status](docs/IMPLEMENTATION-STATUS.md) for the exact evidence and open limits.

## Previews and validation

Download qualified macOS arm64 and Windows x64 prereleases from [GitHub Releases](https://github.com/stephenlclarke/meridian/releases). Use the launcher and Read Me included with your download. Keep its app, notices and versioned save folder together; preserve earlier previews and profiles separately. Releases use Conventional Commit versions and immutable annotated tags, and retain the newest three published previews after verified downloads and a durable receipt.

The Preview 8 Mac app is Developer ID signed, notarized and stapled. Apple accepted the new EMP submission; the actual public download passed strict signature, ticket and Gatekeeper checks. No older ticket was reused. Windows remains unsigned and requires the external Microsoft Visual C++ x64 runtime; no DLL is bundled. Native Windows interactive gameplay remains unobserved. Notarization does not establish commercial content clearance or broader gameplay acceptance.

## Preview 8 Rust checkpoint

[Preview 8 v0.8.0-preview.1](https://github.com/stephenlclarke/meridian/releases/tag/v0.8.0-preview.1) was published on 18 November 2026 at 07:26 UTC from protected main `bf31ab7`, compiled source `2d5f4a5`. All 300 committed files and modes are identical between those revisions. It retains contextual H/? Help and explicit Ctrl+Shift+X XYZZY testing, and adds the paid EMP utility. All six exact-main CI checks passed. Sonar uses that exact commit as revision and project version: zero unresolved findings, 98.8% new coverage, zero new duplication and 100% hotspot review.

All four actual anonymous downloads matched their sealed hashes. The complete 474-file Mac wrapper and 331-file Windows package matched their payload bytes and modes. All 1,465 Mac ZIP entries, including 732 AppleDouble metadata files, were checked separately from the extracted payload. The app, game and launcher passed strict Developer ID verification; the new EMP ticket passed stapler and Gatekeeper validation. The immutable annotated tag points to the protected main. The actual release receipt was saved and flushed before removing only v0.5 and its tag. Retained previews are v0.8, v0.7 and v0.6.

The new `2d5f4a5` Mac binary passed bounded title/Help, equipment/pylon and ordinary-flight checks, two normal exits with code 0, one cold Continue and 430 unheld accepted ticks. Position advanced while speed, orientation, ownership and cash/cargo stayed consistent; Space input does not prove a stopped ship. The full paid EMP purchase/grade/cancel/pulse/disable/recovery journey remains earlier `5e3e98a` evidence through the independently reviewed three-helper semantic inverse; it was not repeated on the new binary. Its labelled headless/XYZZY setup was not native-earned progression.

Moving EMP-disabled coast, Help interruption during held U, the exact native 1,800/1,801-tick recovery boundary, Windows GUI gameplay, full original XYZZY parity and native Lareria travel/refill remain open. The original 416-object/19,936-byte leak is unresolved; a scoped negative 472-second run does not establish a cure, an OS-only cause or current memory acceptance. Mission/fleet completion, measured performance, original-input obligations, upstream Apple SDK policy and commercial content acceptance remain separate. Current saves use V17 with the EMP content fingerprint; unsupported older content refuses without migration. Use the isolated profile supplied with each download.

### Earlier Preview 7 Help and XYZZY evidence

[Preview 7 v0.7.0-preview.1](https://github.com/stephenlclarke/meridian/releases/tag/v0.7.0-preview.1) retains main `a05c69e` and compiled `076639d`. Its 603 Rust, 99 Python and 48 Markdown checks per platform, 91.8% exact-main new coverage, bounded new-binary Help/activation/cold-Continue checks and broader earlier `c98fc86` journey retain those original identities. Preview 8 does not turn them into new native runs. Preview 7 was Developer ID signed without an EMP notarization claim.

### Earlier Preview 6 mining evidence

[Preview 6 v0.6.0-preview.1](https://github.com/stephenlclarke/meridian/releases/tag/v0.6.0-preview.1) is published for macOS arm64 and Windows x64 from main `07bb22c`, compiled `72dcc28`. All three owned platforms passed 539 Rust and 94 Python tests; exact-main CI and Sonar passed with zero open issues. All four public downloads, archive contents, Mac signature and annotated tag were verified before retention. The updated Mac build passed title/render/Escape-close smoke without a commander. Its earlier bounded mining journey retains the separate `a1ee135` source identity. Windows interactive gameplay and normal exit remain unobserved. This release includes neither the help/XYZZY candidate nor Lareria.

Preview 6 candidate `a1ee135` adds paid Beam and Mining Lasers, Extra Energy System and Fuel Scoops, shared resource accounting, physical asteroid work and persistent pending ore. Mac qualification passed 534 workspace Rust tests; Dell Linux and native Windows passed those tests plus three explicit observer cases, or 537 each. All passed 94 Python tests, 46 Markdown checks, formatting, all-target checks, strict Clippy and locked release builds. Hosted PR checks passed. Nested subprocess summaries are not extra cases.

The installed Developer ID signed Mac preview was operated through the supported computer-use adapter. An explicitly funded headless setup was continued in its isolated profile; native purchases fitted the Mining Laser for 3300 CR and Fuel Scoops for 1900 CR, leaving 3314.5 CR. Two real front-contact strokes reduced one asteroid from 100 to 50 work and then exhausted it, producing exactly one pending Alloys unit. Both partial work and terminal pending identity survived Save/Quit and Continue in new processes. Cash and cargo stayed unchanged. This validates the bounded mining and persistence path; funding was setup, not player-earned native acceptance.

Physical ore pickup and sale remain unavailable without eligible original body geometry. Fitted scoops do not grant cargo or income. That Preview 6 scope excludes XYZZY and the subsequent navigation/custody increments. Its completed publication retains its separate source and receipt.

Earlier qualified previews retain their own native flight, physical docking, trading, finite missiles, freight deliveries, once-only 730 CR reward and trusted-clue evidence. Those journeys are not relabelled as execution of this binary. Native Windows interactive gameplay, remaining populations and equipment, memory qualification, window/focus edges and measured frame timing/race remain open. Full original **1337 visual and gameplay parity** and the expanded mission/fleet acceptance remain open. That earlier Preview 6 binary writes private save V16; Preview 7 uses V17 and refuses earlier formats.

See [implementation status](docs/IMPLEMENTATION-STATUS.md), [mining scope](docs/ASTEROID-MINING.md) and [source-grounded roadmap](docs/LEGACY-MILESTONES.md).

## Navigation and custody evidence before Preview 7

Compiled candidate `991fd91` passed owned Mac, Linux and native Windows qualification: 551 Rust tests each, 94 Python tests, 47 Markdown checks, strict Clippy and locked release builds. It reads and writes only V17 and refuses earlier formats without changing the file. In the signed isolated Mac app, supported native input purchased an Intermediate Navigation module for 3000 CR, stored and refitted the same paid item, and retained it across a new-process Continue. Module reach was 14 LY; stored reach fell to the SH025 floor of 9 LY. The chart correctly showed the actual 9.6 LY fuel limit. No serviced flight benefiting from a leg beyond 9 LY is accepted: only Avenor and Renas services exist, and the required cumulative fuel exceeds tank capacity.

The next source increment adds the original Lareria G01-S091 main orbital. The paid Intermediate grade enables the 9.93 LY Avenor–Lareria leg, costing 99 fuel units. Actual berthing provides paid refill for a return journey. Exact Lareria market data is imported, and the desktop admits this one additional destination. That exact Lareria-only source passed 555 top-level Rust and 94 Python tests on Mac, Linux and native Windows, with strict Clippy and locked release. Native Lareria acceptance remains pending. These results do not qualify the following changed help/XYZZY layout.

A separate explicitly prepared public-core setup supplied four nonfatal contacts and a live trader. One supported native final shot from a paid missile produced the exact terminal SH025 Food manifest, preserving its native endpoint and source pose across Save/Quit and new-process Continue. Cargo and cash stayed unchanged; the retained hull stopped the player on physical contact. Setup funding, piloting and earlier damage are not native journey credit. Physical pods, pickup, sale and salvage income remain unavailable. Independent audit cleared these bounded observations. Their source is included in Preview 7 through later joins; these native observations retain the earlier compiled identity. Preview 6 keeps its separate source and release evidence.

## Play

Open the [Unwritten Route clue journal](integration/unwritten-route/README.md) with **Shift+J** to see the source clues your commander has earned.

The current desktop source opens **Commander details** on startup. An empty slot starts a fresh commander at Avenor in memory. A valid slot shows the existing commander at the saved location before simulation resumes. Press **L** to launch from a station or **Enter** to resume a saved flight. Station services use **1–5** at a valid berth, and **0** returns to Commander details when docked. Use **H** or **?** for the current controls.

Preview 8 uses the earlier startup controls. N starts a commander; C continues a current-format V17 save with the supported EMP content fingerprint. Unsupported formats/content refuse without changing the file. Use the download’s isolated profile: `Meridian EMP Sonar Preview/saves-current-emp-2d5f4a5` under macOS Application Support, or adjacent `saves-emp-2d5f4a5` on Windows. A second N confirms replacing the current commander, and the old disk file remains until an explicit save or normal saved quit. Click the window for keyboard focus.

H or ? opens a dismissible help popup for the current screen, including station details and transaction reviews. Every station popup lists 1 Galaxy, 2 System, 3 Showroom, 4 Market, 5 Equipment, Tab/Shift+Tab and Escape/back. Escape, H or ? closes help and retains the screen, selection, search and review. Gameplay and underlying actions wait while help is open. During galaxy text search, H remains a typed letter and ? opens help.

### Controls

The table describes the current source. Use the number row for views, training and missiles. Press the equals or minus key without Shift for missile launch or safe/unlock. Hold Control for save, reload and fullscreen.

| Controls | Action |
| --- | --- |
| H / ? | Context help; Escape, H or ? dismisses |
| Ctrl+Shift+M | Toggle generated sound (session-local); status in Help |
| Ctrl+Shift+X when docked | Explicit XYZZY testing menu; Enter reviews then confirms |
| 0 when docked (current source) | Commander details |
| 1 / 2 / 3 / 4 / 5 when docked | Galaxy / System / Ships / Market / Equipment |
| P on the station Galaxy chart (development) | Toggle the best-profit scan with commodity quantities and live quote estimates; P in flight still pauses |
| R, then D in a station route review (development) | Inspect estimated trade per leg; D, B or Enter closes details without installing a route |
| L; W/S; arrows; A/D; Q/E; Space | Launch; thrust; pitch; yaw; roll; brake |
| N in flight (development) | Engage the fitted navigation computer for docking, or cancel the approach |
| Tab / F / Delete / Shift+I in flight (development) | Cycle scanner contacts by distance / switch between assisted and manual targeting / release target / show target details |
| J; T (docked); R; Enter; B; F (docked) | Travel; select contact; review; confirm; cancel; buy fuel |
| M; [ / ]; Left/Right; Shift+Left/Right; G / Shift+G | Market; commodity selection; adjust quantity by 1 / 10; review buy/sell |
| O; K; U; Y | Equipment panel; buy/fit cargo bay; store when docked; refit |
| Hold U in active flight | Charge the fitted EMP for 600 accepted ticks; early release cancels |
| I; V; X | Mission details; review all held freight deliveries; review abandonment |
| T in flight; 8; 9; =; - | Lock the selected ship for missiles; next ship/practice lock; arm/safe; launch one missile; safe/unlock |
| Shift+5; Shift+4; Shift+3 | Review attempt closure; reception; fresh consent |
| Shift+1; Shift+2; C; 0/1/2/3 | Flight lesson consent/reception; cancellation; details; view direction |
| P; Ctrl+Enter; Ctrl+S; Ctrl+R | Pause; fullscreen; save; reload the current qualified save |
| Escape / window close | Save before exit; failure keeps the app open |

The [station trade estimate details](integration/followup-trade-detail/README.md) show the current finite commodity allocation behind the chart scan and route estimates. They do not purchase, sell or install a route.

The [atomic market quantity controls](integration/followup-bulk-market/README.md) review and settle all requested native units together or refuse the entire purchase or sale. They retain the existing per-unit receipts and save format.

The freight journey requires eight identified one-tonne deliveries and awards 730 CR plus the trusted clue once. Markets use finite stock and the same account as mission and equipment transactions. Invalid saves refuse without migration, repair or fallback.

## Build and contribute

The workspace pins Rust and dependencies in [rust-toolchain.toml](rust-toolchain.toml) and [Cargo.lock](Cargo.lock). Use `make ci` for documentation, formatting, importers, compilation, tests and strict Clippy. `make run` starts the desktop app in an owned graphics session. G1 owns Mac compilation/native validation; Dell owns Linux/native Windows validation; MBA retains its existing fleet work.

- [Development and repository maintenance](docs/DEVELOPMENT.md)
- [Saker source envelope and Hull Studio definition](docs/SAKER-GAMEPLAY-ENVELOPE.md)
- [Eagle Cruiser source envelope and Hull Studio definition](docs/EAGLE-CRUISER-SOURCE.md)
- [Harpy Assault source envelope and Hull Studio definition](docs/HARPY-ASSAULT-SOURCE.md)
- [Erne Corvette source envelope and Hull Studio definition](docs/ERNE-CORVETTE-SOURCE.md)
- [Harpy Dropship source envelope and Hull Studio definition](docs/HARPY-DROPSHIP-SOURCE.md)
- [Harpy Gunship source envelope and Hull Studio definition](docs/HARPY-GUNSHIP-SOURCE.md)
- [Lanner source envelope and Hull Studio definition](docs/LANNER-SOURCE.md)
- [Hyena source envelope and Hull Studio definition](docs/HYENA-SOURCE.md)
- [Yak source envelope and Hull Studio definition](docs/YAK-SOURCE.md)
- [Jackal source envelope and Hull Studio definition](docs/JACKAL-SOURCE.md)
- [Banteng source envelope and Hull Studio definition](docs/BANTENG-SOURCE.md)
- [Caracal Scout source envelope and Hull Studio definition](docs/CARACAL-SCOUT-SOURCE.md)
- [Hobby Fighter source envelope and Hull Studio definition](docs/HOBBY-FIGHTER-SOURCE.md)
- [Condor source envelope and Hull Studio definition](docs/CONDOR-SOURCE.md)
- [Aplomado Interceptor source envelope and Hull Studio definition](docs/APLOMADO-INTERCEPTOR-SOURCE.md)
- [Falconet source envelope and Hull Studio definition](docs/FALCONET-SOURCE.md)
- [Muskox source envelope and Hull Studio definition](docs/MUSKOX-SOURCE.md)
- [Barbary source envelope and Hull Studio definition](docs/BARBARY-SOURCE.md)
- [Wombat source envelope and Hull Studio definition](docs/WOMBAT-SOURCE.md)
- [Saiga source envelope and Hull Studio definition](docs/SAIGA-SOURCE.md)
- [Moose Clipper source envelope and Hull Studio definition](docs/MOOSE-CLIPPER-SOURCE.md)
- [Seriema Courier source envelope and Hull Studio definition](docs/SERIEMA-COURIER-SOURCE.md)
- [Arna Cutter source envelope and Hull Studio definition](docs/ARNA-CUTTER-SOURCE.md)
- [Vulture Raider source envelope and Hull Studio definition](docs/VULTURE-RAIDER-SOURCE.md)
- [Rabbit Explorer source envelope and Hull Studio definition](docs/RABBIT-EXPLORER-SOURCE.md)
- [Rhinoceros Trader source envelope and Hull Studio definition](docs/RHINOCEROS-TRADER-SOURCE.md)
- [Badger source envelope and Hull Studio definition](docs/BADGER-SOURCE.md)
- [Baza Fighter source envelope and Hull Studio definition](docs/BAZA-FIGHTER-SOURCE.md)
- [Baza Mk II source envelope and Hull Studio definition](docs/BAZA-MK-II-SOURCE.md)
- [Griffon source envelope and Hull Studio definition](docs/GRIFFON-SOURCE.md)
- [Griffon Mk II source envelope and Hull Studio definition](docs/GRIFFON-MK-II-SOURCE.md)
- [Griffon Spectre source envelope and Hull Studio definition](docs/GRIFFON-SPECTRE-SOURCE.md)
- [Dzo source envelope and Hull Studio definition](docs/DZO-SOURCE.md)
- [Dzo Mk II source envelope and Hull Studio definition](docs/DZO-MK-II-SOURCE.md)
- [Hog source envelope and Hull Studio definition](docs/HOG-SOURCE.md)
- [Tamaraw source envelope and Hull Studio definition](docs/TAMARAW-SOURCE.md)
- [Oryx Voyager source envelope and Hull Studio definition](docs/ORYX-VOYAGER-SOURCE.md)
- [Carabao Freighter source envelope and Hull Studio definition](docs/CARABAO-FREIGHTER-SOURCE.md)
- [Carabao Highliner source envelope and Hull Studio definition](docs/CARABAO-HIGHLINER-SOURCE.md)
- [Jaguar source envelope and Hull Studio definition](docs/JAGUAR-SOURCE.md)
- [Hare Explorer source envelope and Hull Studio definition](docs/HARE-EXPLORER-SOURCE.md)
- [Llama source envelope and Hull Studio definition](docs/LLAMA-SOURCE.md)
- [Laggar source envelope and Hull Studio definition](docs/LAGGAR-SOURCE.md)
- [Alpaca source envelope and Hull Studio definition](docs/ALPACA-SOURCE.md)
- [Elk source envelope and Hull Studio definition](docs/ELK-SOURCE.md)
- [Springbok Starliner source envelope and Hull Studio definition](docs/SPRINGBOK-STARLINER-SOURCE.md)
- [Antelope source envelope and Hull Studio definition](docs/ANTELOPE-SOURCE.md)
- [Gopher source envelope and Hull Studio definition](docs/GOPHER-SOURCE.md)
- [Elephant source envelope and Hull Studio definition](docs/ELEPHANT-SOURCE.md)
- [Aurochs source envelope and Hull Studio definition](docs/AUROCHS-SOURCE.md)
- [Giraffe Liner source envelope and Hull Studio definition](docs/GIRAFFE-LINER-SOURCE.md)
- [Shikra source envelope and Hull Studio definition](docs/SHIKRA-SOURCE.md)
- [Shikra Interdictor source envelope and Hull Studio definition](docs/SHIKRA-INTERDICTOR-SOURCE.md)
- [Wisent source envelope and Hull Studio definition](docs/WISENT-SOURCE.md)
- [Wisent Mk II source envelope and Hull Studio definition](docs/WISENT-MK-II-SOURCE.md)
- [Anoa source envelope and Hull Studio definition](docs/ANOA-SOURCE.md)
- [Ox source envelope and Hull Studio definition](docs/OX-SOURCE.md)
- [Ox Mk II source envelope and Hull Studio definition](docs/OX-MK-II-SOURCE.md)
- [Cavy Racer source envelope and Hull Studio definition](docs/CAVY-RACER-SOURCE.md)
- [Stag Liner source envelope and Hull Studio definition](docs/STAG-LINER-GAMEPLAY-ENVELOPE.md)
- [Architecture](docs/ARCHITECTURE.md) and [core API](crates/meridian-core/README.md)
- [First playable criteria](docs/FIRST-PLAYABLE.md) and [source-grounded roadmap](docs/LEGACY-MILESTONES.md)
- [Content provenance and reuse](docs/LEGACY-REUSE.md)
- [Finite missile behavior and save boundary](docs/FINITE-STARTER-MISSILES.md)

GitHub Actions validates Linux, Windows and macOS. Protected main requires pull requests, current required checks, linear history and resolved conversations; force pushes and deletion are blocked, including for administrators. SonarQube runs after merge from committed main with the exact commit SHA and Previous version policy. Historical change descriptions live in merged GitHub PRs and Git history rather than duplicate issue/PR Markdown files.

## Next two milestones

These are the source-grounded M2 and M3 boundaries; completed entries below describe bounded reusable Rust slices, not completed milestones or all 64 mission acceptances.

| State | M2: flight, equipment and ordinary economy | M3: mission families and trusted clues |
| --- | --- | --- |
| Completed bounded slices | Seventeen finite commodity markets; basic paid equipment/storage/refitting; Pulse and finite missile/damage/save mechanics; bounded SH025 trader/pirate journey; paid mining/Fuel Scoops with persistent pending ore; contextual Help/XYZZY; paid defensive EMP and grades with qualified tests and separately attributed native evidence. | Freight custody, once-only receiving/payment/clue and cold-Continue foundations; bounded gunnery and navigation/trade lesson mechanics. |
| In progress | Positive ore collection still needs eligible original body geometry; broader equipment, utility, population and ship-exchange acceptance remains open. | Core prerequisites come first. Existing family mechanisms and source walkthroughs do not close the expanded mission acceptance ledger. |
| Not yet implemented or accepted | Broader EMP civilian/off-system behavior, positive salvage and bounty/legal consequences; remaining equipment and population gameplay acceptance. | Rust crew/passenger/rescue, investigation and Wayfarer family journeys, then their individual consent/failure/reward/cold-Continue acceptance. All 64 expanded runtime acceptances remain open. |

Fleet construction is a separate M4 lane. Its current changed original-five run returned no candidate meshes; no new hull admission follows from focused tests. Original mesh/material/colour fidelity and 1337 visual/mechanics parity remain requirements across these milestones. See the [source-grounded roadmap](docs/LEGACY-MILESTONES.md).

## Context help and XYZZY testing

Preview 8 retains screen-specific H/? help with explicit [XYZZY testing](docs/XYZZY.md). Every station help popup explains how to reach Galaxy, System, Showroom, Market and Equipment. Help pauses host stepping and preserves the current screen, selection, search and review. Press H or ? for flight controls or station navigation. The current desktop source clears routine text from above the instruments in idle flight and at the docked exterior. Command feedback and requested reviews still appear there. The development build also gives the Lion Citadel blue-grey plating and amber lights, with four small joins around its docking entrance. [Station exterior](docs/DOCKED-PREVIEW.md#sh028-aperture-crosspieces)

At a station berth, **Ctrl+Shift+X** opens the labelled XYZZY testing menu. Use Up/Down to choose, Enter to review, then Enter again to confirm and save. The menu stays open on the selected action. Use the arrow keys to choose another action or Escape/B to leave. H/? opens help above the menu. First activation changes the current commander to CLARKE and sets cash to 1,000,000 CR once. It grants/refits the supported SH025 outfit with explicit secret provenance; missiles and cargo stay finite. The menu shows cash, current jump reach, fuel mode, the fitted targeting grade and the three EMP grades. Reading or closing it grants nothing. Other profiles are unchanged.

Source `c98fc86` passed Mac qualification: 603 top-level Rust tests (378 core, two compatibility integrations, 220 desktop and three explicit observer cases), 94 Python tests, 48 Markdown checks, formatting, all-target checks, strict Clippy and locked release. The signed isolated app passed supported native checks of all five station Help cards, navigation, blocked controls, hidden search with nested testing Help, explicit activation, visible grant provenance and a real paid Food purchase. Repeating activation preserved 999,977.4 CR and the purchased tonne. Accounted fuel was switched off, flight Help displayed the fitted Naval Laser, manual pause survived Help, and actual Save/Quit/new-process Continue retained CLARKE and the same cash, cargo and fuel mode. Precise native clock, OS exit status and a Lareria docking/refill journey were not measured. Independent native acceptance and artifact audit passed. Dell Linux and native Windows also passed the same 603 Rust and 94 Python tests, strict Clippy and locked release; native Windows importer checks used native Python. The [36-action inventory](docs/XYZZY-ACTIONS.json) retains every unsupported or partial original action. Preview 7 now publishes these features using the corrected binary described below.

PR 47 merged the composition at `de7d6bb`; main CI and the Sonar quality gate passed, but nine unresolved maintainability findings held publication. Correction `076639d` extracts four helpers and replaces five wildcard test imports with independently reviewed behavior and unchanged assertions. Its new Mac, Linux and native Windows binaries passed 603 Rust tests, 99 Python tests, strict Clippy and release builds. The separately signed Quality Preview passed bounded Help, explicit XYZZY activation and cold Continue checks. PR 48 merged at `a05c69e` after all final-head checks passed. All six main CI jobs and exact-main Sonar passed with zero unresolved issues; v0.7.0-preview.1 was published and its public downloads verified. The broader journey above remains attributed to `c98fc86`.

Published Help lists mission details and flight-lesson review/cancel/details in both the station HUD and flight popups. Its retained minimum-window regression checks the concise HUD hint, all those popup controls, complete card text, bounded rendering and unchanged saved state through docked, launch and all four flight views. The regression passed on the qualified Mac source; minimum-size native resizing remains unobserved.

Preview 8 includes the paid Equipment EMP utility slot, held-U flight control and contextual Help/readouts. Its bank starts empty; docked U remains Store. The current defensive slice requires a real recent attacker and valid physical geometry. It does not grant salvage, cargo, bounty or crew rewards. See [EMP scope and acceptance](docs/EMP.md) for prices, grades, electrical rules and the remaining native boundaries.

## Offline reference importer

The [Rust reference importer](crates/meridian-import/README.md) converts only the pinned original SH025/SH028 source into the five existing binary fixtures. It preserves original notices and emits its own Rust producer receipt. The current source has passed designated Mac, Linux and native Windows qualification, including actual CLI conversion and the current core loader. PR 52 merged at `22715d6`; all six exact-main CI jobs and the Sonar quality gate passed. One reported method-reference maintainability finding is being corrected; the correction has not yet been qualified. The importer does not add ships or change gameplay, and is not included in Preview 8.
