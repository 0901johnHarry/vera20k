"""Original retained Terrain coordinate -> projection -> body/shadow draw calls.

Run with --check (default) or --write. The original Terrain Render suffix
71CD22..71CD81 executes after its visibility and rectangle-overlap admission.
Only CC_Draw_Shape is replaced by a recorded draw sink; no pixels are drawn.
"""
import struct
from pathlib import Path

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import (
    UC_X86_REG_EAX, UC_X86_REG_EDI, UC_X86_REG_ESI, UC_X86_REG_ESP,
    UC_X86_REG_EIP,
)
from tools.native_oracle import finish_vectors, provenance, run_checked
from tools.projectile_oracle.ordinary_collision import slope_matrices
from tools.spatial_oracle.bridge_damage_admission import (
    CELL, MEM, SP, base, call, read32, words,
)
from tools.spatial_oracle.terrain_coordinate import generate as coordinate_vectors


def signed_words(u, address, count):
    return list(struct.unpack('<' + 'i' * count, u.mem_read(address, count * 4)))


def execute(case, viewport_y, dirty_y):
    u = base(case)
    u.mem_write(0x89E7C0, words(104))
    u.mem_write(0xB0CD48, struct.pack('<Q', 0x3FC25E5374344960))
    for index, matrix in enumerate(slope_matrices()):
        u.mem_write(0xB45188 + 48 * index, struct.pack('<12I', *matrix))
    source, adjusted, obj, observed, typ, shp, tactical, clip = (
        MEM + offset for offset in
        (0x18000, 0x18020, 0x18100, 0x18300, 0x19000, 0x19400, 0x1A000, 0x1C000)
    )
    u.mem_write(source, words(2688, 5248, case['input_z']))
    call(u, 0x71E0D0, args=(adjusted, source))
    u.mem_write(obj, words(0x7F522C))
    u.mem_write(obj + 0xC8, words(typ))
    u.mem_write(obj + 0x6C, words(200))
    u.mem_write(typ, words(0x7F5458))
    u.mem_write(typ + 0xA4, words(shp))
    u.mem_write(shp + 6, struct.pack('<h', 4))
    call(u, 0x5F6940, obj, (adjusted,))
    placement = signed_words(u, adjusted, 3)
    # Change the original cell after construction. Rendering reads retained XYZ.
    u.mem_write(CELL + 0x11B, bytes((7, 0)))
    call(u, 0x5F65A0, obj, (observed,))
    retained = signed_words(u, observed, 3)
    u.mem_write(CELL + 0x34, words(MEM + 0x1D000))
    u.mem_write(CELL + 0x10A, struct.pack('<hh', 1000, 1000))
    u.mem_write(0x822CF1, b'\x01')
    u.mem_write(0x887324, words(tactical))
    u.mem_write(tactical + 0xB0, words(0, 0))
    u.mem_write(0xB0CE30, words(800, 600))
    u.mem_write(0x886FA0, words(0, viewport_y, 800, 600))
    u.mem_write(clip, words(0, dirty_y, 800, 600))
    # Explicit caller-local state at the suffix entry, after admission.
    u.mem_write(SP - 128, bytes(256))
    u.reg_write(UC_X86_REG_ESP, SP)
    u.reg_write(UC_X86_REG_EDI, obj)
    u.reg_write(UC_X86_REG_ESI, clip)
    draws, projection, height = [], [], []

    def observe(_uc, address, _size, _data):
        if address == 0x71CD42:
            projection.extend(signed_words(u, SP + 0x10, 2))
        if address == 0x71C256:
            height.append(struct.unpack('<i', words(u.reg_read(UC_X86_REG_EAX)))[0])
        if address != 0x4AED70:
            return
        sp = u.reg_read(UC_X86_REG_ESP)
        raw = bytes(u.mem_read(sp + 4, 56))
        args = list(struct.unpack('<14i', raw))
        draws.append(dict(
            caller=f'{read32(u, sp):08X}', frame=args[1],
            point=signed_words(u, args[2], 2), flags=args[4],
            z_adjust=args[6], gradient=args[7], brightness=args[8],
            raw_args_hex=raw.hex(),
            point_bytes=bytes(u.mem_read(args[2], 8)).hex(),
        ))
        u.reg_write(UC_X86_REG_EAX, 0)
        u.reg_write(UC_X86_REG_EIP, read32(u, sp))
        u.reg_write(UC_X86_REG_ESP, sp + 60)

    u.hook_add(UC_HOOK_CODE, observe)
    run_checked(u, 0x71CD22, 0x71CD81, required_addresses=(
        0x71CD30, 0x41BE00, 0x5F65A0, 0x71CD3D, 0x6D2140,
        0x71CD7B, 0x71C1B0, 0x5F5F30, 0x6D20E0, 0x71C304, 0x71C34E,
    ))
    assert retained == placement and len(draws) == 2 and len(height) == 1
    return dict(
        input=case, viewport_y=viewport_y, dirty_y=dirty_y,
        placement=placement, retained=retained,
        retained_bytes=bytes(u.mem_read(observed, 12)).hex(),
        projected_point=projection, projection_bytes=words(*projection).hex(),
        lift_px=height[0], draws=draws,
    )


