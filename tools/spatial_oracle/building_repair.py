"""Original building repair: BuildingClass::ToggleRepair, the repair step's
cost, BuildingClass::UpdateRepairAndPower's auto-repair start and repair tick,
and HouseClass::Update's release of the owner's auto-repair latch.

- `cost` rows: BuildingTypeClass's repair step cost (vt+0xB0 = 0x7120D0)
  natively on the slave_manager fixture's refinery type: GetCost (vt+0xAC =
  0x45ED50: Cost through TechnoTypeClass::GetCost 0x711EB0, less the average
  of the two PadAircraft= types' costs when this type is the first pad
  aircraft's first Dock= and SeparateAircraft= is clear, less the FreeUnit='s
  cost floored at 0), Strength / RepairStep and the cost over those steps
  (IDIV), times RepairPercent under the ambient PC53/chop x87 word, through
  ftol, at least 1.
- `toggle` rows: BuildingClass::ToggleRepair (0x446FF0, vt+0x19C) over its
  control (-1 toggles, 0 stops, 1 starts, any other keeps), the repair byte
  (+0x6E8), Health at or below Strength and the local player (IsHumanPlayer
  0x50B6F0: PlayerPtr in a multiplayer game, +0x1EC or +0x1ED in a campaign):
  ScoldSound= or GenericClick= through VocClass::PlayAt (0x7509E0) at the
  building's Location, the flash (vt+0x148 = 0x456E00), the wrench byte
  (+0x6DE) and EVA_Repairing (0x752700). Flash, EVA and PlayAt are observed
  and answered.
- `update` rows: BuildingClass::UpdateRepairAndPower (0x450630) from its entry
  to its return on the refinery: the admission (CurrentIQ against [IQ]
  RepairSell=, Get_Mission, Can_Repair, Available_Money against [AI]
  CreditReserve=), the computer's auto-repair start (0x4506B2: the owner's
  latch +0x245, the repair byte, HasBeenCaptured +0x6E3, AIRepairable +0x6CB,
  IsControlledByHuman 0x50B730; the latch, ToggleRepair(1), and for a house
  no human controls the latch timer +0x280 = {Frame, TimeLeft =
  RandomRanged(ftol(RepairDelay * 225), ftol(RepairDelay * 1800))} on the
  Scenario RNG, RepairDelay = House+0x1C0) and the repair tick (0x450813:
  Frame % ftol(RepairRate * 900), the wrench byte, the step cost against
  Available_Money, HouseClass::Spend_Money 0x4F9790, RepairStep added to
  Health +0x6C and the estimate +0x70, the clamp at Strength that ends the
  repair, the damage-state slots and the smoke's retirement).
- `build` rows: a damaged building's build-up on the routes of
  building_construction's `route` rows (a deploy, a computer house's
  placement, a human player's placement), per frame BuildingClass::Update's
  construction pieces then UpdateRepairAndPower, until two frames after
  Grand_Opening: Get_Mission reads Construction (current, or only queued
  before a human player's placement commences) through the build-up and
  Guard on the completion frame, whose UpdateRepairAndPower starts the
  repair. The fixture's observers also see the pieces' calls and the Guard
  mission's draws; each frame keeps only what UpdateRepairAndPower calls and
  draws.
- `wrench` rows: TechnoClass::DrawExtras' repair wrench frame (0x6F52D8..
  0x6F532D) over the stored game speed (0xA8EB60) and Frame: the cycle
  SpeedNormalize(14) / 4 (signed, at least 2) and WRENCH.SHP's frame
  ((Frame % cycle) * 6) / (cycle - 1).
- `release` rows: HouseClass::Update's release of the latch (0x4F9302..
  0x4F9338): the latch clears once its timer expired (Start -1: TimeLeft 0;
  otherwise Frame - Start >= TimeLeft, signed).

Usage: python -m tools.spatial_oracle.building_repair [--check|--write]
"""
from pathlib import Path
import struct

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_EDX, UC_X86_REG_ESP
from tools.native_oracle import RET_MAGIC, finish_vectors, provenance, run_checked
from tools.spatial_oracle import building_construction as bc
from tools.spatial_oracle import slave_manager as sm
from tools.spatial_oracle.building_sale import MISSION, coord, ret, signed
from tools.spatial_oracle.map_queries import dwords
from tools.spatial_oracle.refinery_dock import HOUSE, RULES
from tools.spatial_oracle.unit_source_scatter import SCENARIO

