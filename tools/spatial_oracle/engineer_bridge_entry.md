# Ordinary Engineer approach and bridge-hut entry

This chain covers an ordinary ground ENGINEER ordered to enter the physical
CABHUT at `(68,74)` in the selected retail Hills scenario after its connected
high bridge collapses. The legal approach starts at `(66,76)`. It follows the
normal CaptureBuilding command, Foot path request, AStar neighbors, Walk head
admission, Infantry PerCell entry and bridge repair publication. The whole-bridge
goal remains open; this is one common path and its required shared dependencies.

The earlier diagnostic source `(69,74)` was invalid. Its physical tile is
`Cliff21.tem`, tile69/subtile4, ground level10, Rock land3 and movement class6.
Original Foot4D3810/Map56D100 rejects the connection too. Original56C510 matched
all thirteen connectivity rows and base IDs of seven exported Hills snapshots.
That diagnostic is retained as a negative control, not grounds for weakening
the zone test. The frozen external receipt is
`bridge-target-evidence-20260927/engineer-entry-followup/HILLS_ENGINEER_NEGATIVE_CONTROL.md`.

## State and decision owners

The existing live Foot entry body in `world/bridge_repair_admission.rs` owns
the cell-entry decision. Its read context now accepts `&Simulation`; AStar and
Walk use that same receiver rather than separately rejecting the hut's static
building footprint. Canonical cell queries retain their shared Dummy effects.
Missing required input propagates as an error, distinct from ordinary refusal.
Foot restores Mark1 before returning a search error.

Foot navigation owns the queued direction words, NavCom and path timers. Walk
owns its destination, head, moving and motion bytes. The movement pass suspends
at the live query, then resumes the same Process invocation. Clear admission
reaches the existing subcell head selector; it is not reclassified by the
reduced path-grid/occupancy adapter. Nonzero replies follow the native caller's
retry, timer, gate, scatter and obstacle branches. The shared ground-object
contact callback uses the existing cloak transition and sound owners.

Infantry PerCell owns hut entry. Its prefix resolves effective current/queued
mission and the first ground building, distinguishes the 1x1 undeploy route,
and matches that building against NavCom or attack Target before checking the
Engineer and BridgeRepairHut types. `capture_target` is command/UI metadata,
not a second repair authority. The old adjacent-repair fallback is removed.
The existing bridge repair publication owner chooses the bridge family; the
wood tile interval is `[wood_base, wood_base+16)`, with an exclusive upper end.

## Native evidence

All new executable corpora pin original gamemd SHA256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
Their metadata records supplied inputs, execution boundaries and substitutions.
Scalar bodies and caller fragments do not prove full native route equality.

| Corpus | Executed boundary |
| --- | --- |
| [Infantry SpeedType](../rules_oracle/infantry_speed_type.md) | Full original5236A0 constructor, selected7121D1 layered field read, 26 parser controls and six-pass retained-default history |
| [AStar signed heights](astar_signed_height.py) | Eleven original initial-height/blocked-goal-tail controls, including signed raw bytes and later current-node heights |
| [AStar structural heights](astar_structural_height.py) | Twenty-two original node-height/list controls distinguishing raw structural0x100 from derived walkability |
| [AStar Infantry entry](astar_capture_neighbor.py) | Fifteen concrete51BF90→429830→429FEA controls on supplied interior search frames; fifth Infantry argument remains unread |
| [Walk Infantry entry](walk_capture_entry.py) | Original75B59C coordinate/height/query producer through concrete51BF90, stopping at head/refusal boundary |
| [Walk response](walk_prehead_response.md) | Supplied result classes0..7, timers, actual cloak and obstacle callbacks, and bounded recursive continuations |
| [Foot failure-sound byte](foot_scold_latch.md) | Original construction and raw-load retention, three Walk guards, thirty paid/idle tails and twelve Drive/Ship guard controls; sound requests stop before audio playback |
| [Engineer admission](engineer_repair_admission.py) | Twenty-two original PerCell prefix controls; runtime object-iteration-disabled is separately labelled |
| [Bridge family](engineer_family_selector.py) | Fourteen original selector controls, including exclusive wood bound, Y-major scan and shared Dummy behavior |
| [Structural side cells](bridge_side_admission.md) | Original constructor25 topology through six Walk/Infantry controls and two radar branches; deck admission does not require an own overlay |

The AStar height storage now preserves signed ground bytes widened before +4:
raw127 on a deck is131; raw128 is-128, or-124 on a deck. The blocked-goal tail
uses the expanded node's current height. Only the candidate's structural0x100
selects the bridge list; derived walkability does not. These scalar corrections
are compared directly to native output in `pathfinding/astar_entry_tests.rs`.

ENGINEER's absent SpeedType key keeps the Infantry constructor's Foot0,
through RULESMD, absent LANGRULE, MPBattleMD and Hills. Rust previously seeded
Track1. The reader uses an exact key, a128-byte trimmed string and the previous
field as default; a nonempty invalid name stores-1. The enum preserves that
invalid value. Native out-of-range terrain-table indexing for malformed mod
values is not emulated; the live entry owner reports an unavailable row.