def generate():
    # Share the tracked native coordinate corpus's input enumeration.
    cases = [row['input'] for row in coordinate_vectors()['cases']]
    return dict(rows=[execute(case, viewport, dirty) for case in cases
                      for viewport, dirty in ((0, 0), (37, 100))])


def metadata():
    return provenance(
        scope='82 original Terrain render captures: 41 tracked terrain_coordinate inputs crossed with two supplied viewport/dirty pairs. Original type adjustment, Object coordinate storage, Render suffix projection/rebasing and DrawIt body/shadow arguments execute. No visibility admission, shape decoding, GPU pixels or complete scene parity.',
        assumptions=[
            'Input enumeration comes from tracked terrain_coordinate.generate: levels -128,-1,0,2,127; slopes0,1,2,15; structural flags0/0x100; inputZ0, plus level2/inputZ999. All sourceXY2688,5248 is cell10,20 center. Original71E0D0 clamps placement to ground, then5F6940 stores it. Cell changes to level7/flat before native retained getter and rendering.',
            'Original vtables: Terrain7F522C+48=5F65A0,+AC=41BE00,+104=71CC50,+114=71C1B0,+1D0=5F5F30; TerrainType7F5458+9C=41CFA0. Render suffix71CD22..71CD81 bypasses prior visibility and render-rectangle admission, supplies EDI=Terrain and ESI=clip. Native71CD30 calls center/getter,71CD3D projects,71CD7B invokes DrawIt.',
            'Ordinary nonanimated, non-SpawnsTiberium Terrain with health200, death byte0; synthetic SHP frame count4 and no decoded image; supplied nonnull Cell Convert and both brightness fields1000; shadow enable byte1. No constructors, retail type/image binding or gameplay state transitions claimed here.',
            'LevelHeight104 and startup AdjustForZ multiplier bits0x3FC25E5374344960 supplied; slope matrices produced by original initializer. Native x87 control0x0E7F. Tactical camera offsets0,0 and dimensions800,600; viewport/dirtyY pairs0/0 and37/100. Native pixels exclude VERA world-row bias15.',
        ],
        substitutions=[
            'CC_Draw_Shape4AED70 records original caller, all14 raw stack arguments and pointed coordinate bytes; supplies return0 and callee cleanup56. No actual shape decode, blitter, surface, pixels or Z-buffer write. All other reached native calls execute unchanged.',
            'Prepared map/cell/object/type/shape/tactical storage and caller locals are fixture inputs. Original executable code is not patched. Visibility admission and image/resource loading precede the bounded entry and are not emulated.',
        ],
        entry_points={
            'type_coordinate': 0x71E0D0, 'set_coords': 0x5F6940,
            'get_coords': 0x5F65A0, 'terrain_render': 0x71CC50,
            'render_suffix_begin': 0x71CD22, 'render_suffix_end': 0x71CD81,
            'projection': 0x6D2140, 'terrain_draw': 0x71C1B0,
            'get_height': 0x5F5F30, 'adjust_for_z': 0x6D20E0,
            'draw_sink': 0x4AED70,
        },
    )


if __name__ == '__main__':
    finish_vectors(generate, Path(__file__).with_suffix('.json'), provenance=metadata)
