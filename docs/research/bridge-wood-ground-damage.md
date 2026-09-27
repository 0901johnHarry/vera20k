# Ordinary wooden bridge damage on Shrapnel

This chain follows the untagged stock `XShrapnel.MAP` span at x114..116,
y54..64: an Engineer repairs the authored gap, an ordinary MTNK force-fires
at115,59, damage rechecks residents and incoming Drive heads, collapse releases
the Cell target, and a second Engineer rebuilds the crossing. Native executable
comparisons establish the bounded functions below. Production tests exercise the
connected Rust route. This is not a whole-bridge closure claim.

## Authority and implementation

`bridge_state::ordinary_damage` owns both ordinary overlay families. Wooden
74..101 and concrete205..232 identify artwork/control families, not elevation.
The common host publishes each raw Cell overlay, its OverlayGrid and optional
runtime mirror, resident TMP recalculation, live occupants and navigation before
returning to the caller. The serialized dynamic-cell owner retains each write.
Area damage and hut damage call this same owner. The old deferred wooden walker,
its duplicate classifiers and its invented structural fallout are removed.
Hut sweeps construct each preceding explosion immediately through the existing
Anim owner, before the next cell's draws or live occupant receivers. The obsolete
deferred animation buffer is removed for both ordinary families.

The remaining structural body/ramp controllers retain their own flags, anchors
and ordered fallout. Neither ordinary wooden nor concrete damage calls
`CellClass::BlowUpBridge47DD70`. The ordinary controller never adds an elevated
movement layer, drops objects from a deck or produces structural debris.

The native low selector57BAA0 chooses the middle of the three-cell width. NS
overlays have width along Y and propagate along X; EW have width along X and
propagate along Y. Root57BCF0/57C2B0 stores negative side, positive side, center;
sibling57DD50/57E2A0 stores center, negative side, positive side. Each touched
triple executes all three47D2B0 calls, then all three487A10 calls, synchronously.
Retained cells survive recursive callbacks; subsequent selection reads live
fields. Signed16 coordinates and the shared Dummy preserve native lookup effects.

Both wooden roots pass the final-collapse byte to487A10. First damage therefore
uses0; final100/101 uses1. Siblings use1 only when their own selected result is
100/101. Concrete EW's unconditional1 is deliberately preserved as a family
difference. A root returnsfalse on first damage despite completing its effects;
only final root collapse returnstrue. Sibling effects do not change that AL.

Final root collapse marks radar cells, propagates siblings, scans endpoints and
offers575EE0 notification, then recalculates/rechecks root occupants, calls56C510
and the X-major3x3 rectangle rebuild. The area caller releases the original Cell
target via70D4A0 only after the synchronous driver returns true.

## Retail data and native packets

Original `gamemd.exe` SHA256:
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
Unchanged mapped `.text` SHA256:
`4cd5557a7490debc493ff965afc4483d8d2f1065f434f6b665cbb8fc4835b0cc`.
Physical Shrapnel SHA256:
`c32e412e938e1b9a0b29f89ff9bc0e6f410f75bc76701fb4acef6581fd19b438`.

The original OverlayTypes reader resolves74=`LOBRDG01`,100=`LOBRDG27`,
101=`LOBRDG28`. The two terminal overlays have `NoUseTileLandType=false`.
Recalc therefore exposes the existing underlying Water at the gap; it does not
replace the TMP. Earlier Ghidra notes confused INI key labels with list ordinals
and incorrectly claimed that all terminal wood overlays remained Road. The five
selector/root/sibling labels and comments have been corrected, saved and read
back exactly; the prior annotations are preserved with the external receipt.
The two wooden hut-walker comments also had their terminal caps reversed:
575220 checks100, and575540 checks101. Both comments now state the original
constructor-before-damage order and have saved, exact readback receipts.

The selected physical cells remain level2/Z208 with raw bridge flags0.
There is no structural deck/Tube/BridgeRecord or selected CellTag prerequisite.
The map's two huts are117,56 and113,62. The physical rules layers contain no
selected MTNK/105mm/AP/General overrides; native readers and retained manifests
cover RULESMD, absent LANGRULE, MPBattleMD, map, ARTMD and SNOWMD as applicable.

| Boundary | Native evidence |
|---|---|
| Selector, roots, siblings, return/store/callback order | [Scalar runner](../../tools/spatial_oracle/shrapnel_damage/scalar.py): all73..102 overlays at all three width entries; both axes, rejected controls and three physical retained sequences |
| Rust golden projection | [Scalar vectors](../../tools/spatial_oracle/shrapnel_damage/scalar_test_vectors.json): extraction from frozen native output, no expected-value implementation |
| Live MTNK, Drive and death effects | [Joined occupant runner](../../tools/spatial_oracle/shrapnel_damage/occupants_joined.py): seven retained two-stage cases, original47D2B0/487A10/admission/C4/list lifecycle/conditional Cell detach and Anim constructors |
| Migrated wooden hut caller | [Joined hut runner](../../tools/spatial_oracle/shrapnel_damage/hut_joined.py): both physical huts, original574C20/574780/575540 and live constructor/receiver order |
| Full initial and changed navigation | [Navigation runner](../../tools/spatial_oracle/shrapnel_damage/navigation.py): native-derived SNOW inputs, complete planes and ordered graphs |
| Save/restore navigation | [Restore runner](../../tools/spatial_oracle/shrapnel_damage/navigation_restore.py): four independent original581F50 replays |
| First repair and native retail inputs | [Shrapnel repair packet](../../tools/spatial_oracle/shrapnel_repair/README.md) |

