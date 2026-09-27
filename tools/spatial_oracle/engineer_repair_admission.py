"""Original519948 effective-mission and ground-building target admission.

This is a supplied INTERIOR PerCellProcess frame, not full PerCell2 entry.
The upstream reason, spy/thief/transporter/zone/path/Walk admission and cached
physical-cell producer have NOT executed. Original Infantry Mission5B3040,
Building457620/465D40 and Cell47C520 execute unchanged; hooks only observe.
Stops before519B58 (Engineer/type/BridgeRepairHut and effects not executed),
or before the original rejection continuations51A071/51A0D4.
"""
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_ESP
from tools.native_oracle import load_image, run_checked, STACK_BASE, STACK_SIZE, SCRATCH, RET_MAGIC, finish_vectors, provenance
from tools.spatial_oracle.map_queries import dwords

ACTOR, HUT, OTHER, KIND, CELL = [SCRATCH + n * 0x2000 for n in range(1, 6)]
BEGIN, ADMITTED, NO_TARGET, NO_MISSION = 0x519948, 0x519B58, 0x51A071, 0x51A0D4
INF_VTABLE, BUILDING_VTABLE = 0x7EB058, 0x7E3EBC


def execute(case):
    u = Uc(UC_ARCH_X86, UC_MODE_32)
    load_image(u)
    u.mem_map(STACK_BASE, STACK_SIZE)
    u.mem_map(SCRATCH, 0x10000)
    u.mem_map(RET_MAGIC, 0x1000)
    sp = STACK_BASE + STACK_SIZE - 0x1000
    identities = {0: None, ACTOR: 'infantry', HUT: 'hut', OTHER: 'other', CELL: 'cached_cell'}
    pointers = {None: 0, 'hut': HUT, 'other': OTHER}

    def read32(address):
        return struct.unpack('<I', u.mem_read(address, 4))[0]

    def signed_value(value):
        return value if value < 0x80000000 else value - 0x100000000

    assert read32(INF_VTABLE + 0x184) == 0x5B3040
    assert read32(BUILDING_VTABLE + 0x80) == 0x457620
    building_what_am_i = read32(BUILDING_VTABLE + 0x2C)
    u.mem_write(ACTOR, dwords(INF_VTABLE))
    u.mem_write(ACTOR + 0xAC, dwords(case['current']))
    u.mem_write(ACTOR + 0xB4, dwords(case['queued']))
    u.mem_write(ACTOR + 0x5A4, dwords(pointers[case['nav_com']]))
    u.mem_write(ACTOR + 0x2B4, dwords(pointers[case['attack_target']]))
    for address in (HUT, OTHER):
        u.mem_write(address, dwords(BUILDING_VTABLE))
        # Abstract+14 bit1 is Techno RTTI, not Object cell-marked state.
        u.mem_write(address + 0x14, dwords(1))
        u.mem_write(address + 0x520, dwords(KIND))
        u.mem_write(address + 0x34, dwords(0))  # No attached Tag.
    u.mem_write(KIND + 0x408, dwords(0))  # UndeploysInto=null, ordinary branch.
    u.mem_write(0xA8E9A0, bytes([case.get('object_iteration_enabled', True)]))
    ground = case.get('ground_list', ['hut'])
    for index, name in enumerate(ground):
        following = ground[index + 1] if index + 1 < len(ground) else None
        u.mem_write(pointers[name] + 0x30, dwords(pointers[following]))
    u.mem_write(CELL + 0xE4, dwords(pointers[ground[0]] if ground else 0))
    # Explicit upper-only control; the original47C520 reads ground+E4.
    u.mem_write(CELL + 0xE8, dwords(pointers[case.get('upper_head')]))
    u.mem_write(sp + 0x14, dwords(CELL))
    state_before = bytes(u.mem_read(ACTOR, 0x800))
    spans = [(BEGIN, 0x519B58), (0x5B3040, 0x5B3052),
             (0x457620, 0x45762B), (0x465D40, 0x465D6E),
             (0x47C520, 0x47C54F), (building_what_am_i, building_what_am_i + 6)]
    code = [bytes(u.mem_read(a, b - a)) for a, b in spans]
    trace = []

    def observe(_u, address, _size, _data):
        if address == 0x5B3040:
            assert u.reg_read(UC_X86_REG_ECX) == ACTOR
        elif address in (0x519952, 0x519961, 0x519970):
            trace.append(dict(kind='effective_mission', comparison={0x519952: 8, 0x519961: 11, 0x519970: 25}[address],
                              value=signed_value(u.reg_read(UC_X86_REG_EAX))))
        elif address == 0x457620:
            trace.append(dict(kind='nav_1x1_undeploy_query', target=identities[u.reg_read(UC_X86_REG_ECX)]))
        elif address == 0x51999E:
            trace.append(dict(kind='nav_1x1_undeploy_result', value=u.reg_read(UC_X86_REG_EAX) & 255))
        elif address == 0x47C520:
            assert u.reg_read(UC_X86_REG_ECX) == CELL
            trace.append(dict(kind='first_ground_building_query', cell='cached_cell'))
        elif address == building_what_am_i:
            trace.append(dict(kind='what_am_i', object=identities[u.reg_read(UC_X86_REG_ECX)]))
        elif address == 0x519B20:
            trace.append(dict(kind='first_ground_building_result', object=identities[u.reg_read(UC_X86_REG_EAX)]))
        elif address in (0x6E53A0, 0x65C780, 0x65C7E0, 0x570050, 0x573540):
            raise AssertionError('interior gate unexpectedly entered a side-effect receiver')

    u.hook_add(UC_HOOK_CODE, observe)
    u.reg_write(UC_X86_REG_ESP, sp)
    u.reg_write(UC_X86_REG_ESI, ACTOR)
    end = run_checked(u, BEGIN, (ADMITTED, NO_TARGET, NO_MISSION), count=5000,
                      required_addresses=(BEGIN, 0x5B3040))
    assert [bytes(u.mem_read(a, b - a)) for a, b in spans] == code
    assert bytes(u.mem_read(ACTOR, 0x800)) == state_before
    assert u.reg_read(UC_X86_REG_ESP) == sp
    return dict(input=case, output=dict(
        boundary=f'{end:08X}',
        outcome={ADMITTED: 'admitted_to_engineer_type_gate', NO_TARGET: 'no_matching_first_ground_building', NO_MISSION: 'mission_not_admitted'}[end],
        retained_building=identities[u.reg_read(UC_X86_REG_EDI)],
        trace=trace))