TOGGLE_REPAIR, REPAIR_STEP_COST, UPDATE_REPAIR_AND_POWER = 0x446FF0, 0x7120D0, 0x450630
HOUSE_RELEASE = (0x4F9302, 0x4F9338)
# RandomRanged, the returns of the latch timer's draw and of the sale's roll.
RANDOM_RANGED, TIMER_DRAW_RETURN, SALE_ROLL_RETURN = 0x65C7E0, 0x45075E, 0x4507C9
PLAY_AT, PLAY_EVA, BUILDING_FLASH = 0x7509E0, 0x752700, 0x456E00
# BuildingClass::GetCurrentFrame (only the redraw byte +0x80 reads it here),
# the damage-state slot anim and Sell_Back.
CURRENT_FRAME, CREATE_ANIM_FOR_SLOT, SELL_BACK = 0x43EF90, 0x451890, 0x447110
PLAYER_PTR, FRAME = 0xA83D4C, 0xA8ED84
# TechnoClass::DrawExtras' wrench frame and the stored game speed
# (GameOptionsClass 0xA8EB60, SpeedNormalize's receiver).
WRENCH_FRAME, GAME_SPEED = (0x6F52D8, 0x6F532D), 0xA8EB60
HOUSE_MONEY_VTABLE = 0x7EA834
# AircraftTypeClass and UnitTypeClass vtables (GetCost vt+0xAC = 0x711EB0) for
# the PadAircraft= pair and the FreeUnit=; ParticleSystemClass's for the smoke
# (+0x310), whose vt+0xF8 sets its done byte (+0xF8).
AIRCRAFT_TYPE_VTABLE, UNIT_TYPE_VTABLE, PARTICLE_SYSTEM_VTABLE = 0x7E2868, 0x7F6218, 0x7EFB9C
# Scratch after the slave_manager fixture's region: the pad pair and its
# vector, the free unit's type and the smoke.
REGION = sm.REGION + 0x40000
PAD_ITEMS, PAD_TYPES, PAD_DOCKS = REGION, REGION + 0x1000, REGION + 0x3000
FREE_UNIT = REGION + 0x4000
SMOKE = REGION + 0x6000
TYPE_SIZE = 0x1000
# Sound indices for [AudioVisual] ScoldSound= and GenericClick=.
SCOLD_SOUND, GENERIC_CLICK = 43, 42
# Retail `RepairPercent=15%` (ReadDouble: 15.0f widened, times .01) and the
# RulesClass constructor's .25; retail `RepairRate=.016` and the difficulty
# rows' `RepairDelay=.02`/`.05` (a `%f` float widened); the ReadDifficulty
# default .02 (0x0066D317).
PERCENT_15, PERCENT_25 = 0x3FC3333333333333, 0x3FD0000000000000


def widened(text):
    return struct.unpack('<Q', struct.pack('<d', struct.unpack('<f', struct.pack('<f', float(text)))[0]))[0]


RATE_016, DELAY_02, DELAY_05, DELAY_02_DEFAULT = widened('.016'), widened('.02'), widened('.05'), 0x3F947AE147AE147B


def double_bits(bits):
    return struct.pack('<Q', bits)


def fixture(case):
    """The slave_manager fixture's refinery (a Building over YTYPE, 2x2 at NW
    (12, 12)) owned by HOUSE, with the pad vector the type cost reads."""
    u, call, read32, events = sm.make_fixture(dict(
        name=case['name'], manager_state=0, nodes=[], ore=[], seed=case.get('seed', 1),
        human=case.get('human', False), game_mode=case.get('game_mode', 1)))
    u.mem_map(REGION, 0x10000)
    kind = sm.YTYPE
    # Cost=, Strength=, FreeUnit=, the Rules repair keys.
    u.mem_write(kind + 0x610, dwords(case.get('cost', 2500)))
    u.mem_write(kind + 0xA0, dwords(case.get('strength', 1000)))
    free_unit = case.get('free_unit')
    u.mem_write(kind + 0xEA0, dwords(FREE_UNIT if free_unit is not None else 0))
    if free_unit is not None:
        u.mem_write(FREE_UNIT, dwords(UNIT_TYPE_VTABLE))
        u.mem_write(FREE_UNIT + 0x610, dwords(free_unit))
    u.mem_write(RULES + 0x16CC, dwords(case.get('step', 8)))
    u.mem_write(RULES + 0x16D0, double_bits(case.get('percent', PERCENT_15)))
    u.mem_write(RULES + 0x16E0, double_bits(case.get('rate', RATE_016)))
    # [General] PadAircraft= (the vector's items at Rules+0xB5C), each type's
    # Cost= and Dock= list (+0x3EC), and SeparateAircraft= (+0x17E8).
    pads = case.get('pads', [1000, 1200])
    u.mem_write(RULES + 0xB5C, dwords(PAD_ITEMS))
    u.mem_write(PAD_ITEMS, dwords(*[PAD_TYPES + index * TYPE_SIZE for index in range(len(pads))]))
    for index, pad_cost in enumerate(pads):
        pad = PAD_TYPES + index * TYPE_SIZE
        u.mem_write(pad, dwords(AIRCRAFT_TYPE_VTABLE))
        u.mem_write(pad + 0x610, dwords(pad_cost))
        u.mem_write(pad + 0x3EC, dwords(PAD_DOCKS + index * 0x10))
    u.mem_write(PAD_DOCKS, dwords(kind if case.get('pad_dock') else sm.STYPE))
    u.mem_write(RULES + 0x17E8, bytes([case.get('separate_aircraft', True)]))
    return u, call, read32, events


