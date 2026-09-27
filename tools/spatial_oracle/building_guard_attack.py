"""Original building Guard and Attack missions.

- `guard` rows: one BuildingClass::Mission_Guard (0x4496B0) call on the
  building_construction fixture's refinery (the slave_manager fixture's 2x2
  building), armed by a WeaponType pointer in Weapon[0] (Type+0x898, the slot
  TechnoClass::GetWeapon 0x70E140 reads for a non-elite techno) or unarmed,
  with the row's mission (+0xAC), status (+0xBC), TarCom (+0x2B4), type flags
  (EMPulseCannon +0x16C3, HasStupidGuardMode +0x16B5, UnitRepair +0x16A9,
  WeaponsFactory +0x16BD) and MissionControl Rate/AARate (0xA8E3A8 +
  mission * 0x20 + 0x10/+0x18) stored as MissionControlClass::Read_INI's
  CCINIClass::ReadDouble leaves them (`%f` into a float, widened). Begin_Mode
  (0x447780), Queue_Mission (0x5B35E0), Commence (0x5B3570) and the Scenario
  RandomRanged (0x65C7E0) run natively; ClearBibArea (0x449540) is observed.
- `attack` rows: one BuildingClass::Mission_Attack (0x44ACF0) call with
  BuildingClass::GetFireError (0x447F10) answered from the row, each code
  0..11 and a null TarCom; IsAnimDelayedFire (+0x16A7) with DelayedFireDelay
  (+0x16EC) for the delayed arm.
- `set_target` rows: BuildingClass::SetTarget (0x443B90) with Is_Operational
  (0x4555D0), IsCloseEnough (vt+0x3AC 0x6F7780), IsHumanPlayer (0x50B730) and
  the EMP test (vt+0x37C 0x70EFD0) answered from the row; slot 0's projectile
  AA flag (BulletType+0x2A4) supplied.
- `unlimbo` rows: BuildingClass 0x44D6A0 (Unlimbo's vt+0x484) with the row's
  arguments, Scenario-init flag (0xA8E7AC) and 0xA8ED6B.
- `cadence` rows: an armed building's frames through BuildingClass::Update's
  mission pieces in its order, each native: the ready check that commences a
  queued mission unless BState is 0 (0x43FE27..0x43FE54), MissionClass::AI
  (0x5B3060, dispatching Mission_Guard and Mission_Attack) and the ready check
  that commences (0x43FF91..0x43FFB4). Fire_At (0x6FDD50) is answered with its
  rearm write (+0x2EC = {Frame, _, ROF}, 0x6FE4A4..0x6FE4C5) at the row's
  ROF, and GetFireError with TechnoClass::GetFireError's rearm test
  (0x6FC94F..0x6FC975: REARM while the timer has time left, else OK). Row
  events set or clear TarCom (the passive scan's or a pointer detach's write)
  or queue Attack as a player order (Queue_Mission(Attack, false)).

Usage: python -m tools.spatial_oracle.building_guard_attack [--check|--write]
"""
from pathlib import Path
import struct

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_ESP
from tools.native_oracle import finish_vectors, provenance
from tools.spatial_oracle import building_construction as bc
from tools.spatial_oracle import slave_manager as sm
from tools.spatial_oracle.map_queries import dwords
from tools.spatial_oracle.refinery_dock import ACTOR
from tools.spatial_oracle.unit_source_scatter import SCENARIO

