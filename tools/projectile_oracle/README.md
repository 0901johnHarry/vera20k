# Projectile native comparisons

These tools execute the pinned retail `gamemd.exe` under Unicorn. See the
[shared native runner](../native_oracle.md) for installation and executable
identity. This page currently indexes the consolidated FireAt-tail fixture;
other projectile fixture families remain separate and need their own inventory.

## FireAt tail and BulletFire

`fireat_fixture.py` owns the synthetic object/stack initialization, virtual and
world hooks, checked native execution and observations for these five generators:

| Module (`python -m tools.projectile_oracle.<module> --check`) | Cases | Coverage |
| --- | ---: | --- |
| `fireat_launch` | 144 | Ordinary arcing/lobber/floater and speed/delta combinations |
| `directed_launch` | 64 | Original Unit heading receivers, turret/hull heading and Dropping origin reset |
| `building_pitch` | 64 | Original building/base-Techno pivot getters at supplied building heights |
| `voxel_launch` | 20 | Voxel/Vertical combinations and native maximum speed |
| `arc_second_probe` | 96 | Second arc-solver result with controlled prior stack bytes |

Each generator owns its case matrix; shared execution starts at `0x006FE8EE`
and must reach `0x006FF01A` or `0x006FF93C` within the checked runner's instruction
and time limits. Successful ordinary-tail returns also require temporary and
persistent velocity bytes to agree. Failures raise exceptions, including under
optimized Python. Importing a generator or asking for `--help` does not load the
retail binary, execute native code or write references.

The default and `--check` reproduce and compare the reference without writing.
`--write` explicitly replaces output and its `.meta.json` provenance sidecar;
review any change before accepting it. The sidecar records the verified binary,
Unicorn versions, exact canonical payload hash, fixture assumptions and substituted
receivers. It certifies the declared corpus only.

Rust consumers:

- `sim::projectile::launch::tests::original_fireat_and_fire_preserve_exact_launch_bits`
  consumes all five corpora (388 rows) and compares launch success and successful
  binary64 velocity bits using verified retail math tables.
- `sim::world::projectile_collision::tests::runtime_fireat_fractional_velocity_survives_live_gravity_and_snapshot`
  consumes `fireat_runtime.json`: a composition of the existing native Rules
  Process, GetSpeed, shared launch and motion fixtures. It checks production
  parsed inputs, the firing frame's first motion visit and snapshot continuation.
  Reproduce with `python -m tools.projectile_oracle.fireat_runtime --check`.

The runtime composition preserves the two different speed values: the supplied
numeric rules produce stored weapon speed 95 (the postpass runs with prior
Gravity=3), then GetSpeed derives launch speed 59 using the live Gravity=6 and
500-lepton distance. The original ignored test assumed speed 100 and delayed the
first AI until the next frame, and applied projectile ART after its authoritative
read stage. Those expectations predated the native speed, live Logic-vector and
retained ART-state migrations. The repaired fixture supplies ART during rules
processing and asserts the resulting Voxel/Vertical flags. The native loop reloads its count at `0x0055B613`
after object AI, and Bullet Unlimbo appends at `0x005F5040`. The composition checks
numeric components; it supplies the connections and admitted collision commits.
The Rust runtime test establishes that integration only for these two fixtures;
it does not certify the whole native scheduler or collisions.

These are ignored retail tests; set `RA2_DIR` to the verified install and run each
with `python -m tools.cargo_run -- test -p vera20k --lib <test-name> -- --ignored`.
Python import/help regression tests run in `python -m tools.run_tests` without
retail files. Native comparison and Rust regression are separate evidence levels.

The fixtures supply source/target/type/weapon/Rules/stack state, zero-fill other
fixture memory and set the recorded x87 control word. Hooks substitute selected
virtual receivers and world insertion, not numeric results. They establish bounded
launch-tail arithmetic, not full FireAt admission, retail reader construction,
world registration, downstream impact or whole-game parity. In particular,
`bridge_render_flight` deliberately keeps its separate reader/constructor-backed
fixture; collapsing it into this synthetic setup would discard upstream coverage.