The [full-map navigation packet](../../tools/spatial_oracle/shrapnel_damage/navigation.md)
derives all initial class, slope, height and navigation inputs through the original
readers, Cell/Terrain constructors, placement and Recalc. It supplies no VERA planes
or graph IDs. Original Map/Theater reading establishes Scenario+1258 as SNOW=1
before terrain occupation is selected. The packet retains all 13 movement rows
and three complete ordered graphs for the authored map, first repair, first damage,
collapse, second repair and repeated repair. First damage leaves navigation
unchanged. Collapse exposes three Water/class4 cells. Rebuilding restores Road
and the original base navigation; incremental hierarchy IDs retain allocation history.

Four independent native `581F50` replays reconstruct saved healthy, damaged,
collapsed and repaired boundaries. Their source planes and rebuilt graphs match
the production restore exports. This covers the post-load navigation rebuild
called at `67E8CD`; it does not emulate the native SaveGame file or pointer swizzling.

## Occupants, RNG, timers and detach

The controller itself consumes no RNG. The outer area caller admits
DestroyableBridges/Wall and consumes the Scenario1..BridgeStrength roll, except
the IonCannon identity bypass. Successful first damage still returnsfalse;
target detach is reserved for final root collapse.

The native occupant packet proves that a resident MTNK on the new Water dies,
as does a north neighbor whose actual Drive head enters the gap while its current
cell remains Road. The resident list caches Next before callbacks, so both
co-residents are visited. Incoming-head traversal reads live Next after callbacks;
the first death can unlink the pointer and leave its successor unvisited. A
stationary north Road tank survives. Replacing its target during callbacks is
observed by the later Cell detach: a different target persists, the same Cell
is cleared.

The preserved request-boundary packet stops at421EA0. The joined packet executes
the actual constructors, Bouncer initialization, Start, Logic and display admission.
The first DBRIS7LG constructor adds six Scenario draws, changing the subsequent
explosion pick from the request-boundary S_BRNL58 to TWLT070. One lethal callback
finishes at Main indices1/104, Scenario9/112 and unchanged MapGen0/103. Packets
retain complete streams, receiver arguments, timer writes, object-list mutations,
constructor states, relative native IDs and detach order. The two-resident case
also compares its later draws and ordered three-effect result.

Native House+5574 counts active units. The production comparison observes the
existing HouseTracking owner, subtracting unrelated scene members; EntityStore's
stored-object count includes retained dead actors and is not that native field.
RadarCombatFlash start/duration writes on the dead native object are retained in
evidence. An independent read watch observes no reads of that timer and no
`70D990` radar-update calls during the four measured lethal cases. Cleanup removes
the actor from cell, Logic and display membership before the controller returns.
This bounds the absent Rust timer to dead-object state in these cases; it does not
establish surviving-damage, global trigger or radar-presentation equivalence.
The native sound requests are retained, but playback is outside the packet.
Rust's existing consequence delivery reverses the two same-frame death/report
sound starts; this inaudible presentation-order residual is not a sound parity claim.

## Production validation and limits

The retained production tests exercise physical repair/damage/collapse/
second-repair and four independently restored future cases. Ordinary
retail MTNK firing reaches first damage atLogic1258, collapse1625 and consumes
the second Engineer at1664 in that fixed Rust world. These frame numbers are
Rust regressions, not native whole-world scheduling goldens.

The joined Rust comparisons pass all seven supplied occupant cases and both
physical hut calls, including full RNG state, ordered live memberships, relative
native IDs and Anim runtime/Bouncer fields. Frame1000 and game speed4 are
explicit supplied boundaries on both sides; normalized Anim rates are not taken
from the loaded scenario's default speed. House totals use the existing active
unit owner. Separate scalar/controller and cache-reconciliation checks pass24
tests. The full retail library suite passes9550 tests (211ignored), clippy passes, and
the release build loads physical Shrapnel through the shared headless scenario
loader and advances10 frames. Build identities, commands and literal results
belong to the [validation receipt](../../tools/spatial_oracle/bridge_wood_validation.json).

The offscreen witness uses the production overlay builder, active SNOW atlas and
GPU submission. It checks source-art color/depth, camera/fog admission and
simulation-hash independence. This proves output from the selected source art,
not native full-screen composition. Ordinal101 has empty `LOBRDG28` frames;
neighboring90/91 end stubs remain visible after collapse.

Required later chains include tagged event31 and its live Tag/Trigger lifecycle,
the Bomb/Building hut destruction prefix and structural fallback, raised wooden237/238
structures, other physical orientations, and remaining affected consumers. This
chain migrates both ordinary-low damage callers without claiming full native
parity for every hut-trigger prerequisite or the global world scheduler.