MISSION_GUARD, MISSION_ATTACK, BUILDING_SET_TARGET = 0x4496B0, 0x44ACF0, 0x443B90
GET_FIRE_ERROR, FIRE_AT, TECHNO_SET_TARGET = 0x447F10, 0x6FDD50, 0x6FCDB0
SET_DESIRED, DIRECTION_TO, UNCLOAK = 0x4C9220, 0x43ED40, 0x7036C0
PLAY_ANIM, CLEAR_BIB = 0x451890, 0x449540
IS_OPERATIONAL, IS_CLOSE_ENOUGH, IS_HUMAN, UNDER_EMP = 0x4555D0, 0x6F7780, 0x50B730, 0x70EFD0
RANDOM_RANGED, RANDOM_NEXT, RANDOM_SEED = 0x65C7E0, 0x65C780, 0x65C6D0
ENTER_CONSTRUCTION = bc.ENTER_CONSTRUCTION
# Weapon[0] (TechnoTypeClass +0x898, WeaponStruct stride 0x1C) and a supplied
# WeaponType/BulletType pair in the fixture's free region.
WEAPON_SLOT0 = sm.YTYPE + 0x898
WEAPON, BULLET = sm.REGION + 0x9000, sm.REGION + 0xA000
MISSION = {'none': -1, 'attack': 1, 'guard': 5, 'sticky': 6, 'selling': 0x13, 'wait': 0x1C}
MISSION_NAME = {value: name for name, value in MISSION.items()}
# Retail [Guard] Rate=.030 AARate=.016, [Sticky] Rate=.016 (no AARate: the
# reader copies Rate), [Attack] Rate=.016 AARate=.016.
RATES = {'guard': ('.030', '.016'), 'sticky': ('.016', '.016'), 'attack': ('.016', '.016')}
TYPE_FLAGS = {'emp_cannon': 0x16C3, 'stupid_guard': 0x16B5, 'unit_repair': 0x16A9,
              'weapons_factory': 0x16BD, 'delayed_fire': 0x16A7, 'tick_tank': 0x16C4,
              'artillary': 0x16CA}


def read_double(text):
    """CCINIClass::ReadDouble (0x5283D0): `%f` into a float, widened."""
    return struct.unpack('<f', struct.pack('<f', float(text)))[0]


def name_of(pointer):
    return {ACTOR: 'target', 0: None}.get(pointer, hex(pointer))


def signed(u, address):
    return struct.unpack('<i', u.mem_read(address, 4))[0]


def fixture(case):
    u, read32, building, frame, calls = bc.building_fixture(dict(name=case['name'], control=[0, 1, 0],
                                                                 mission='none'))
    kind = sm.YTYPE
    if 'seed' in case:
        bc.invoke(u, RANDOM_SEED, SCENARIO + 0x218, case['seed'])
    u.mem_write(building + 0xAC, dwords(MISSION[case.get('mission', 'guard')]))
    u.mem_write(building + 0xB4, dwords(MISSION[case.get('queued', 'none')]))
    u.mem_write(building + 0xBC, dwords(case.get('status', 0)))
    u.mem_write(building + 0xC0, dwords(frame, 0))
    u.mem_write(building + 0xC8, dwords(frame, 0, 0))
    u.mem_write(building + 0x90, bytes([1]))
    u.mem_write(building + 0x2B4, dwords(ACTOR if case.get('target') else 0))
    u.mem_write(building + 0x6DD, bytes([case.get('ready', 0)]))
    u.mem_write(WEAPON_SLOT0, dwords(WEAPON if case.get('armed', True) else 0))
    u.mem_write(WEAPON + 0xA0, dwords(BULLET))
    u.mem_write(BULLET + 0x2A4, bytes([case.get('anti_air', False)]))
    for flag, offset in TYPE_FLAGS.items():
        u.mem_write(kind + offset, bytes([case.get(flag, False)]))
    u.mem_write(kind + 0x16EC, dwords(case.get('delayed_fire_delay', 0)))
    u.mem_write(kind + 0x16F0, dwords(-1))  # SuperWeapon= unset
    for mission, (rate, aa_rate) in {**RATES, **case.get('rates', {})}.items():
        entry = 0xA8E3A8 + MISSION[mission] * 0x20
        u.mem_write(entry + 0x10, struct.pack('<dd', read_double(rate), read_double(aa_rate)))
    events = []
    answers = dict(errors=list(case.get('errors', [])), rof=case.get('rof'))

    def ret(cleanup, value=0):
        sp = u.reg_read(UC_X86_REG_ESP)
        u.reg_write(UC_X86_REG_EAX, value & 0xFFFFFFFF)
        u.reg_write(UC_X86_REG_EIP, read32(sp))
        u.reg_write(UC_X86_REG_ESP, sp + 4 + cleanup)

    def rearm_left():
        start, delay = signed(u, building + 0x2EC), signed(u, building + 0x2F4)
        if start == -1:
            return delay
        elapsed = (read32(bc.FRAME) - start) & 0xFFFFFFFF
        return 0 if elapsed >= delay else delay - elapsed

    def hook(_u, address, _size, _data):
        sp = u.reg_read(UC_X86_REG_ESP)
        arg = lambda n: read32(sp + 4 * n)
        if address == GET_FIRE_ERROR:
            if answers['rof'] is not None:
                code = 3 if rearm_left() else 0
            else:
                code = answers['errors'].pop(0)
            events.append(['fire_error', name_of(arg(1)), arg(2), arg(3), code])
            ret(12, code)
        elif address == FIRE_AT:
            events.append(['fire_at', name_of(arg(1)), arg(2)])
            if answers['rof'] is not None:
                u.mem_write(building + 0x2EC, dwords(read32(bc.FRAME), 0, answers['rof']))
            ret(8, 0)
        elif address == SET_DESIRED:
            events.append(['set_desired'])
            ret(4)
        elif address == DIRECTION_TO:
            u.mem_write(arg(1), dwords(0))
            ret(8, arg(1))
        elif address == TECHNO_SET_TARGET:
            events.append(['set_target', name_of(arg(1))])
            u.mem_write(building + 0x2B4, dwords(arg(1)))
            ret(4)
        elif address == UNCLOAK:
            events.append(['uncloak', arg(1)])
            ret(4)
        elif address == PLAY_ANIM:
            events.append(['play_anim'])
            ret(20)
        elif address == CLEAR_BIB:
            events.append(['clear_bib'])
            ret(0)
        elif address == IS_OPERATIONAL:
            ret(0, case.get('operational', True))
        elif address == IS_CLOSE_ENOUGH:
            events.append(['in_range', name_of(arg(1))])
            ret(4, case.get('in_range', True))
        elif address == IS_HUMAN:
            ret(0, case.get('human', False))
        elif address == UNDER_EMP:
            ret(0, case.get('under_emp', False))
        elif address == RANDOM_RANGED:
            events.append(['random_ranged', arg(1), arg(2)])
        elif address == RANDOM_NEXT:
            events.append(['random_next'])
        elif address == MISSION_GUARD:
            events.append(['mission_guard'])
        elif address == MISSION_ATTACK:
            events.append(['mission_attack'])

    u.hook_add(UC_HOOK_CODE, hook)
    return u, read32, building, frame, events


