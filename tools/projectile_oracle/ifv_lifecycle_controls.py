"""Original guided states isolating lifecycle regression fixture boundaries."""
from pathlib import Path

from tools.native_oracle import finish_vectors, provenance
from tools.projectile_oracle import ifv_guided_controls as controls


def specs():
    for height in (1, 0):
        yield dict(
            name=f"ground_old{height}_level_no_acceleration",
            old=[2688, 5248, 624 + height],
            velocity=[4.0, 0.0, -2.0],
            target=(17, 20),
            maximum=4,
            type_keys={"Level": "yes", "Acceleration": "0"},
        )
    yield dict(
        name="expired_null_below_safety",
        old=[2688, 5248, 1024],
        velocity=[64.0, 0.0, 0.0],
        target=None,
        maximum=0,
        locked=True,
        safety=500,
    )


def generate():
    original_specs = controls.specs
    try:
        controls.specs = specs
        result = controls.generate()
    finally:
        controls.specs = original_specs
    result["scope"] = (
        "Three supplied incoming guided states through original4668BD..467B7A: "
        "Level/no-acceleration ground crossing and target-null below safety. "
        "No pointer-expiry, snapshot, scheduler, damage or retirement execution."
    )
    return result


def metadata():
    return provenance(
        scope="Original guided controls for two lifecycle-test fixture assumptions",
        assumptions=[
            "Tracked ifv_guided_controls/guided_step setup executes original Bullet construction/configure and physical layered Weapon/Bullet/General readers. This is not complete Rules Process chronology.",
            "Map cells are supplied flat level6. Native ID100/frame1, incoming XYZ/velocity, maximum speed, counters/lock, null source and target kind are supplied. Level and Acceleration overrides execute the full original BulletType reader. Safety500 is a supplied retained Rules scalar.",
            "All three controls stop before world coordinate commit. No original pointer expiry, save/load, Unit scheduling, collision receiver or deletion is executed here.",
            "Rust ground fixture translates native floor624 to208 (level6 to2), and cells[10,20]/[17,20] to[5,5]/[12,5]. Its first step compares the native relative crossing; phase-independent Level/yaw-aligned candidate is the bounded comparison.",
            "Rust null fixture translates floor624 to0 and places the Bullet at[1024,1024,400]. ID100/frame1, incoming velocity, maximum, latch and guidance flags match. Velocity and height deltas are comparable in the zero-target pitch arm; its coordinate-dependent closing accumulator is not compared.",
        ],
        substitutions=[
            "Inherits tracked BulletReader physical lexical INI caches and archive byte delivery, allocation/delete/CRT TLS and verified OS imports. Observation records original calls and rejects RNG. Only the Python supplied-state table differs; original instructions and callees are unchanged.",
        ],
        entry_points={
            "guided_ai": 0x4668BD,
            "homing": 0x5B20F0,
            "null_safety": 0x466E70,
            "precommit_stop": 0x467B7A,
        },
    )


if __name__ == "__main__":
    finish_vectors(generate, Path(__file__).with_suffix(".json"), provenance=metadata)
