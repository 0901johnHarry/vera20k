# Bridge-collapse metallic debris

This chain connects `CellClass::BlowUpBridge` (`0x0047DD70`) to the existing
AnimStore constructor, Bouncer flight, contact/landing receivers and cleanup.
It also fixes the shared RulesClass animation vectors and their asset binding;
ordinary death debris consumes the same corrected metallic vector. This document
bounds the evidence for this chain, not whole-bridge parity.

## Rules and assets

The original vector constructors (`0x00665827..0x0066585F`) initialize both lists
empty. `ReadGeneral` reads `MetallicDebris` (`0x0066DA90`) and `BridgeExplosions`
(`0x0066DB93`) through `ReadString` with capacity128. It truncates to127 bytes,
trims the whole buffer, splits only on commas, and calls the original AnimType
factory. A zero-length result retains the previous list. A nonempty result
replaces it, even when comma-only input or exact `none`/`<none>` tokens produce
an empty list. Individual tokens retain spaces; successful duplicate references
retain their order and first registered spelling.

The production retail `RULESMD.INI` value authors20 names but the native read
produces **15**, ending in literal **`D`**. The first14 have ART bodies and
15-frame,30x30 SHPs. `D` has neither an ART body nor an image; its real AnimType
retains constructor defaults and is not a Bouncer. The four bridge explosions
are `TWLT026`, `TWLT036`, `TWLT050`, `TWLT070`, with17,17,17,26 raw SHP frames.
The supplied `LANGRULE.INI` is absent; `MPBattleMD.ini` and `Hills.mmx` omit both
keys and retain the prior lists.

[`RulesPassProcessor`](../../src/rules/native_processing.rs) owns the retained
vectors for one ordered RulesClass processing stack and publishes them through
`ProcessedRulesLayers` to [`RuleSet`](../../src/rules/ruleset.rs). Process-resident
Type registries remain a different owner. GeneralRules and BridgeRules no longer
reparse the merged INI or invent a20-entry default. Configuration hash version7
includes both resolved ordered vectors. Simulation keeps an interned projection
for allocation-free reference selection; map binding and validated restoration
rebuild it from the bound rules, and snapshots omit that derived cache.

Animation roots and both binders preserve literal type identities. The dedicated
animation Image value is read from the exact type's ART section with the native
25-byte buffer/current-ID default (`ObjectType::ReadINI 0x005F933B`). The loader
uses literal type ID only when that value is empty (`0x00427B9F..0x00427BBC`).
Thus `[ FX] Image=REAL` differs from `[FX] Image=WRONG`; an absent Image in the
former section can legitimately trim its default to `FX`. Generic object and
particle image resolution is unchanged. Binding, atlas candidates and smudge
frame dimensions share this animation authority.

The ART-read receipt gives registered unread types constructor metadata before
binding. They enter the scheduler without loading an orphan image. The headless
loader now applies that receipt before binding its ART copy, matching ordinary
and generated app loading. Both atlas candidate generation and the existing
presentation gate reject image drawing for unread `D`.

## Producer and consumers

[`spawn_bridge_debris`](../../src/sim/world/bridge_orchestrator.rs) reads signed
Cell.Level and emits at `level * 104 + 416`, including after structural bridge
flags disappear. Each admitted cell takes the original outer gate, X/Y jitter,
metallic gate and metallic selection, constructs that AnimClass immediately,
then draws the sibling explosion delay and selection. The metallic constructor's
RNG and immediate Start therefore precede the sibling draws. Coordinates retain
the native final-absolute-coordinate truncation, including negative cells.

The existing AnimStore remains the only animation lifecycle owner. Its Bouncer
body updates the exact world coordinate; contact walks the landing cell's ground
object list and delegates entity and Terrain receivers to their existing owners.
Landing damage uses the existing area-damage dispatch, including bridge-state
notifications. Destruction removes animation ownership through the existing
lifecycle. No parallel debris effect store is introduced. The joined comparison
below extends producer evidence through the primary animation's flight and
landing. Its empty contact lists do not establish receiver behavior by themselves.

The joined native corpus schedules only the primary metallic animation, starting
at binary frame 1000. The Rust test applies that same boundary: the sibling
explosion, smoke trailers and landing children construct immediately but do not
receive AI visits. It compares every primary visit's exact position, nonzero
velocity and Bouncer scalar bits, world coordinate, frame/delay/loop/timer state, child
construction order, full Scenario RNG state and four continuation draws. Its
32 cases cover four seeds across ground at levels 0 and 4, water, structural deck,
water beneath a deck, mesa, pit and cliff fixtures. Structural deck landing
executes original area-damage admission, including the rejection-sampled draw
that consumes three raw Scenario values in seed 1.