def state(u, building):
    return dict(ready=u.mem_read(building + 0x6DD, 1)[0],
                mission=MISSION_NAME.get(signed(u, building + 0xAC), signed(u, building + 0xAC)),
                queued=MISSION_NAME.get(signed(u, building + 0xB4), signed(u, building + 0xB4)),
                status=signed(u, building + 0xBC), mission_start=signed(u, building + 0xC0),
                counter=signed(u, building + 0xC4),
                timer=[signed(u, building + 0xC8), signed(u, building + 0xD0)],
                target=name_of(struct.unpack('<I', u.mem_read(building + 0x2B4, 4))[0]),
                bstate=signed(u, building + 0x534), queued_bstate=signed(u, building + 0x538),
                delayed_fire=[signed(u, building + 0x704), signed(u, building + 0x708),
                              signed(u, building + 0x714)],
                prism_count=signed(u, building + 0x664), turret_count=signed(u, building + 0x148))


def draws(events):
    return [event for event in events if event[0] in ('random_ranged', 'random_next')]


def one_call(entry):
    def run(case):
        u, read32, building, frame, events = fixture(case)
        before = [read32(SCENARIO + 0x21C), read32(SCENARIO + 0x220)]
        value = bc.invoke(u, entry, building, *case.get('args', []))
        after = [read32(SCENARIO + 0x21C), read32(SCENARIO + 0x220)]
        calls = [event for event in events if event[0] not in ('random_ranged', 'random_next')]
        return dict(input=case, frame=frame, returns=struct.unpack('<i', dwords(value))[0], calls=calls,
                    draws=draws(events), random_indices=dict(before=before, after=after), **state(u, building))
    return run


