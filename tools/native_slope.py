"""Checked retail startup slope matrices shared by native fixtures.

The pinned image supplies the original CRT/control-word and matrix initializer
bodies. A reset-style FPCW, scratch stack and return sentinel are supplied; this
does not execute complete Windows startup. No behavioral hooks or code patches
are installed. Each call completes before the next initializer uses its state.
"""

import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_FPCW

from tools.native_oracle import load_image, run_checked


_BASE, _SP, _STOP = 0x20000000, 0x2000E000, 0x30000000


def slope_matrices():
    """Return the 21 native matrices as fresh rows of twelve raw f32 words.

    Preserve one machine across the original initializer chronology, including
    the control-word cache written by 7C5EE4. In particular, 755852..755875 writes
    identity into rows 17..20; do not reconstruct those rows from other slopes.
    """
    uc = Uc(UC_ARCH_X86, UC_MODE_32)
    load_image(uc)
    uc.mem_map(_BASE, 0x10000)
    uc.mem_map(_STOP, 0x1000)
    uc.reg_write(UC_X86_REG_FPCW, 0x037F)

    def run(address, arguments=()):
        uc.mem_write(
            _SP,
            struct.pack('<' + 'I' * (len(arguments) + 1), _STOP, *arguments),
        )
        uc.reg_write(UC_X86_REG_ESP, _SP)
        run_checked(uc, address, _STOP, count=10_000_000,
                    required_addresses=(address,))

    run(0x007CEAAF)
    run(0x007CBF49, (0x300, 0x300))
    run(0x007C5EE4)
    for address in (
        0x754910, 0x7549A0, 0x7549C0, 0x7549E0,
        0x754A20, 0x754A50, 0x754CB0,
    ):
        run(address)
    return [
        list(struct.unpack('<12I', uc.mem_read(0xB45188 + 48 * slope, 48)))
        for slope in range(21)
    ]