# --- cost ----------------------------------------------------------------


def cost(case):
    u, _call, _read32, _events = fixture(case)
    value = bc.invoke(u, REPAIR_STEP_COST, sm.YTYPE)
    return dict(input=case, cost=struct.unpack('<i', dwords(value))[0])


def cost_cases():
    rows = []
    # Strength 1000 over RepairStep 8: 125 steps. Per-step costs 20, 40 and
    # 100 times 15% land just below 3, 6 and 15 in double precision.
    for base_cost in (0, 100, 750, 800, 875, 1000, 2000, 2500, 5000, 12500, 100000):
        rows.append(dict(name=f'k_{base_cost}', cost=base_cost))
    rows += [
        # The RulesClass constructor's 25%.
        dict(name='k_2500_default_percent', cost=2500, percent=PERCENT_25),
        # Strength not a multiple of the step, a step of 1, a large step.
        dict(name='k_uneven_strength', cost=2000, strength=1100),
        dict(name='k_step_one', cost=2000, step=1),
        dict(name='k_step_strength', cost=2000, step=1000),
        # A negative cost (the per-step IDIV truncates toward zero).
        dict(name='k_negative', cost=-5000),
        # FreeUnit=: less its cost, floored at 0 only there.
        dict(name='k_free_unit', cost=3000, free_unit=1400),
        dict(name='k_free_unit_exceeds', cost=3000, free_unit=5000),
        dict(name='k_free_unit_equal', cost=3000, free_unit=3000),
        # The first pad aircraft docks at this type: less the pair's average
        # (signed division) unless SeparateAircraft=.
        dict(name='k_pad_dock', cost=3000, pad_dock=True, separate_aircraft=False),
        dict(name='k_pad_dock_odd_sum', cost=3000, pad_dock=True, separate_aircraft=False, pads=[1000, 1201]),
        dict(name='k_pad_dock_separate', cost=3000, pad_dock=True),
        dict(name='k_pad_other_dock', cost=3000, separate_aircraft=False),
        dict(name='k_pad_dock_below_zero', cost=500, pad_dock=True, separate_aircraft=False),
        dict(name='k_pad_dock_free_unit', cost=3000, pad_dock=True, separate_aircraft=False, free_unit=1000),
    ]
    return rows


# --- toggle --------------------------------------------------------------


def observe_toggle(u, read32, events):
    def hook(_u, address, _size, _data):
        sp = u.reg_read(UC_X86_REG_ESP)
        if address == BUILDING_FLASH:
            events.append(['flash', read32(sp + 4)])
            ret(u, read32, 4)
        elif address == PLAY_EVA:
            name = bytes(u.mem_read(u.reg_read(UC_X86_REG_ECX), 32)).split(b'\0')[0].decode('ascii')
            events.append(['eva', name, struct.unpack('<i', dwords(u.reg_read(UC_X86_REG_EDX)))[0],
                           struct.unpack('<i', dwords(read32(sp + 4)))[0]])
            ret(u, read32, 4)
        elif address == PLAY_AT:
            events.append(['play_at', u.reg_read(UC_X86_REG_ECX), coord(u, u.reg_read(UC_X86_REG_EDX)),
                           read32(sp + 4)])
            ret(u, read32, 4)

    u.hook_add(UC_HOOK_CODE, hook)


def building_state(u, read32):
    building = sm.YAREFN
    return dict(health=signed(u, building + 0x6C), estimate=signed(u, building + 0x70),
                repairing=u.mem_read(building + 0x6E8, 1)[0], wrench=u.mem_read(building + 0x6DE, 1)[0],
                damaged=u.mem_read(building + 0x6E6, 1)[0])


def toggle(case):
    u, _call, read32, events = fixture(case)
    building = sm.YAREFN
    u.mem_write(building + 0x6C, dwords(case['health']))
    u.mem_write(building + 0x6E8, bytes([case['repairing']]))
    u.mem_write(building + 0x6DE, bytes([case.get('wrench', 0)]))
    u.mem_write(RULES + 0x700, dwords(SCOLD_SOUND))
    u.mem_write(RULES + 0x70C, dwords(GENERIC_CLICK))
    u.mem_write(HOUSE + 0x1ED, bytes([case.get('player_control', False)]))
    u.mem_write(PLAYER_PTR, dwords(HOUSE if case.get('player') else 0))
    observe_toggle(u, read32, events)
    bc.invoke(u, TOGGLE_REPAIR, building, case['control'] & 0xFFFFFFFF)
    return dict(input=case, events=events, location=coord(u, building + 0x9C), **building_state(u, read32))