def guard_cases():
    return [
        # Armed: the AARate delay without a target, the Attack commit with one.
        # Seeds 1, 2 and 8 draw 1, 2 and 0; seed 9 re-draws (raw & 3 == 3).
        *[dict(name=f'armed_idle_seed{seed}', mission='guard', seed=seed) for seed in (1, 2, 8, 9)],
        dict(name='armed_target', mission='guard', target=True),
        dict(name='armed_sticky_idle', mission='sticky'),
        dict(name='armed_sticky_target', mission='sticky', target=True),
        dict(name='armed_emp_cannon', mission='guard', target=True, emp_cannon=True),
        # A modded AARate whose x900 has a fraction above .5 (ftol chops).
        dict(name='armed_modded_aarate', mission='guard', rates={'guard': ('.030', '.0206')}),
        # Unarmed: status 0 enters the idle mode; status 1 and later.
        dict(name='unarmed_status0', mission='guard', armed=False, status=0),
        dict(name='unarmed_status1', mission='guard', armed=False, status=1),
        dict(name='unarmed_status2', mission='guard', armed=False, status=2),
        dict(name='unarmed_repair_status1', mission='guard', armed=False, status=1, unit_repair=True),
        dict(name='unarmed_factory_status1', mission='guard', armed=False, status=1, weapons_factory=True),
        dict(name='unarmed_factory_status0', mission='guard', armed=False, status=0, weapons_factory=True),
        dict(name='unarmed_stupid', mission='guard', armed=False, stupid_guard=True),
        dict(name='unarmed_sticky', mission='sticky', armed=False, status=1),
    ]


def attack_cases():
    cases = [dict(name=f'code_{code}', mission='attack', target=True, errors=[code]) for code in range(12)]
    cases += [
        dict(name='no_target', mission='attack'),
        dict(name='delayed_ok', mission='attack', target=True, errors=[0], delayed_fire=True,
             delayed_fire_delay=28),
        dict(name='delayed_rearm', mission='attack', target=True, errors=[3], delayed_fire=True,
             delayed_fire_delay=28),
    ]
    return cases


def set_target_cases():
    return [
        dict(name='null', mission='guard', args=[0]),
        dict(name='in_range', mission='guard', args=[ACTOR], in_range=True),
        dict(name='out_of_range', mission='guard', args=[ACTOR], in_range=False),
        dict(name='no_weapon', mission='guard', args=[ACTOR], in_range=False, armed=False),
        dict(name='anti_air', mission='guard', args=[ACTOR], in_range=False, anti_air=True),
        dict(name='selling', mission='selling', args=[ACTOR], in_range=True),
        dict(name='not_operational', mission='guard', args=[ACTOR], in_range=True, operational=False),
        dict(name='artillery_out_of_range', mission='guard', args=[ACTOR], in_range=False, artillary=True),
        dict(name='artillery_human', mission='guard', args=[ACTOR], in_range=False, artillary=True, human=True),
        dict(name='artillery_emp', mission='guard', args=[ACTOR], in_range=False, artillary=True,
             under_emp=True),
    ]


def unlimbo(case):
    u, read32, building, frame, events = fixture(dict(case, mission='none'))
    u.mem_write(bc.SCENARIO_INIT, dwords(case['scenario_init']))
    u.mem_write(bc.SCENARIO_FLAG_ED6B, bytes([case['flag_ed6b']]))
    bc.invoke(u, ENTER_CONSTRUCTION, building, *case['args'])
    return dict(input=case, **state(u, building))


def unlimbo_cases():
    return [dict(name=f'args{a}{b}_init{i}_ed6b{f}', args=[a, b], scenario_init=i, flag_ed6b=f)
            for a, b in ((1, 1), (0, 1), (1, 0)) for i in (0, 1) for f in (0, 1)]


def cadence(case):
    """Per frame: the pre-AI commence block, MissionClass::AI and the post-AI
    commence block (module doc), with the row's events applied at the start
    of their frame."""
    u, read32, building, frame, events = fixture(case)
    bc.invoke(u, bc.BEGIN_MODE, building, 1)
    u.mem_write(building + 0x2EC, dwords(-1, 0, 0))
    if case['start'] == 'map':
        # Unlimbo during scenario init queues Guard; Grand_Opening's first
        # opening sets +0x6DD (0x4467C9).
        u.mem_write(bc.SCENARIO_INIT, dwords(1))
        u.mem_write(building + 0xAC, dwords(-1))
        bc.invoke(u, ENTER_CONSTRUCTION, building, 1, 1)
        u.mem_write(building + 0x6DD, bytes([1]))
        u.mem_write(bc.SCENARIO_INIT, dwords(0))
    script = {event[0]: event[1:] for event in case.get('events', [])}
    frames = []
    for k in range(1, case['frames'] + 1):
        u.mem_write(bc.FRAME, dwords(frame + k))
        for action in script.get(k, []):
            if action == 'acquire':
                u.mem_write(building + 0x2B4, dwords(ACTOR))
            elif action == 'lose':
                u.mem_write(building + 0x2B4, dwords(0))
            elif action == 'order':
                u.mem_write(building + 0x2B4, dwords(ACTOR))
                bc.invoke(u, bc.QUEUE_MISSION, building, 1, 0)
        before = len(events)
        bc.run_block(u, building, bc.READY_COMMENCE_UNLESS_BUILDING)
        bc.invoke(u, bc.MISSION_AI, building)
        bc.run_block(u, building, bc.READY_COMMENCE)
        frames.append(dict(frame=k, events=events[before:], **state(u, building)))
    return dict(input=case, frames=frames)


