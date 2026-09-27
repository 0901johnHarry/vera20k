"""Original factory start and step cadence for chosen builds.

A build starts with FactoryClass 0x4C9EA0 (Ghidra label FactoryClass__SetRate;
Begin_Production calls it at 0x4FA628 right after StartProduction), then
FactoryClass::AI 0x4C9B20 runs once per frame. Each row runs the originals on the
Time_To_Build oracle's fixture objects and records the rate, every step attempt
and the state at the end.
"""
from pathlib import Path
import struct
from tools.native_oracle import call, finish_vectors, provenance, SCRATCH
from tools.spatial_oracle import time_to_build as ttb

FRAME = 0xA8ED84
FACTORY = SCRATCH + 0xB000
FACTORY_SIZE = 0x74
# The HouseClass constructor's IHouse vtable (House+0x24); its +0x18 is
# Available_Money 0x4F6990.
HOUSE_MONEY_VTABLE = 0x7EA834
START_FRAME = 100


def new_factory(cost):
    """A factory as StartProduction's create path (0x4C9D6E..0x4C9DED) leaves it
    over the constructor's (0x4C98BE..0x4C9917): suspended, rate 0, the timer
    started empty, the object held and its cost owed."""
    factory = bytearray(FACTORY_SIZE)
    struct.pack_into('<iiii', factory, 0x2C, START_FRAME, 0, 0, 0)
    struct.pack_into('<i', factory, 0x3C, 1)
    struct.pack_into('<I', factory, 0x40, 0x7E8934)
    struct.pack_into('<i', factory, 0x54, 10)
    struct.pack_into('<I', factory, 0x58, ttb.OBJECT)
    factory[0x5D] = 1
    struct.pack_into('<iiiI', factory, 0x60, cost, 0, -1, ttb.HOUSE)
    factory[0x70] = 1
    factory[0x71] = 1
    return bytes(factory)


def state(factory, credits, spent):
    stage, = struct.unpack_from('<i', factory, 0x24)
    timer_start, _, timer_duration, rate = struct.unpack_from('<iiii', factory, 0x2C)
    balance, = struct.unpack_from('<i', factory, 0x60)
    return dict(stage=stage, rate=rate, timer_start=timer_start,
                timer_duration=timer_duration, balance=balance,
                on_hold=bool(factory[0x5C]), suspended=bool(factory[0x70]),
                latch=bool(factory[0x71]), credits=credits, spent=spent)


def run(row):
    """Start the build at START_FRAME, then run FactoryClass::AI each frame
    through row['last_frame'], carrying the factory and the house's credits
    (House+0x30C) and spent total (House+0x2DC) between calls."""
    base = ttb.fixture_writes(row)
    base[ttb.HOUSE + 0x24] = struct.pack('<I', HOUSE_MONEY_VTABLE)
    factory, credits, spent = new_factory(row['cost']), row['credits'], 0
    deposits = dict(row['deposits'])

    def step(entry, frame, **kwargs):
        nonlocal factory, credits, spent
        writes = dict(base)
        writes[FACTORY] = factory
        writes[FRAME] = struct.pack('<i', frame)
        writes[ttb.HOUSE + 0x2DC] = struct.pack('<i', spent)
        writes[ttb.HOUSE + 0x30C] = struct.pack('<i', credits)
        result = call(entry, ecx=FACTORY, writes=writes, timeout_instr=500_000,
                      dumps={'factory': (FACTORY, FACTORY_SIZE),
                             'spent': (ttb.HOUSE + 0x2DC, 4),
                             'credits': (ttb.HOUSE + 0x30C, 4)}, **kwargs)
        factory = bytes.fromhex(result['dumps']['factory'])
        spent, = struct.unpack('<i', bytes.fromhex(result['dumps']['spent']))
        credits, = struct.unpack('<i', bytes.fromhex(result['dumps']['credits']))
        return result

    started = step(0x4C9EA0, START_FRAME, stack_args=[0],
                   required_addresses=[0x4C9EEF, 0x4F6990])
    after_start = dict(state(factory, credits, spent), started=bool(started['eax'] & 0xFF))
    attempts = []
    for frame in range(START_FRAME, row['last_frame'] + 1):
        credits += deposits.get(frame, 0)
        timer_before, = struct.unpack_from('<i', factory, 0x2C)
        step(0x4C9B20, frame)
        timer_after, = struct.unpack_from('<i', factory, 0x2C)
        if timer_after != timer_before:
            now = state(factory, credits, spent)
            attempts.append([frame, now['stage'], now['on_hold'], now['credits']])
    return dict(row, time_to_build=ttb.run(row)['time_to_build'], after_start=after_start,
                attempts=attempts, final=state(factory, credits, spent))