def toggle_cases():
    rows = []
    for control in (-1, 0, 1, 2):
        for repairing in (0, 1):
            for health, label in ((500, 'damaged'), (1000, 'full')):
                for player in (False, True):
                    rows.append(dict(name=f't_{control}_{repairing}_{label}_{"player" if player else "other"}',
                                     control=control, repairing=repairing, health=health, player=player))
    # A campaign: IsHumanPlayer is House+0x1EC or +0x1ED, not PlayerPtr.
    for human, player_control in ((True, False), (False, True), (False, False)):
        rows.append(dict(name=f't_campaign_{int(human)}{int(player_control)}', control=-1, repairing=0,
                         health=500, game_mode=0, human=human, player_control=player_control))
    # Health above Strength: GenericClick and the local player's EVA.
    rows.append(dict(name='t_above_strength', control=1, repairing=0, health=1001, player=True))
    return rows


# --- update --------------------------------------------------------------


# Slots whose occupant the damage-state change replaces: 0 and 2 name both
# anims, 1 is occupied without names, 3 has names and no occupant.
SLOT_OCCUPANTS = {0: True, 1: True, 2: True, 3: False}
SLOT_NAMES = {0: True, 1: False, 2: True, 3: True}


def prepare_update(u, read32, events, case):
    """The update rows' building, owner and Rules at the row's frame, with
    UpdateRepairAndPower's callees observed; returns the draws and the hook's
    state (GetCurrentFrame is answered only inside UpdateRepairAndPower)."""
    building, kind = sm.YAREFN, sm.YTYPE
    frame = case.get('frame', 196)
    u.mem_write(FRAME, dwords(frame))
    health = case.get('health', 300)
    u.mem_write(building + 0x6C, dwords(health, case.get('estimate', health)))
    u.mem_write(building + 0xAC, dwords(MISSION[case.get('mission', 'guard')]))
    u.mem_write(building + 0xB4, dwords(MISSION[case.get('queue', 'none')]))
    u.mem_write(building + 0x6E8, bytes([case.get('repairing', False)]))
    u.mem_write(building + 0x6DE, bytes([case.get('wrench', 0)]))
    u.mem_write(building + 0x6E3, bytes([case.get('captured', False)]))
    u.mem_write(building + 0x6CB, bytes([case.get('ai_repairable', True)]))
    u.mem_write(building + 0x3D1, bytes([case.get('attacked', False)]))
    u.mem_write(building + 0x6DC, b'\x00')
    u.mem_write(building + 0x34, dwords(0))
    u.mem_write(building + 0x80, b'\x00')
    # The retained damage state (+0x6E6) as the last change left it: damaged
    # at or below ConditionYellow.
    strength = case.get('strength', 1000)
    u.mem_write(building + 0x6E6, bytes([case.get('damaged', health * 2 <= strength)]))
    for slot in range(21):
        occupied = SLOT_OCCUPANTS.get(slot, False)
        u.mem_write(building + 0x55C + slot * 4, dwords(REGION + 0x8000 + slot * 0x10 if occupied else 0))
        for offset, prefix in ((0xF4C, 'N'), (0xF5C, 'D')):
            named = SLOT_NAMES.get(slot, False)
            u.mem_write(kind + slot * 0x44 + offset, (f'{prefix}{slot:02}' if named else '').encode() + b'\0')
    u.mem_write(building + 0x310, dwords(SMOKE if case.get('smoke', True) else 0))
    u.mem_write(SMOKE, dwords(PARTICLE_SYSTEM_VTABLE))
    # ClickRepairable=, Repairable=, no UndeploysInto=, a 2x2 foundation,
    # not a yard.
    u.mem_write(kind + 0x157A, bytes([case.get('click_repairable', True)]))
    u.mem_write(kind + 0xCCC, b'\x01')
    u.mem_write(kind + 0x408, dwords(0))
    u.mem_write(kind + 0xEF0, dwords(3))
    u.mem_write(kind + 0xEB8, dwords(-1))
    # The owner: money interface, Balance, CurrentIQ, the authored IQ and
    # TechLevel, the latch and its timer, RepairDelay, player control.
    u.mem_write(HOUSE + 0x24, dwords(HOUSE_MONEY_VTABLE))
    u.mem_write(HOUSE + 0x30C, dwords(case.get('balance', 5000)))
    u.mem_write(HOUSE + 0x2DC, dwords(0))
    u.mem_write(HOUSE + 0x24C, dwords(case.get('current_iq', 2)))
    u.mem_write(HOUSE + 0x1D0, dwords(2, 10))
    u.mem_write(HOUSE + 0x245, bytes([case.get('latched', False)]))
    u.mem_write(HOUSE + 0x280, dwords(*case.get('timer', [0, 0x5A5A5A5A, 0])))
    u.mem_write(HOUSE + 0x1C0, double_bits(case.get('delay', DELAY_02)))
    u.mem_write(HOUSE + 0x1ED, bytes([case.get('player_control', False)]))
    u.mem_write(PLAYER_PTR, dwords(HOUSE if case.get('player') else 0))
    # [IQ] RepairSell=/SellBack=, [AI] CreditReserve=, [AudioVisual]
    # ConditionYellow= (the fixture's .5) and ConditionRed=, the sounds.
    u.mem_write(RULES + 0x1444, dwords(1))
    u.mem_write(RULES + 0x145C, dwords(2))
    u.mem_write(RULES + 0x1758, dwords(case.get('credit_reserve', 100)))
    u.mem_write(RULES + 0x1708, struct.pack('<d', 0.25))
    u.mem_write(RULES + 0x700, dwords(SCOLD_SOUND))
    u.mem_write(RULES + 0x70C, dwords(GENERIC_CLICK))
    observe_toggle(u, read32, events)
    draws = []
    state = dict(in_update=False)

    def hook(_u, address, _size, _data):
        sp = u.reg_read(UC_X86_REG_ESP)
        if address == RANDOM_RANGED:
            draws.append([signed(u, sp + 4), signed(u, sp + 8)])
        elif address in (TIMER_DRAW_RETURN, SALE_ROLL_RETURN):
            draws[-1].append(u.reg_read(UC_X86_REG_EAX))
        elif address == TOGGLE_REPAIR:
            events.append(['toggle_repair', signed(u, sp + 4)])
        elif address == CURRENT_FRAME and state['in_update']:
            ret(u, read32, 0, 0)
        elif address == CREATE_ANIM_FOR_SLOT:
            name = bytes(u.mem_read(read32(sp + 4), 16)).split(b'\0')[0].decode('ascii')
            events.append(['slot_anim', name, *[signed(u, sp + offset) for offset in (8, 12, 16, 20)]])
            ret(u, read32, 20)
        elif address == SELL_BACK:
            events.append(['sell_back', signed(u, sp + 4)])
            ret(u, read32, 4, 1)

    u.hook_add(UC_HOOK_CODE, hook)
    return draws, state