def cadence_cases():
    return [
        # A map building: Guard queued at unlimbo, +0x6DD from the first
        # opening; a target acquired at frame 5 (after that frame's
        # dispatch in native order is modelled by the next frame's read).
        dict(name='map_acquire_rof20', start='map', frames=70, rof=20, events=[[5, 'acquire']]),
        dict(name='map_acquire_rof21', start='map', frames=70, rof=21, events=[[5, 'acquire']]),
        dict(name='map_acquire_rof1', start='map', frames=24, rof=1, events=[[5, 'acquire']]),
        # The target is lost mid-reload: Attack's null-target tail, then Guard.
        dict(name='lose_target', start='map', frames=60, rof=20, events=[[5, 'acquire'], [30, 'lose']]),
        # A player order on an idle armed building: Queue(Attack) with the
        # target, promoted by the next ready check.
        dict(name='player_order', start='map', frames=40, rof=20, events=[[20, 'order']]),
    ]


def generate():
    return {'source': 'unicorn/gamemd.exe',
            'guard': [one_call(MISSION_GUARD)(case) for case in guard_cases()],
            'attack': [one_call(MISSION_ATTACK)(case) for case in attack_cases()],
            'set_target': [one_call(BUILDING_SET_TARGET)(case) for case in set_target_cases()],
            'unlimbo': [unlimbo(case) for case in unlimbo_cases()],
            'cadence': [cadence(case) for case in cadence_cases()]}


def main(argv=None):
    finish_vectors(
        generate, Path(__file__).with_suffix('.json'),
        provenance=lambda: provenance(
            scope='BuildingClass::Mission_Guard 0x4496B0, BuildingClass::Mission_Attack 0x44ACF0, '
                  'BuildingClass::SetTarget 0x443B90, BuildingClass 0x44D6A0 and their dispatch by '
                  'MissionClass::AI 0x5B3060 between BuildingClass::Update\'s ready checks',
            entry_points={'mission_guard': MISSION_GUARD, 'mission_attack': MISSION_ATTACK,
                          'set_target': BUILDING_SET_TARGET, 'enter_construction': ENTER_CONSTRUCTION,
                          'mission_ai': bc.MISSION_AI, 'update_ready_commence_unless_building': 0x43FE27,
                          'update_ready_commence': 0x43FF91},
            assumptions=['the building_construction fixture refinery (slave_manager fixture, Building '
                         'vtables), armed through Weapon[0] = a supplied WeaponType whose Projectile is a '
                         'supplied BulletType; TarCom is the fixture\'s unit',
                         'MissionControl Rate/AARate written as float-widened doubles (ReadDouble '
                         '0x5283D0); retail [Guard] .030/.016, [Sticky] .016/.016, [Attack] .016/.016',
                         'cadence rows run only the Update mission pieces; UpdateAnimation, TechnoClass::'
                         'AI_Update\'s other steps and the passive scan are not run (the row events stand '
                         'in for the scan\'s TarCom write)'],
            substitutions=['BuildingClass::GetFireError 0x447F10 answered (per row, or the rearm test in '
                           'cadence rows); Fire_At 0x6FDD50 answered (cadence rows write its rearm timer); '
                           'TechnoClass::SetTarget 0x6FCDB0 answered with its +0x2B4 write; FacingClass::'
                           'Set_Desired 0x4C9220, the direction 0x43ED40, StartUncloaking 0x7036C0, the '
                           'delayed-fire anim 0x451890 and ClearBibArea 0x449540 observed and answered '
                           '(DestroyNthAnim 0x451E40 by the refinery_dock observer); Is_Operational 0x4555D0, IsCloseEnough 0x6F7780, IsHumanPlayer '
                           '0x50B730 and the EMP test 0x70EFD0 answered from the row']),
        argv=argv)


if __name__ == '__main__':
    main()
