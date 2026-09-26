# Retained Terrain coordinates in drawing

`terrain_render.py` preserves 82 original-engine captures: the 41 inputs from
`terrain_coordinate.py`, crossed with viewport/dirty-Y pairs `(0,0)` and
`(37,100)`. The original Terrain placement, Object coordinate storage, Terrain
Render suffix, projection, height getter and DrawIt execute. After placement,
the source cell becomes level 7/flat; rendering still reads the retained XYZ.

The binary is active-retail `gamemd.exe`, SHA-256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
With the existing Unicorn environment and `VERA20K_GAMEMD_EXE` or `RA2_DIR` set:

```sh
python -m tools.spatial_oracle.terrain_render --check
```

`--write` deliberately regenerates the JSON and provenance. The checked payload
records the raw retained coordinate, projected point and all 14 native draw
arguments, along with their interpreted fields. No executable or image bytes
are distributed. Native regeneration and `--check` passed when this corpus was
introduced; Rust and production validation are recorded by the chain owner.

## Native owner and calls

Terrain's actual vtable `7F522C+104` is Render `71CC50`, whose admitted suffix
`71CD22..71CD81` calls center getter `41BE00` through `+AC`. That getter calls
Terrain's `+48 = 5F65A0`, copying retained Object Location. At `71CD3D`, original
`6D2140` projects the XYZ and subtracts the tactical camera. The suffix then
rebases for dirty clipping and calls `+114 = 71C1B0` at `71CD7B`. The active
Tactical rendering loop dispatches the `+104` slot (for example `6D916C`);
its class/visibility admission is outside this fixture. The older generic
Object Render `5F4B10` is not Terrain's `+104` owner.

DrawIt copies its supplied point at `71C234..245`. The `+1D0` call at `71C249`
is Terrain's original `5F5F30`, which reads signed Location.Z at `Object+A4`.
Original `6D20E0` converts it to pixel lift. The ordinary body at `71C304` uses
Z adjustment `-lift-12`, gradient 2, flags `4E00`; shadow at `71C34E` uses the
same point, adjustment `-lift-3`, gradient 0 and flags `4E01`. No second ground
or bridge-height query changes either position or depth adjustment.

Examples from original execution at world XY `(2688,5248)`, before camera or
VERA's constant world-row bias:

| Placement input | Retained Z | Point | Lift | Body/shadow Z adjustment |
| --- | ---: | --- | ---: | --- |
| Level 0, flat | 0 | (-300,465) | 0 | -12 / -3 |
| Level 2, flat | 208 | (-300,435) | 30 | -42 / -33 |
| Level 2, slope 1 | 260 | (-300,428) | 37 | -49 / -40 |
| Level 2, slope 15 | 312 | (-300,420) | 45 | -57 / -48 |
| Level 2, supplied Z 999 | 999 | (-300,321) | 144 | -156 / -147 |

The structural `0x100` flag does not lift Terrain's ground placement to a
bridge deck. The companion coordinate oracle establishes that behavior.

## Production comparison and limits

The changed `overlays.rs` reads `TerrainObjectState::world_coord`, projects it
through `absolute_leptons_to_screen`, and supplies the same retained-Z lift to
`native_static_terrain_instances`. VERA's established world frame adds 15 to
native absolute screen Y. Its temporary half-tile subtraction/addition cancels
before the pair is emitted. `lifted_z_adjust` matches the native body/shadow
arguments; `compute_sprite_depth_params_lifted` cancels the lift for the legacy
sort/depth scalar. These are source comparisons; the accompanying Rust test
must establish the named production helper comparisons separately.

Only `CC_Draw_Shape4AED70` is substituted: it records arguments and returns
without drawing. Map lookups, coordinate getters, projection, AdjustForZ and
the Terrain DrawIt body run unchanged. Fixture state supplies a synthetic
4-frame SHP, ordinary nonanimated/non-SpawnsTiberium Terrain with health 200,
nonnull Cell Convert, brightness 1000, shadow enable, runtime startup constants
and clipping/camera locals. The suffix begins after visibility and rectangle
admission. Consequently this corpus does not prove image loading, GPU output,
dirty-region admission, full render scheduling or complete Terrain rendering.
Existing raw/animated Terrain and SpawnsTiberium drawing residuals remain
separate; no whole-bridge or whole-Terrain parity claim follows from these rows.