def run_update(u, state):
    state['in_update'] = True
    bc.invoke(u, UPDATE_REPAIR_AND_POWER, sm.YAREFN)
    state['in_update'] = False


def owner_state(u, read32):
    return dict(balance=signed(u, HOUSE + 0x30C), spent=signed(u, HOUSE + 0x2DC),
                latched=u.mem_read(HOUSE + 0x245, 1)[0],
                timer=[signed(u, HOUSE + 0x280), signed(u, HOUSE + 0x288)])


def random_indices(read32):
    return [read32(SCENARIO + 0x21C), read32(SCENARIO + 0x220)]


def update(case):
    """UpdateRepairAndPower (module doc) from its entry to its return."""
    u, _call, read32, events = fixture(case)
    draws, state = prepare_update(u, read32, events, case)
    before = random_indices(read32)
    run_update(u, state)
    return dict(input=case, draws=draws, events=events, **building_state(u, read32), **owner_state(u, read32),
                smoke_done=u.mem_read(SMOKE + 0xF8, 1)[0], redraw=u.mem_read(sm.YAREFN + 0x80, 1)[0],
                random_indices=dict(before=before, after=random_indices(read32)))


def update_cases():
    return [
        # A computer house's AIRepairable building below Strength: the latch,
        # ToggleRepair(1), the latch timer's draw, then the tick on a frame
        # the period (ftol(.016f * 900) = 14) divides.
        dict(name='u_start'),
        dict(name='u_start_seed_7', seed=7),
        dict(name='u_start_off_cadence', frame=200),
        dict(name='u_start_frame_zero', frame=0),
        # The latch holds any start; the repair tick still runs.
        dict(name='u_latched', latched=True),
        dict(name='u_latched_repairing', latched=True, repairing=True),
        # Already repairing: past the start to the tick (0x450821).
        dict(name='u_repairing', repairing=True),
        dict(name='u_repairing_off_cadence', repairing=True, frame=197),
        # Not AIRepairable: only a capture or a human's control starts it.
        dict(name='u_not_flagged', ai_repairable=False),
        dict(name='u_captured', ai_repairable=False, captured=True),
        dict(name='u_human', ai_repairable=False, human=True),
        dict(name='u_human_player', ai_repairable=False, human=True, player=True),
        dict(name='u_campaign', ai_repairable=False, game_mode=0),
        dict(name='u_campaign_flagged', game_mode=0),
        dict(name='u_campaign_player_control', ai_repairable=False, game_mode=0, player_control=True),
        # The admission: CurrentIQ, the mission, Can_Repair, money below the
        # reserve (the sale arm, not attacked here).
        dict(name='u_iq_below', current_iq=0),
        dict(name='u_iq_below_repairing', current_iq=0, repairing=True),
        dict(name='u_selling_repairing', mission='selling', repairing=True),
        dict(name='u_construction_repairing', mission='construction', repairing=True),
        # Construction or Selling, current or only queued (Get_Mission),
        # holds the start (0x450679 -> 0x450813).
        dict(name='u_construction_start', mission='construction'),
        dict(name='u_construction_queued_start', mission='none', queue='construction'),
        dict(name='u_selling_start', mission='selling'),
        dict(name='u_not_click_repairable', click_repairable=False),
        dict(name='u_below_reserve', balance=99),
        dict(name='u_below_reserve_repairing', balance=99, repairing=True),
        dict(name='u_below_reserve_attacked', balance=99, attacked=True, health=200),
        dict(name='u_at_reserve', balance=100),
        # Full Strength: no start; a repair already on spends one step and
        # clamps.
        dict(name='u_full', health=1000),
        dict(name='u_full_repairing', health=1000, repairing=True),
        # The step cost against the money: 100000 over 125 steps is 800,
        # times 15% is 119.
        dict(name='u_cannot_afford', cost=100000, balance=118),
        dict(name='u_affords_exactly', cost=100000, balance=119, credit_reserve=0),
        dict(name='u_affords_exactly_repairing', cost=100000, balance=119, repairing=True),
        # The clamp: past, at and just below Strength.
        dict(name='u_completes', health=995, repairing=True),
        dict(name='u_exact', health=992, repairing=True),
        dict(name='u_one_short', health=991, repairing=True),
        # The estimate (+0x70) takes the same step, and Strength at the clamp.
        dict(name='u_estimate_differs', estimate=250, repairing=True),
        dict(name='u_estimate_differs_completes', health=995, estimate=900, repairing=True),
        # The damage state at ConditionYellow (.5): crossing above replaces
        # the occupied named slots and retires the smoke; at yellow it stays.
        dict(name='u_crosses_yellow', health=496, repairing=True),
        dict(name='u_to_yellow', health=492, repairing=True),
        dict(name='u_above_yellow', health=600, repairing=True),
        dict(name='u_crosses_yellow_no_smoke', health=496, repairing=True, smoke=False),
        dict(name='u_stale_damaged_state', health=600, repairing=True, damaged=True),
        # RepairDelay: the difficulty rows' .02/.05 as ReadDouble stores
        # them, ReadDifficulty's default, and 0 (the House constructor's).
        dict(name='u_delay_05', delay=DELAY_05),
        dict(name='u_delay_default', delay=DELAY_02_DEFAULT),
        dict(name='u_delay_zero', delay=0),
        # A wrench byte already on flips back.
        dict(name='u_wrench_on', repairing=True, wrench=1),
        # The latch timer overwritten while an older one runs.
        dict(name='u_timer_overwritten', timer=[150, 0, 40]),
    ]