On a Stopped result, native queues the primary twice; original drain `0x00725C70`
removes both entries and calls the scalar destructor once. Rust's idempotent
Destroy queues once and its existing drain finalizes once. The comparison checks
the resulting logical destruction and physical removal, while populated expiry
observers and duplicate-call side effects remain outside these empty-observer
fixtures. Rust's `inactive` flag also represents deferred deletion, so it maps to
native Alive for this comparison, not directly to native field `+0x19B`. Native
quaternion bytes are not represented or compared; these zero-spin SHP animations
do not consume orientation when drawn.

The existing flat-reflection/cliff-zero shortcuts can store `+0` velocity where
native matrix arithmetic stores `-0` ([prior bounded comparison](PHASE3_BOUNCE_GROUND_QUERY_DELIVERY_NATIVE_REPORT.md)).
The joined test excludes only that sign difference on a terminal zero-elasticity
visit. The stop calculation converts velocity to integers, landing consumes the
position, and the animation is destroyed before another physics visit. No
coordinate, outcome, timer or RNG tolerance is introduced; full native body-bit
parity is not claimed. Rust retains its deterministic stored bits for hashing
and snapshots.

Terrain receivers require the native Strength fallback. The original TerrainType
constructor initializes Strength to `-1` (`0x0071DBAC`); ObjectType's exact-key
read retains the current field as its default (`0x005F94D3..0x005F94F3`), and
TerrainType replaces a successful read's remaining `-1` with current TreeStrength
(`0x0071DEC8..0x0071DEDC`). The fresh production reader now implements that
fallback. The [retail terrain inputs](../../tools/spatial_oracle/bridge-retail-terrain-inputs.json)
establish TreeStrength 200, absent TREE01/TIBTRE01 Strength and no relevant
LANGRULE/mode/Hills override for this selected chain. See the
[native reader comparison](../../tools/spatial_oracle/terrain_strength.md).

One required layered-reader discrepancy remains for later work: an already-read
TerrainType retains its numeric Strength when a later layer changes only General
TreeStrength, whereas Rust currently merges type sections and constructs Terrain
types against the final General value. The saved two-pass native rows give 200
after an omitted Strength and 375 after explicit `Strength=-1`. This does not
affect the selected Hills inputs, but it prevents a broader layered Terrain
reader equivalence claim and remains open in the whole-bridge goal.

Terrain object coordinates now have one retained authority in
[`TerrainObjectState`](../../src/sim/terrain_object.rs). Construction centers
the signed map cell and clamps input Z0 against the native ground surface;
structural deck flags do not raise a tree onto the deck. Original TerrainType
placement `0x0071E0D0`, Map ground lookup `0x00578080`, raw coordinate write
`0x005F6940` and GetCoords `0x005F65A0` provide the
[41-row native corpus](../../tools/spatial_oracle/terrain_coordinate.json).
The Rust comparison covers its 40 authored-construction rows; the extra row has
nonzero caller-supplied Z. The retained Z participates in the object's hash and
snapshot version 215. Area collection, direct contact, lethal nested C4 and
terrain presentation consume that retained XYZ after ground changes, instead
of resampling current ground or deck height.

The separate [24-row Terrain contact corpus](../../tools/spatial_oracle/terrain_debris_receiver.json)
executes original Anim contact admission and Terrain ReceiveDamage through
ordinary damage gates, immediate Limbo, ground-list unlink, Logic removal and
deferred deletion. Its Rust regression compares 19 conditional single plain-tree
contact rows, including radius and receiver gates. Direct-only receiver rows,
synthetic two-tree lists and custom SpawnsTiberium with failed allocation remain
native characterizations outside that Rust comparison. The empty-list joined
flight corpus and populated conditional contact corpus are separate evidence;
they do not prove a single native loaded-map flight striking a centered tree.

Native47DD70 does not guard an empty MetallicDebris vector before indexing it.
Rust deliberately skips that invalid-list metallic spawn and continues the
explosion; it does not restore fabricated retail defaults. This is safe handling
of an invalid native input, not a native successful-construction equivalence.

## Reproducible comparisons and validation

All native corpora use Unicorn2.1.4 and original `gamemd.exe` SHA-256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
The companion `.meta.json` files record inputs, substitutions and payload hashes.
From the repository root, with `VERA20K_GAMEMD_EXE` configured:

```sh
python -m tools.rules_oracle.bridge_anim_lists --check
python -m tools.rules_oracle.anim_image --check
python -m tools.spatial_oracle.bridge_debris_producer --check
python -m tools.spatial_oracle.bridge_debris_flight --check
python -m tools.spatial_oracle.terrain_render --check
python -m tools.spatial_oracle.terrain_coordinate --check
python -m tools.spatial_oracle.terrain_debris_receiver --check
python -m tools.spatial_oracle.terrain_strength --check
```

