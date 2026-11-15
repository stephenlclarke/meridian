# World G02 freight journey

Status: World integration and three public-command journey/refusal tests are authored. Formatting, compilation, Clippy and test execution are UNRUN on this source. No desktop interaction, native graphics or save/reload acceptance is claimed. This successor starts at navigation commit `9e22d877dbcbce559a41a3baed165ff7da536dec`, tree `6457d430e6963b3fd31f9bfcabea9401eea51a3e`, which includes the frozen owned-starter packet and station main.

## Actual ownership and authority

`World::new_commander(seed)` creates the same original Avenor SH025 profile, one retained `CommanderAccount` and one `FreightSession`. That account owns credits, fuel and identified starter equipment. The freight session owns supplier allocations, job custody, confirmed review and clue receipts. There is no second wallet, mission actor, public arrival constructor or new identity allocated by a getter.

The navigation producer supplies `DockContext` only for the legitimate authored new-profile dock or actual continuous station entrance admission. The two origins remain distinct. World creates a station roster at those publications: Arlen at Avenor, the utility receiving crew at Renas, and no G02 NPC at other systems. Roster entries carry the owning commander/ship, original system, actual station actor, admission sequence, visit, NPC entity generation and roster revision. NPC entity slots 2 and 3 are new Meridian scene identities; they are not a claim about original Swift NPC entity numbers. Station actor identity 1 comes from the navigation context.

`station_contacts()` returns descriptive copies of the existing roster. `request_contact(npc, tick)` must match the complete current roster entry and current World-owned dock context before selecting a contact. A copied contact from another owner, an earlier dock or an airborne position cannot select a contact. A requested bare actor ID is never consent. `request_freight(command, tick)` builds private facts from that actual selection, current admission, owning account, paused state and session epoch, then synchronously prepares and commits against that same World state.

Launching removes dock and roster/contact authority. Physical entry and hyperspace arrival publish a new context generation; hyperspace arrival remains Flight at the original 16km approach distance with no dock/contact authority. Pause/resume and explicit recontact change the epoch, so earlier confirmed reviews cannot be reused. Invalid commands preserve flight/navigation, source/custody, account, clue and current roster/contact state.

The content identity is the fixed original G02 rules compiled into this Rust port, pinned by [G02-CONNECTED.md](G02-CONNECTED.md). There is no public pack loader, caller-set content digest, Lua execution or runtime claim that an arbitrary external pack was admitted. Dynamic pack loading and reinstalls are outside this executable slice. This compiled-source scope must remain explicit in UI and reviews.

## Public integration API

| API | Purpose |
| --- | --- |
| `World::station_contacts()` | Current actual station roster, empty in flight or prototype worlds |
| `World::request_contact(StationNpc, expected_tick)` | Explicitly select an existing current NPC |
| `World::request_freight(g02::Command, expected_tick)` | Review, confirm, receive one/all, abandon or Back through actual World facts |
| `World::mission_snapshot()` | Freight custody/source/clue, current selected contact and pause state |
| `World::request_pause(paused, expected_tick)` | Normal pause lifecycle; paused World does not step, launch, travel or execute freight/contact commands |
| `World::request_refuel(expected_tick)` | Pay the original ordinary main-orbital fuel price from the same account |

`StationNpc` has read-only `actor`, `definition`, `name` and `dock` methods. It has no public constructor. `MissionError` distinguishes missing profile, stale command, missing dock, wrong contact, paused state, sequence exhaustion and the original freight refusal. The existing public flight/navigation commands retain their purpose. Desktop controls remain a separate Root-owned join.

Normal refuelling uses the actual current admission and the same fuel/credit account. The original source is immutable a11 `Plugh.swift:72-80`, `Game.swift` Fuel equipment base price 2, and `EquipmentActorTransactions.swift:220-231`: refill the actual missing fuel at two credit tenths per fuel tenth. The current navigation slice supports ordinary main-orbital systems, not a secret-port discount. The service checks charge/revision before mutation and refuses a full tank. It does not grant equipment, paid prerequisite history or free travel fuel. Updating the account invalidates earlier freight reviews.

## Authored checks and required execution

The three new tests operate only through public World APIs and immutable presentation values. The positive journey creates a fresh profile, contacts Arlen, reviews/confirms the eight-unit allocation, launches, runs the real 600-tick countdown, checks fuel stays 130 until actual arrival then becomes 96, checks the 16km Flight arrival has no contact authority, flies normal controls through the original continuous entry checks, contacts the actual Renas crew and transfers the original units one at a time. The seventh pays nothing; the eighth commits 730 CR, completion and first clue. Foreign/out-of-order/duplicate units and replay must preserve public observable state.

The other checks cover copied NPCs from another owner, prototype worlds, Back, changed contact, pause/resume, stale ticks and normal paid refuelling invalidating a receiving review. The 11 existing domain tests and seven navigation tests remain present. Invalid-equipment and overflow checks retain their explicitly synthetic internal domain scope; there is no new public gear-grant or cargo-injection API merely to produce a test fixture.

Required before acceptance: final whole-artifact source review; Dell formatting, workspace/all-target checks, all existing plus new tests, strict Clippy without warning suppression; Root desktop integration and native interaction evidence. Resolve actual findings in the complete joined artifact. Source inspection and authored test names are not test results.

## Limits

The transaction publishes memory only. It does not serialize a prospective game, atomically replace a save, restore validated mission/account state or test disk failure. Do not claim continuation/reload completion. General equipment purchases/refits, item firing/unfitting, other hulls, cargo trading and loss recovery remain unsupported; existing standard equipment is actual retained property. The original station geometry's documented collision and physical fidelity limits remain unchanged.
