# Qualified v15 native fixture

`qualified-v15-npc-flight.bin` is the unchanged 13,952-byte `pulse-safe-settled` checkpoint from the 17 November 2026 signed Mac NPC qualification. A fresh ordinary commander was created in an initially empty isolated qualification slot using the supported native UI. The bytes were published by the game during the native run, copied into test evidence, and read successfully by the original read-only observer. This fixture is from those retained evidence copies; no live profile was opened or copied. It contains a randomly generated game identity and in-game state from that isolated test session.

- Producer source: `bd506f9f9a8390bceecc4fe76dce5f60ffc6e410`, tree `452c9c4e461c15e8b94f47f3e13f01913bcafb74`.
- Native App executable SHA-256: `6eba5d34f7657a4f5d080a4f558c2433c7eb49b0c1940dad3661d33a70c75f25`.
- Fixture and original native UI evidence copy SHA-256: `f226c072e95546686cf3bccd99578921bb3a38e8628063fac36c6d31ecd2d989`.
- Original observer SHA-256: `66a5c0da55347e3b06512e1ae89ee4fc96b31793b1f48a4a5f9583dc94a999d5`.
- Original observer output SHA-256: `06157d459db728f157f07182e841e5530de8fff26e18ac8cb9f53615b087c1bc`.

The checkpoint is airborne and paused at Renas, tick 9012. It retains two paid missiles that hit the native pirate at ticks 8830 and 8879, incoming front-facing NPC fire at tick 9012, three unspent born rounds, one Food tonne and 337.4 CR. The born front Pulse is fitted and ready. This fixture does not establish migration of every v15 state or exercise a retained player-to-NPC beam receipt.

`qualified_v15_npc_continue` now asserts the literal historical version and an explicit unsupported-format refusal from the V17 reader. It checks that live restore preserves current state and that current-format cold Continue succeeds. The original acceptance test and qualification history remain in the preceding commits. The fixture bytes stay unchanged.

## PR #44 format-16 M02 checkpoint

`pr44-v16-m02-purchases.bin` is the unchanged 3,102-byte
`after-purchases.meridian` diagnostic copy from the isolated M02 profile.
The producer was PR #44 head `a1ee1358c4f8bdb91dad2e6c4d0e299b00c63ab3`,
tree `6685f513d6b25ac5ee0bad021a69673cad9e76c5`. Its SHA-256 is
`d2f995aa2826d8e77fc215cbaf7dee950b68c530b20d09a0dfd226d50980ddae`.
The desktop input in the originating run remains diagnostic. This regression
uses the emitted bytes to check explicit V16 refusal and live-state preservation. It separately checks current V17 cold Continue.

`xyzzy-paid-base-v17.bin` is a synthetic commander saved on Ubuntu by Meridian revision `fcec61d182b66478846b92390ad17805b0dfc7e0`. The EMP tests' `paid()` journey earned and purchased the base, then `secret()` activated the original outfit at Renas. The original encoder wrote the fixture before XYZZY granted EMP grades. It preserves an ordinary paid base at grade zero, 1,000,000 CR, and the earlier outfit history for cold-load and paid-upgrade regression tests. SHA-256 `87caec50eba9e8cbeb1b205c545a71429972e896b1844cd53bd71b7cb39596df`.

## Occupied cutter warning fixture

`occupied-cut-held-v17.bin` is the exact 39,168-byte scratch profile emitted on 26 November 2026 by G1 from the existing `disabled_native_target_with_fit` / `occupied_cut_requires_explicit_review_and_retains_actual_finite_game_contact` core test at its first genuine occupied contact review. The source producer starts with the retained paid-base and explicitly XYZZY-supported fixture, uses public flight, paid grades, actual laser and EMP contacts and native selected hold; it does not inject a ship or a writeable contact. This is headless actual simulation evidence, not a native UI or normally earned whole-profile acceptance.

- Source producer's committed base: `6a21c890f5ec2e33c44f037cacdac825570592dd`; the occupied selection correction is `93d752c65f2989b9bea8626a4ae1aba66c060c95`. Desktop confirmation tests passed on Mac, Linux and Windows on combined commit `d0f7daba`.
- SHA-256: `e9e156f9761d7c39e7c2c9767574ae73d7d4b6204527c5bb59647c127f62b9b9`.
- Save format: V17; World tick 34,515; target profile 16,101, galaxy 0, system 185 (Renas), serial 2; fitted cutter item 116,114; no occupied contact receipt.

The desktop regression copies these unchanged bytes to disposable scratch slots, opens normal Profile Continue, and explicitly selects the actual disabled occupied actor through the public targeting API. It then checks the modal's frozen tick, ordinary key suppression, Escape, stale selection and focus cancellation, one atomic confirmed contact, failed-writer rollback and cold Continue without repeat. It does not infer material, cargo, hull or crew acquisition. No live player profile was used.