| Boundary | Native evidence | Rust/production validation |
| --- | --- | --- |
| Constructors and retained list reads | [24 sequential rows](../../tools/rules_oracle/bridge_anim_lists.json), original ReadString/strtok/factory/constructors/vector copy; supplied cached INI indexes. Native `--check` passed. | `bridge_animation_lists_match_original_retained_vectors`, `retail_bridge_animation_vectors_match_original_reader_and_unread_d`; passed with required retail INIs. |
| Image25 and empty-image fallback | [11 rows](../../tools/rules_oracle/anim_image.json), original reader and loader selection; exact section/current default supplied from inspected caller. Native `--check` passed. | `animation_image25_matches_original_reader_and_loader_selection` and exact-token binder tests passed; atlas regression passed after including the existing generic-letter filename retry in its expected candidates. |
| Actual bridge producer and constructors | [Producer corpus](../../tools/spatial_oracle/bridge_debris_producer.json), original47DD70/AnimType ctor/AnimClass ctor/immediate Start; all15 metallic slots exercised. Actual ART scalar/image-header inputs supplied from production export. | Production-reader producer/RNG comparisons and shared death-loop12 rows passed, including seed5 selecting `D`. |
| Primary flight, empty contact lists, landing and cleanup | [32 joined cases](../../tools/spatial_oracle/bridge_debris_flight.json), original primary AI/Bounce/result handling, dry/water/deck landing, area bridge admission and native drain; [explicit boundaries](../../tools/spatial_oracle/bridge_debris_flight.md). Native `--check` passed. | `native_bridge_producer_primary_flight_landing_and_rng_continuation` passed all32 histories through final physical removal. The separate `debris_contact_matches_original_tree_damage_gates_radius_and_retirement` passed its19 covered contact cases. |
| Terrain retained-coordinate rendering | [82 native rows](../../tools/spatial_oracle/terrain_render.json), original Render projection and ordinary static DrawIt through the shape-call boundary, before pixels. | `terrain_retained_xyz_projection_and_piece_z_match_original_render` uses the production projection and static-pair helper, compares draw points with explicit world/dirty-rectangle translation and exact body/shadow gradients and Z-adjust. All82 rows passed. |
| Mixed scheduler and persistence | Native Load reseeds Scenario RNG; the primary-only native corpus does not establish whole-world scheduling. | [Retail force-fire example](../../examples/bridge_forcefire.rs) follows ordinary Hills collapse for 200 further frames. The [ignored production test](../../src/sim/combat/bridge_live_chain_tests.rs) restores live debris through the production snapshot/fixup APIs twice and compares both loaded continuations for 200 frames. Final release/test execution pending; this is production composition, not native whole-frame proof. |
| Retail loading and rendering | AssetManager/ART/SHP production inputs are recorded in [retail input export](../../tools/spatial_oracle/bridge-retail-anim-inputs.json); no native pixel comparator. | Owner to insert release map-load result, visible collapse evidence and artifact paths. |
| PR candidate checks | No claim from a parser pass alone. | Owner to insert final retail-required full `--lib` suite, clippy and single fresh critic outcome. |

The earlier research266-row fixture supplied20 metallic types,16-frame images and
three synthetic explosions. It established only that supplied producer/constructor
composition. It is **not retail evidence** and is superseded for retail selection
and constructor continuation by the15/4 corpus above. The older
`anim_bouncer_launch` fixture remains bounded evidence for its explicit supplied
inputs, not the shared production MetallicDebris pool.

Still-unproven boundaries include native physical INI and asset loading as one
joined run, native GPU output, complete custom animation lifecycle branches, and
all bridge mechanisms outside this producer chain. The producer fixture
supplies map lookups, marking/display callbacks, allocations, ART values and
reference pointers; it omits DBRIS1LG's trailer pointer and delayed explosion
Start behavior from its producer claim. The joined flight fixture binds the
actual smoke trailer, wake and splash references, but still leaves all child AI,
audio Report effects, populated destruction observers and actual combat-light
creation outside its execution boundary. These limits are not resolved by
matching RNG snapshots or passing the library suite.

The production animation renderer reads each Bouncer's updated exact world
coordinate, selects its current frame and native display layer, and applies the
world-Z projection. An ordinary visible Hills collapse and a GPU screenshot are
the concrete rendering-validation route; the existing fixed tactical capture
does not drive this chain. Pixel equivalence is not established here. A separate
bridge rendering follow-up remains in the projectile screen-position consumer:
it treats a raw Z lepton value as a clamped map level. That consumer is outside
this animation producer chain.