def cadence(credits, last_steps, deposits=(), **inputs):
    """A row run through `last_steps` rates of its start plus a few frames."""
    row = dict(ttb.case(**inputs), credits=credits, deposits=[list(d) for d in deposits])
    rate = ttb.run(row)['time_to_build'] // 54
    row['last_frame'] = START_FRAME + max(1, min(255, rate)) * last_steps + 3
    return row


def generate():
    # The start alone: the rate and timer across Time_To_Build values around the
    # division and both clamps (0x4C9EF6..0x4C9F34).
    starts = []
    for cost, btm in ((0, 1.0), (-100, 1.0), (1, 1.0), (50, 1.0), (85, 1.0), (86, 1.0),
                      (171, 1.0), (172, 1.0), (700, 1.5), (5000, 1.0), (21800, 1.0),
                      (21858, 1.0), (22000, 1.0), (30000, 1.0)):
        row = dict(ttb.case('unit', cost, btm=btm), credits=10_000, deposits=[],
                   last_frame=START_FRAME)
        starts.append(run(row))
    # Whole builds: the retail test world's MTNK, FV and E1 (three GAPOWR, stock
    # [General]), a rate-1 build, a free build and a low-power one; then MTNK
    # with no money, with money for seven steps, with exactly one step's charge,
    # and with a deposit arriving while it waits.
    builds = [
        cadence(10_000, 56, kind='unit', cost=700, btm=1.5, power=(600, 25)),
        cadence(10_000, 56, kind='unit', cost=600, power=(600, 25)),
        cadence(10_000, 56, kind='infantry', cost=200, power=(600, 10)),
        cadence(10_000, 56, kind='infantry', cost=50),
        cadence(10_000, 56, kind='infantry', cost=0),
        cadence(10_000, 56, kind='unit', cost=600, power=(50, 100)),
        cadence(0, 5, kind='unit', cost=700, btm=1.5, power=(600, 25)),
        cadence(100, 10, kind='unit', cost=700, btm=1.5, power=(600, 25)),
        cadence(13, 3, kind='unit', cost=700, btm=1.5, power=(600, 25)),
        cadence(100, 14, deposits=[(START_FRAME + 10 * 12 + 5, 1000)],
                kind='unit', cost=700, btm=1.5, power=(600, 25)),
    ]
    return dict(start_frame=START_FRAME, starts=starts, builds=[run(row) for row in builds])


if __name__ == '__main__':
    finish_vectors(generate, Path(__file__).with_suffix('.json'), provenance=lambda: provenance(
        scope='Original FactoryClass 0x4C9EA0 (the build start Begin_Production calls at '
              '0x4FA628) and FactoryClass::AI 0x4C9B20 once per frame on a factory holding '
              'the Time_To_Build oracle fixture object: the rate, the step timer, each step '
              'attempt, the charges through the real Available_Money 0x4F6990 and '
              'Spend_Money 0x4F9790, holds on a shortfall and completion. No claim about '
              'what starts, suspends or resumes a factory, about when AI runs within a '
              'frame, or about rate rewrites after the start (0x4CA6E0).',
        assumptions=['The factory starts as StartProduction 0x4C9C70 leaves a new build '
                     '(0x4C9D6E..0x4C9DED over the constructor 0x4C98BE..0x4C9917): stage '
                     '0, suspended +0x70 and +0x71 set, rate 0, timer {start frame, 0}, '
                     'step +0x3C 1, object +0x58, owed balance +0x60 = Cost, special +0x68 '
                     '-1, house +0x6C. Both originals run at the start frame; the frame '
                     'counter [0xA8ED84] then advances by one per AI call.',
                     'The house is the Time_To_Build fixture house with its IHouse vtable '
                     '0x7EA834 at +0x24, credits +0x30C and spent +0x2DC carried between '
                     'calls, empty ore storage, zero silo capacity +0x310 and a zero '
                     'storage multiplier (country +0x148). Deposits add to +0x30C before '
                     'that frame\'s AI call.',
                     'Inputs are fixture values chosen to cover the rate clamps, retail '
                     'builds and the money branches; they are not a claim that every '
                     'combination occurs in retail play. Default native FPCW 0x0E7F.'],
        substitutions=[], entry_points={'start': 0x4C9EA0, 'factory_ai': 0x4C9B20,
                                        'time_to_build': 0x6F47A0}))