def inputs():
    rows = []
    for current, queued in [(5, 8), (8, 5), (8, -1), (8, 25),
                            (-1, 8), (-1, 11), (-1, 25), (-1, 5), (-1, -1),
                            (11, 5), (25, 5), (5, -1)]:
        rows.append(dict(name=f'mission_current_{current}_queued_{queued}', current=current, queued=queued,
                         nav_com='hut', attack_target=None))
    for nav_com, attack_target, name in [
        (None, 'hut', 'attack_only'), ('other', 'hut', 'other_nav_matching_attack'),
        ('hut', 'other', 'matching_nav_other_attack'),
        ('other', 'other', 'neither_matches'), (None, None, 'no_targets'),
    ]:
        rows.append(dict(name=name, current=8, queued=-1, nav_com=nav_com, attack_target=attack_target))
    for extra in [
        dict(name='empty_ground', ground_list=[]),
        dict(name='upper_only_hut', ground_list=[], upper_head='hut'),
        dict(name='different_first_building', ground_list=['other', 'hut']),
        dict(name='matching_first_building', ground_list=['hut', 'other']),
        dict(name='object_iteration_disabled', object_iteration_enabled=False),
    ]:
        rows.append(dict(current=8, queued=-1, nav_com='hut', attack_target=None, **extra))
    return rows


def generate():
    return [execute(case) for case in inputs()]


if __name__ == '__main__':
    finish_vectors(generate, Path(__file__).with_suffix('.json'), provenance=lambda: provenance(
        scope=__doc__,
        entry_points={'supplied_interior_mission_gate': BEGIN, 'effective_mission': 0x5B3040,
                      'building_undeploy_predicate': 0x457620, 'type_undeploy_predicate': 0x465D40,
                      'first_ground_building': 0x47C520,
                      'engineer_type_gate_not_executed': ADMITTED,
                      'target_rejection_continuation_not_executed': NO_TARGET,
                      'mission_rejection_continuation_not_executed': NO_MISSION},
        assumptions=['ESI Infantry and ESP local frame already established; ESP+14 supplied cached physical CellClass.',
                     'Original Infantry/Building vtables; Techno RTTI bit1 set on Building objects; null UndeploysInto and attached Tags.',
                     'Supplied linked ground/upper objects and active-object lookup global; constructors and map loader omitted.',
                     'Mission current+AC, queued+B4, NavCom+5A4 and attack target+2B4 supplied independently.'],
        substitutions=['No instruction patches or native call answers. Observation-only hooks. All paths stop before suffix side effects.',
                       'This does not validate full519630 PerCell2 admission, Walk entry, Engineer/type flags, notification, repair or consumption.'],
    ))
