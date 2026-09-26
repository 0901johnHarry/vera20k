"""Original Tactical inverse after supplied live bridge flag transitions.

Reuses the unchanged 6D6590 fixture and real lookup/projector leaves. The flag
states below are declared inputs, not an execution of collapse/repair itself;
47E040's writer is independently covered by bridge_body_publication. Each
query reuses the same native CellClass addresses after the flag replacement.
"""
from pathlib import Path

from tools.bridge_click_oracle import NativeFixture
from tools.native_oracle import finish_vectors, provenance


def generate():
    rows = []
    points = [[0, 420], [-15, 420], [15, 420], [0, 435], [0, 480], [30, 420]]
    for direction, cells in [
        (0, [[16, 16], [16, 15], [16, 14], [16, 17]]),
        (6, [[16, 16], [15, 16], [14, 16], [17, 16]]),
    ]:
        fixture = NativeFixture([0, 0, 31, 31], 2, [], [])
        for phase, structural in [("intact", True), ("collapsed", False), ("repaired", True)]:
            flags = (0x100 if structural else 0x400) | (0x800 if direction == 0 else 0)
            for x, y in cells:
                fixture.put32(fixture.addresses[x, y] + 0x140, flags)
            rows.append({
                "direction": direction, "phase": phase, "flags": flags,
                "cells": cells,
                "cases": [{"world_input": point, **fixture.inverse(point, [0, 0], [0, 0])}
                          for point in points],
            })
    return {"source": "unicorn/gamemd.exe", "bounds": [0, 0, 31, 31],
            "base_height": 2, "anchor": [16, 16], "rows": rows}


def metadata():
    return provenance(
        scope="36 original6D6590 queries across supplied intact/collapsed/repaired live flags in both orientations",
        assumptions=[
            "Same binary, matrix/direction initialization and unchanged native leaves as tools/bridge_click_oracle.py.",
            "Synthetic32x32 flat ground level2; four supplied structural cells match anchor/F1/F2/opposite visitation for direction0 or6.",
            "Only relevant structural100, destroyed400 and direction800 inputs are supplied; flag publication is not executed here.",
            "Flag inputs are overwritten on the same CellClass identities between phases; no map or matrix reconstruction.",
            "All map lookups must resolve real mapped cells. No dummy/off-map behavior, UI delivery, movement or GPU proof.",
        ],
        substitutions=[], entry_points={"inverse": 0x6D6590, "map_lookup": 0x5657A0,
                                        "neighbor": 0x481810, "projector": 0x6D1F10},
    )


if __name__ == "__main__":
    finish_vectors(generate, Path(__file__).with_suffix("") / "vectors.json", provenance=metadata)