# --- build ---------------------------------------------------------------


def build(case):
    """A damaged building's build-up (the `build` rows): the update rows'
    building, owner and Rules at the row's start frame, created there on the
    row's route as building_construction's `route` rows create it, then per
    frame BuildingClass::Update's construction pieces (bc.building_update)
    followed by UpdateRepairAndPower, until two frames after Grand_Opening."""
    u, _call, read32, events = fixture(case)
    building, kind = sm.YAREFN, sm.YTYPE
    start = case['frame']
    draws, state = prepare_update(u, read32, events, dict(case, mission='none'))
    calls = []

    def hook(_u, address, _size, _data):
        sp = u.reg_read(UC_X86_REG_ESP)
        if address in bc.PRESENTATION:
            ret(u, read32, bc.PRESENTATION[address])
        elif address in (bc.RADIO_BROADCAST, bc.RADIO_BROADCAST_ALL):
            calls.append(['radio', read32(sp + 4)])
            ret(u, read32, 4)
        elif address == bc.GRAND_OPENING:
            calls.append(['grand_opening', read32(sp + 4)])
            ret(u, read32, 4)
        elif address in (bc.LOOP_UPDATE, bc.SOUND_RELEASE):
            ret(u, read32, 0)
        elif address == bc.TECHNO_RECEIVE_RADIO:
            ret(u, read32, 12, 1)

    u.hook_add(UC_HOOK_CODE, hook)
    # The route rows' creation state (building_construction.building_fixture
    # and route): the control, no UndeploysInto, the TechnoClass
    # constructor's stage state, BState and queued BState -1, in play, +0x6E9.
    u.mem_write(kind + 0xF04, dwords(*case['control']))
    u.mem_write(building + 0xBC, dwords(0))
    u.mem_write(building + 0x218, dwords(0))
    u.mem_write(building + 0x534, dwords(-1))
    u.mem_write(building + 0x538, dwords(-1))
    u.mem_write(building + 0x6DD, bytes([0]))
    u.mem_write(building + 0xF8, dwords(0))
    u.mem_write(building + 0x100, dwords(start, 0, 0, 0, 1))
    u.mem_write(building + 0xC8, dwords(start, 0, 0))
    u.mem_write(building + 0x90, bytes([1]))
    u.mem_write(building + 0x6E9, bytes([1]))
    u.mem_write(bc.SCENARIO_INIT, dwords(0))
    u.mem_write(bc.SCENARIO_FLAG_ED6B, bytes([0]))
    route = case['route']
    bc.invoke(u, bc.ENTER_CONSTRUCTION, building, 1, 1)
    if route == 'computer':
        bc.invoke(u, bc.COMMENCE, building)
    elif route == 'player':
        bc.invoke(u, bc.RECEIVE_RADIO, building, sm.YAREFN + 0x1000, 3, 0)
    elif route == 'deploy':
        bc.invoke(u, bc.QUEUE_MISSION, building, 0x12, 0)
        u.mem_write(building + 0x6DD, bytes([1]))
    frames = []
    completed = None
    # A deployed or computer-placed building's first Update is in its
    # creation frame (building_construction's route rows).
    k = 0 if route in ('deploy', 'computer') else 1
    while completed is None or k <= completed + 2:
        assert k <= case['frames'], case['name']
        u.mem_write(FRAME, dwords(start + k))
        first_call = len(calls)
        bc.building_update(u, building)
        grand = any(call[0] == 'grand_opening' for call in calls[first_call:])
        if grand:
            completed = k
        # Get_Mission's two words as UpdateRepairAndPower reads them, and
        # what it calls and draws (the fixture's observers also see the
        # pieces' calls and the Guard mission's draws).
        mission = [signed(u, building + 0xAC), signed(u, building + 0xB4)]
        first_event, first_draw, before = len(events), len(draws), random_indices(read32)
        run_update(u, state)
        frames.append(dict(frame=start + k, mission=mission, grand_opening=grand, events=events[first_event:],
                           draws=draws[first_draw:], random_indices=dict(before=before, after=random_indices(read32)),
                           **building_state(u, read32), **owner_state(u, read32)))
        k += 1
    return dict(input=case, frames=frames)