Set `VERA20K_PROJECTILE_RENDER_ASSETS` to the physical extracted directory
containing RULESMD.INI, optional LANGRULE.INI, MPBattleMD.ini and Hills.map as
described in [the Hills evidence](bridge_target_layer.md). The native runner
also needs `VERA20K_GAMEMD_EXE` or `RA2_DIR`.

```sh
PYTHONPATH=. python -m tools.rules_oracle.infantry_speed_type --check
PYTHONPATH=. python -m tools.spatial_oracle.astar_signed_height --check
PYTHONPATH=. python -m tools.spatial_oracle.astar_structural_height --check
PYTHONPATH=. python -m tools.spatial_oracle.astar_capture_neighbor --check
PYTHONPATH=. python -m tools.spatial_oracle.walk_capture_entry --check
PYTHONPATH=. python -m tools.spatial_oracle.walk_prehead_response --check
PYTHONPATH=. python -m tools.spatial_oracle.engineer_repair_admission --check
PYTHONPATH=. python -m tools.spatial_oracle.engineer_family_selector --check
PYTHONPATH=. python -m tools.spatial_oracle.bridge_side_admission --check
```

## Production validation and remaining chains

The legal Hills regression uses ordinary CaptureBuilding from `(66,76)` and
requires production Foot/Walk navigation, Engineer consumption, a restored
structural deck and matching bridge/zone authorities. It saves the approach at
`(67,75)`, independently restores twice, compares the retained navigation state
and every subsequent simulation hash, and requires both continuations to repair
and retire the Engineer without stale CellList links. Native load reseeds
Scenario RNG, so these are two restored futures, not an assertion that loading
preserves the uninterrupted future.

The initial integrated run passes: Engineer entry and repair occur 76 frames
after dispatch (zero-based trace frame75); both restored continuations also
pass. This is Rust production evidence, not native timing or full route parity.
The saved receipt is `engineer-entry-followup/hills-side-consumers.log`.
Release/render validation and final PR readiness remain pending.

After integrating `main` at `c5451c8992964fe465b852de3e6c3ccae0498222`,
the legal Hills approach and both restore continuations pass again
(`engineer-entry-followup/hills-main-integrated.log`, retained test binary
SHA256 `c6389fb550ec64ca301ad0d311ecc48c5aa79d7b7e937ff2539bf4665653808d`).
The synthetic Capture detour needed its missing raw bridge-transition0x200
flag: its PathGrid already marked those cells as transitions. Live Foot
admission exposed that inconsistent fixture; route expectations were retained.

Focused validation before integrating current `main`: movement696 passed,
world-orders4 passed, pathfinding core126 passed, lifecycle15 passed and one
ignored, borrowed UnInit1 passed, SpeedType3 passed and all124 Walk response
rows passed. The final fixture-correction binary passes16 radar tests and four
constructor-side tests, including the six native admission answers through
`constructor_side_admission_matches_original_foot_receiver`. Its SHA256 is
`72a17ff359ebab7b88f9a2649f24beb0ff3846e8f47de48763803ad22dce3e03`;
logs are `engineer-entry-followup/side-fixtures-{radar,constructor}.log`.

Repair exposed an affected-consumer error: original bridge constructors leave
structural side cells without their own overlay. PathGrid projection, common
occupation, Anim MakeInfantry occupation and radar now read the live structural
flag, independently of sprite availability. The four constructor corpora pin
the cell fields; the side-cell admission corpus pins the native consumer
branches. Sprite emission retains its separate overlay requirement.

The ordinary Hills hut/Engineer has no attached Tag or Team. Active campaign
huts do: native519B3E synchronously raises hut Tag event1 before the type gates;
51A010 raises Engineer Tag event48; FootUnInit4DE5D0 removes Team membership
before ObjectUnInit5F65F0. Campaign Tag dispatch, Team teardown, special Doing
scatter, nonempty scatter recipients and JumpJet-specific continuations are
required separate mechanisms; this common path does not certify them.

The family scan, ordinary entry prefix and ordinary no-scatter suffix add no
RNG draws. Head selection can draw Scenario RNG, and the Walk response corpus
records the full state where it does. Timer writes and same-visit retries are
listed in the response report. Repair has its existing native damage/rebuild
effects and detachment owners; preserving these through production and restore
is part of this chain's pending integration validation.

Foot's path owner now retains the exact failure-sound byte at native+68A.
Ordinary construction clears it, while original raw Load and no-init Foot
construction preserve supplied1 and255. The existing sound consumer receives
the rules-named centred request; Walk's clear points follow the reached native
branches. Drive/Ship's first rejection can retry before clearing and therefore
does not consume the byte when merely requesting the sound. Hash feature222
adds only a nonzero tagged suffix; previous zero-state hashes stay unchanged.
No ordinary gameplay arming writer has been established. The direct/alias
scan is bounded and does not prove global unreachability; this remains an
audit item. No speculative command-side arming behavior was introduced.

Ghidra corrections are saved and read back: PerCell519CA3/519B3E/51A010,
AStar429E54/42A18B/42A4B6, InfantryType5236AA, and Cell483480 now named
`CellClass__Uncloak_Ground_Objects`. The previous redraw name hid a simulation
side effect. Exact receipts are retained with the Engineer follow-up evidence.
