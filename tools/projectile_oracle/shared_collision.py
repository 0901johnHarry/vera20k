"""Execute 468BB0 and all reached receivers without behavioral hooks.

Synthetic object/cell/house state is the input. Original 565730/5657A0/578080,
47B3A0, 4CC360, 486840/4867E0, 5F6B90/41B920/661F90, 410540/447AC0,
5F6360 and 4F9A50 execute against the retail image. The only code hook records
addresses and cell-query inputs; it never changes registers, memory or control.
This is a post-commit probe oracle, not an upstream trajectory oracle.
"""
import hashlib
import itertools
from pathlib import Path
from tools.native_oracle import NATIVE_FPCW, image_bytes, finish_vectors, provenance
from .collision_fixture import shared_probe


def generate():
    cases = []
    for z in (-417, -416, -415, -1, 0, 1):
        cases.append(dict(candidate=[640, 640, z]))
        for target_z in (-1, 0, 1):
            cases.append(dict(candidate=[640, 640, z], flak=True, target=dict(coord=[640, 640, target_z])))
    for tile in (99, 100, 113, 114, 65535):
        cases.append(dict(level=True, cells={'2,2': dict(tile=tile)}))
    for marked, height, distance in itertools.product((False, True), (207, 208, 209), (127, 128)):
        cases.append(dict(aa=True, candidate=[640 + distance, 640, height], target=dict(coord=[640, 640, height], marked=marked)))
    for phase in range(7):
        for dmisl in (False, True):
            cases.append(dict(aa=True, candidate=[640, 640, 0], target=dict(category='aircraft', rocket=True, dmisl=dmisl, phase=phase, marked=False, coord=[640, 640, 0])))
    for foundation, distance in itertools.product((0, 1, 4), (127, 255, 383)):
        cases.append(dict(aa=True, candidate=[640 + distance, 640, 300], target=dict(category='building', foundation=foundation, coord=[640, 640, 300])))
    for source_level, previous_level, candidate_level, flags in itertools.product((0, 4), (0, 1, 4), (3, 4, 5), (0, 128)):
        cases.append(dict(cliffs=True, cells={'0,0':dict(level=source_level), '1,2':dict(level=previous_level), '2,2':dict(level=candidate_level, flags=flags)}))
    for offset, source_allied, wall_allied, target_same in itertools.product(([0,0], [100,0], [0,100]), (False, True), (False, True), (False, True)):
        cases.append(dict(walls=True, transparency=True, source_allied=source_allied, wall_allied=wall_allied,
            candidate=[640+offset[0],640+offset[1],0], launch_target=[640 if target_same else 1408,640,0], cells={'2,2':dict(wall=True)}))
    for missing in ([[0,0]], [[5,2]], [[1,2]], [[2,2]], [[0,0],[5,2]], [[0,0],[5,2],[1,2],[2,2]]):
        cases.append(dict(walls=True, cliffs=True, missing=missing, candidate=[640,740,500], cells={'2,2':dict(wall=True,level=4)}))
    rows = [shared_probe(case) for case in cases]
    output = dict(sha256=hashlib.sha256(image_bytes()).hexdigest(), fpcw=NATIVE_FPCW, hooks='observation only', cases=rows)
    return output


def metadata():
    return provenance(
        scope='130 original Bullet468BB0 post-commit collision probes and reached receivers, including terrain/cliff/wall and aircraft/building cases. Not upstream trajectory, construction or downstream damage parity.',
        assumptions=[
            'collision_fixture supplies synthetic cell table, Bullet/type/source/target/house/object layouts, map dimensions, levels, overlays, alliance masks and Rules fields. Unspecified fixture memory is zero-filled.',
            'Live FPCW and cached control822D80 are both0E7F. Native receiver trace and cell queries are observed in execution order. Retail foundation dimensions come from the verified image.',
            'No assertion is made about whole-game RNG ordering, timers, detach effects or frame scheduling; this corpus observes only admission, coordinates, receiver visits and queries.',
        ],
        substitutions=['None; the code hook observes receiver visits and query inputs without changing registers, memory or control flow.'],
        entry_points=dict(probe=0x468BB0, cell_by_coord=0x565730, cell_by_index=0x5657A0,
                          ground=0x578080, cliff=0x47B3A0, aircraft=0x4CC360,
                          building_first=0x486840, building_second=0x4867E0,
                          height=0x5F6B90, building_coord=0x447AC0, alliance=0x4F9A50),
    )


def main(argv=None):
    finish_vectors(generate, Path(__file__).with_name('shared_collision_vectors.json'),
                   provenance=metadata, argv=argv)


if __name__ == '__main__':
    main()