def build_cases():
    return [
        # A computer's deployed yard: complete at D + 1 + (count - 1) * rate,
        # here 196, a repair-step frame (196 % 14 == 0).
        dict(name='b_deploy_3x2', route='deploy', control=[0, 3, 2], frame=191, frames=12),
        dict(name='b_deploy_1x0', route='deploy', control=[0, 1, 0], frame=191, frames=12),
        # A computer house's factory placement: complete at N +
        # (count - 1) * rate, off the step period.
        dict(name='b_computer_3x2', route='computer', control=[0, 3, 2], frame=190, frames=12),
        dict(name='b_computer_4x1', route='computer', control=[0, 4, 1], frame=190, frames=12),
        # A human player's placement (IsControlledByHuman starts it without
        # the timer; the local player hears it): complete at N + 2 +
        # (count - 1) * rate.
        dict(name='b_player_3x2', route='player', control=[0, 3, 2], frame=190, frames=12, human=True,
             ai_repairable=False, player=True),
    ]


# --- wrench --------------------------------------------------------------


def wrench(case):
    """TechnoClass::DrawExtras' repair wrench frame (0x6F52D8..0x6F532D): the
    cycle SpeedNormalize(14) / 4 (signed), at least 2, and WRENCH.SHP's frame
    ((Frame % cycle) * 6) / (cycle - 1), both signed IDIVs."""
    u, _call, _read32, _events = fixture(case)
    u.mem_write(GAME_SPEED, dwords(case['speed']))
    u.mem_write(FRAME, dwords(case['frame']))
    bc.run_block(u, sm.YAREFN, WRENCH_FRAME)
    return dict(input=case, frame_index=struct.unpack('<i', dwords(u.reg_read(UC_X86_REG_EAX)))[0])


def wrench_cases():
    return [dict(name=f'w_{speed}_{frame}', speed=speed, frame=frame)
            for speed in range(8)
            for frame in (0, 1, 2, 3, 4, 5, 6, 13, 14, 27, 28, 55, 196, 1000, 123457, 0x7FFFFFFF, -1, -30)]


# --- release -------------------------------------------------------------


def release(case):
    u, _call, read32, _events = fixture(case)
    u.mem_write(FRAME, dwords(case['frame']))
    u.mem_write(HOUSE + 0x245, bytes([case['latched']]))
    start, time_left = case['timer']
    u.mem_write(HOUSE + 0x280, dwords(start, 0, time_left))
    bc.run_block(u, HOUSE, HOUSE_RELEASE)
    return dict(input=case, latched=u.mem_read(HOUSE + 0x245, 1)[0])


def release_cases():
    rows = []
    for latched in (0, 1):
        for start, time_left, frame in ((100, 30, 129), (100, 30, 130), (100, 30, 131), (-1, 0, 50),
                                        (-1, 5, 50), (300, 30, 200), (200, 0, 200), (100, -5, 90),
                                        (0, 0, 0), (0x7FFFFFF0, 30, 16)):
            rows.append(dict(name=f'r_{latched}_{start}_{time_left}_{frame}', latched=latched,
                             timer=[start, time_left], frame=frame))
    return rows


def generate():
    return {'source': 'unicorn/gamemd.exe',
            # The Rules and House doubles the rows write, as bits.
            'constants': {name: f'{bits:016x}' for name, bits in (
                ('percent_15', PERCENT_15), ('percent_25', PERCENT_25), ('rate_016', RATE_016),
                ('delay_02', DELAY_02), ('delay_05', DELAY_05), ('delay_02_default', DELAY_02_DEFAULT))},
            'cost': [cost(case) for case in cost_cases()],
            'toggle': [toggle(case) for case in toggle_cases()],
            'update': [update(case) for case in update_cases()],
            'build': [build(case) for case in build_cases()],
            'wrench': [wrench(case) for case in wrench_cases()],
            'release': [release(case) for case in release_cases()]}


