# Original Bullet isolation and nullify tail

Six controls execute the full original Bullet detonation body `0x4690B0`.
They join the original AreaDamage receipt, animation selector, Scenario RNG,
and animation construction. The three supplied receiver states are an admitted
ordinary Unit, no receiver, and a Unit with an active Iron Curtain timer. Each
is run with `HE.EMEffect=no` and `yes`, read by the full native Warhead reader.

| Native AreaDamage result | EMEffect | Selector result | Constructed animation |
| --- | --- | --- | --- |
| 0, dispatched receiver | no | XGRYSML2 | XGRYSML2 |
| 1, no receiver | no | XGRYSML2 | XGRYSML2 |
| 2, Iron Curtain isolation | no | XGRYSML2 | IRONFX |
| 0, dispatched receiver | yes | XGRYMED1 | XGRYMED1 |
| 1, no receiver | yes | XGRYMED1 | XGRYMED1 |
| 2, Iron Curtain isolation | yes | XGRYMED1 | IRONFX |

In all three EMEffect controls, the native selector first executes
`RandomRanged(0,7)`: seed 31 produces raw word `2026076499` and index `3`.
The isolated path discards that selected ordinary animation only **after** this
RNG transition, then constructs IRONFX. Both constructor routes receive delay
0, loop 1, flags `0x2600`, native Z adjustment -15 and reverse 0. Original
Anim construction, Start, Unlimbo, position commit and Display submission run;
the constructors consume no additional RNG in these physical configurations.

Original layered `WeaponNullifyAnim` reads select IRONFX. Its full ART reader
loads the physical 4,128-byte, 48×46, 10-frame SHP from
`ra2.mix → conquer.mix`, entry `0x4ADCE3F6`; SHA256 is
`94d0dd4461d731a24ea2b2be7ca939b365e57d3d659017314dd6fff11ed9b220`.
This separate extraction leaves the earlier IFV input manifest unchanged.

```sh
source /Users/halvor/Documents/vera20k-dev/env.sh
PYTHONPATH=. PYTHONDONTWRITEBYTECODE=1 \
VERA20K_PROJECTILE_RENDER_ASSETS=/tmp/bridge-ifv-assets-1dDM9D/extract \
VERA20K_IFV_NULLIFY_ASSETS=/tmp/bridge-ifv-nullify-assets/extract \
/Users/halvor/Documents/vera20k-dev/.venv/bin/python \
tools/projectile_oracle/ifv_isolated_tail.py --check
```

`VERA20K_IFV_NULLIFY_ASSETS` may be omitted when `ironfx.shp` is already in the
main extracted input directory. Existing asset tooling can extract that file
without a build. No retail bytes are committed. Metadata pins the executable;
the saved result contains the physical file hashes and reader outputs.

`rows[].input` records receiver mode, authored EMEffect, live/placement XYZ,
frame, timer, health and RNG seed. `area_result` is observed from full original
AreaDamage, not supplied. `selected` and `constructed` distinguish the selector
result from the emitted effect. `constructor` records its actual arguments;
`events` preserves call/return/RNG order, including hashes at selector return
and constructor entry/return. Final native RNG hashes, animation count and
native identity cursor before/after support production comparisons.
`bullet_native_id` observes the retained Bullet's original constructor-assigned
ID 32; the partial fixture cursor is 33 before detonation and 34 afterward.

The receiver's `Unit::ReceiveDamage(0x737C90)` remains an explicit boundary:
it returns zero without mutating health. AreaDamage's original collector,
Iron Curtain timer, isolation and receipt execute around it. Native comparisons
therefore cover the enclosing receipt and animation/RNG ordering, not receiver
health or kills. Cells, occupancy and Unit state are prepared; there is no
bridge topology in these controls. Bullet's constructor CombatLight flag is
false, so the ordinary CombatLight body is not exercised. Sound binding is
absent. The test stops at Detonate return with one admitted animation and a
live Bullet; later scheduling, rendering and retirement are separate witnesses.

The production Rust composition passes all 6 controls on the final focused
candidate, SHA256
`e12b7834225b63dcef6b0f6c8d5b596a9bcfc0b1cb3c930bdcfefd32bc76ef20`
(1.21 s). It uses production AreaDamage and Bullet effect delivery with physical
HE/IRONFX configuration, comparing the receipt, animation type/constructor
fields, Scenario RNG bytes and native cursor delta. The partial registry prefix
remains a fixture boundary, not a whole-game identity. Rust executes its actual
receiver, so the native supplied ReceiveDamage result does not establish
receiver health or kills. The `EMEffect=yes` rows are explicit controls, not a
claim that retail HE enables the key. Existing `ifv_select_anim` and
`ifv_area_receipt` fixtures remain unchanged. Final chain-wide validation is
recorded separately in the [IFV ledger](ifv_launch.md).
