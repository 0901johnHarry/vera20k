"""Check native directed heading, Unit +308 receivers, and Dropping origin reset."""

from pathlib import Path

from tools.native_oracle import finish_vectors
from tools.projectile_oracle import fireat_fixture


def run(dx, dy, dz, speed=100, arcing=True, lobber=False, floater=False,
        dropping=False, rot=-1, turret=False, hull=0, barrel=0):
    return fireat_fixture.run(
        dx, dy, dz, speed, arcing, lobber, floater, variant="directed",
        dropping=dropping, rot=rot, turret=turret, hull=hull, barrel=barrel,
    )


def generate():
    return [
        run(500, 200, dz, arcing=arc, dropping=drop, rot=rot,
            turret=turret, hull=hull, barrel=barrel)
        for dz in (0, -200)
        for arc in (False, True)
        for drop, rot in ((True, 0), (False, -1))
        for turret in (False, True)
        for hull, barrel in ((0, 16384), (16384, 49152), (65535, 0x1234), (0x8123, 0xCDEF))
    ]


def metadata():
    return fireat_fixture.launch_provenance(
        scope="64 synthetic directed FireAt-tail/BulletFire cases using original Unit heading receivers and Dropping origin reset. Full source construction, FireAt admission and world insertion are excluded.",
        assumptions=[
            "Source coordinates [1280,1280,0] differ from supplied launch origin [1390,1320,80]. Successful Dropping launches must retain source coordinates; other successful launches must retain the supplied launch origin.",
            "Original Unit +308 at 0x00740F80 and +2A8 at 0x00746E30 execute on supplied source type, hull/barrel words and Turret byte. The matrix crosses two Z deltas, Arcing, Dropping/ROT pairs, Turret, and four facing pairs at speed 100.",
        ],
        entry_points={"unit_heading_308": 0x740F80, "unit_facing_2a8": 0x746E30},
    )


def main(argv=None):
    finish_vectors(generate, Path(__file__).with_suffix(".json"),
                   provenance=metadata, argv=argv)


if __name__ == "__main__":
    main()