def main(argv=None):
    finish_vectors(
        generate, Path(__file__).with_suffix('.json'),
        provenance=lambda: provenance(
            scope='BuildingTypeClass repair step cost 0x7120D0 with GetCost 0x45ED50; BuildingClass::'
                  'ToggleRepair 0x446FF0; BuildingClass::UpdateRepairAndPower 0x450630 from entry to return '
                  '(admission, the computer\'s auto-repair start and latch timer, the repair tick), alone and '
                  'after BuildingClass::Update\'s construction pieces through a build-up; '
                  'HouseClass::Update\'s latch release 0x4F9302..0x4F9338; TechnoClass::DrawExtras\' repair '
                  'wrench frame 0x6F52D8..0x6F532D',
            entry_points={'repair_step_cost': REPAIR_STEP_COST, 'toggle_repair': TOGGLE_REPAIR,
                          'update_repair_and_power': UPDATE_REPAIR_AND_POWER,
                          'house_release': HOUSE_RELEASE[0], 'enter_construction': bc.ENTER_CONSTRUCTION,
                          'commence': bc.COMMENCE, 'queue_mission': bc.QUEUE_MISSION,
                          'receive_radio': bc.RECEIVE_RADIO, 'update_animation': bc.UPDATE_ANIMATION,
                          'mission_ai': bc.MISSION_AI, 'update_ready_commence_unless_building': 0x43FE27,
                          'update_ready_commence': 0x43FF91, 'update_queued_bstate': 0x43FFB4,
                          'draw_extras_wrench_frame': WRENCH_FRAME[0]},
            assumptions=['the slave_manager fixture refinery (Building vtables over a BuildingType-vtable '
                         'type, 2x2 at NW (12, 12)) owned by the fixture House; multiplayer game mode unless '
                         '`game_mode` is 0; PlayerPtr the owner only when `player`; the Scenario RNG seeded '
                         'through the original seeder',
                         'Rules: RepairStep 8, RepairPercent 15% (0x3FC3333333333333) unless the row sets it, '
                         'RepairRate .016f widened, ConditionYellow .5, ConditionRed .25, [IQ] RepairSell 1 and '
                         'SellBack 2, [AI] CreditReserve 100 unless the row sets it; the PadAircraft= vector holds '
                         'two AircraftType-vtable types (Cost 1000/1200 unless the row sets them) whose first\'s '
                         'first Dock= is this type only when `pad_dock`; SeparateAircraft= set unless the row '
                         'clears it; a FreeUnit= is a UnitType-vtable type',
                         'update rows: the owner\'s money interface House+0x24 holds the constructor\'s vtable '
                         '0x7EA834, its storage empty (Available_Money = Balance); authored IQ 2, TechLevel 10; '
                         'the latch timer\'s unread +0x284 word preset; damage-state slots 0..3 as SLOT_OCCUPANTS/'
                         'SLOT_NAMES name them; the smoke (+0x310) a ParticleSystemClass-vtable object',
                         'build rows: the update rows\' state with no mission, then building_construction\'s route '
                         'creation (the control at Type+0xF04, the TechnoClass constructor stage state, BState and '
                         'queued BState -1, +0x90 and +0x6E9 set) at the row\'s start frame'],
            substitutions=['toggle and update rows: the flash (vt+0x148 = 0x456E00), EVA 0x752700 and '
                           'VocClass::PlayAt 0x7509E0 observed and answered',
                           'update rows: BuildingClass::GetCurrentFrame 0x43EF90 answered 0 (only the redraw '
                           'byte +0x80 reads it), the damage-state slot anim 0x451890 observed and answered '
                           '(ToggleRepair, Health_Ratio, Spend_Money, Available_Money and the smoke\'s vt+0xF8 '
                           'native), Sell_Back 0x447110 observed and answered',
                           'build rows: UpdateRepairAndPower invoked after the queued-BState block (0x440042), '
                           'the rest of BuildingClass::Update before 0x4401B6 and of TechnoClass::AI not run; '
                           'the route rows\' substitutions (UpdateAnimation presentation callees, the radio '
                           'broadcasts 0x65ACB0/0x65ACE0, Grand_Opening 0x445F80, the loop update 0x750D40, '
                           'SoundEvent::Release 0x406060, TechnoClass::Receive_Radio 0x6F4AB0 answered 1); '
                           'GetCurrentFrame answered only inside UpdateRepairAndPower',
                           'release rows: the block run alone with ESI = the House',
                           'wrench rows: the block run alone (SpeedNormalize 0x5FB2E0 native)']),
        argv=argv)


if __name__ == '__main__':
    main()
