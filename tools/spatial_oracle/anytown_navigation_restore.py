"""Original 581F50 post-load hierarchy rebuild on physical Anytown bridge states."""
from pathlib import Path
import gc
import gzip
import hashlib
import json
import sys

from tools.native_oracle import _canonical, first_difference, provenance
from tools.spatial_oracle.shrapnel_repair import packet_io
import tools.spatial_oracle.anytown_damage.navigation as nav_owner
from tools.spatial_oracle.anytown_damage.navigation import Navigation, MAP, sr
from tools.spatial_oracle.anytown_damage.navigation_inputs import (
    Inputs, extract_tiles, identity,
)
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_ESP

HERE = Path(__file__).resolve().parent
ROOT = Path(nav_owner.__file__).resolve().parents[3]
FROZEN = Path(nav_owner.__file__).with_name('navigation.json.gz')
SOURCE_EVIDENCE = HERE / 'anytown_navigation_restore.native_bytes.json'
FROZEN_SHA256 = 'a2197179f0818a7b2c231c8956601b2269df164db04e05d856b99db380a15d35'
FROZEN_PAYLOAD_SHA256 = '584e7fbd46e04bfff30b58d3c0c463df80fbcca8263aa15a3868de327842d14e'
STAGES = ('first_damage', 'collapse', 'repair')


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def normalized(value):
    return json.loads(_canonical(value))


class AtBoundary(Exception):
    """End the existing owner driver after its selected complete primitive call."""


class RestoreProbe(Navigation):
    def __init__(self, readers, theater, tiles, stop_after):
        self.capture_enabled = False
        self.stop_after = stop_after
        self.primitive_calls = []
        self.rebuild_trace = []
        super().__init__(readers, theater, tiles)
        self.capture_enabled = True

    def call(self, address, *args, **kwargs):
        result = super().call(address, *args, **kwargs)
        if self.capture_enabled and address in (0x57CCF0, 0x573540):
            self.primitive_calls.append(dict(entry=address, returned_eax=result,
                                            returned_low_byte=result & 255))
            if len(self.primitive_calls) == self.stop_after:
                raise AtBoundary
        return result

    def observe(self, u, address, size, data):
        if self.activity == 'restore_rebuild' and address in (
                0x581F50, 0x588D60, 0x581F90, 0x42C1C0):
            receiver = u.reg_read(UC_X86_REG_ECX)
            row = dict(entry=address, receiver=receiver,
                       caller=sr.u32(u, u.reg_read(UC_X86_REG_ESP)))
            if address == 0x581F90:
                row['level'] = sr.u32(u, u.reg_read(UC_X86_REG_ESP) + 4)
            if address == 0x588D60:
                assert receiver in (MAP + 0x8C, MAP + 0xA4, MAP + 0xBC)
                row['level'] = (receiver - MAP - 0x8C) // 24
                row['record_count_before'] = sr.u32(u, receiver + 16)
            self.rebuild_trace.append(row)
        return super().observe(u, address, size, data)


def generate():
    frozen_bytes = FROZEN.read_bytes()
    assert sha(frozen_bytes) == FROZEN_SHA256
    frozen = json.loads(gzip.decompress(frozen_bytes))
    assert sha(_canonical(frozen)) == FROZEN_PAYLOAD_SHA256
    theater = identity.theater()
    readers = Inputs(theater)
    tiles, assets = extract_tiles(theater)
    cases = []
    for index, stage in enumerate(STAGES):
        print('Preparing independent native boundary:', stage, flush=True)
        machine = RestoreProbe(readers, theater, tiles, index + 1)
        assert not first_difference(frozen['initial'], normalized(machine.initial))
        try:
            machine.run()  # Exact frozen owner driver; no copied primitive dispatch loop.
        except AtBoundary:
            pass
        else:
            raise AssertionError('Selected original primitive boundary was not reached')
        machine.capture_enabled = False
        assert len(machine.primitive_calls) == index + 1
        frozen_stage = frozen['stages'][index]
        assert machine.primitive_calls[-1]['returned_low_byte'] == frozen_stage['returned_low_byte']
        assert not first_difference(frozen_stage['trace'], normalized(machine.trace))
        before = machine.state()
        assert not first_difference(frozen_stage['state'], normalized(before)), first_difference(
            frozen_stage['state'], normalized(before))
        assert not machine.pending and not machine.range_pending
        machine.activity = 'restore_rebuild'
        machine.trace.clear()
        machine.writes.clear()
        counters_before = machine.counters.copy()
        frame_before = sr.i32(machine.uc, 0xA8ED84)
        returned_eax = machine.call(0x581F50, count=240000000)
        frame_after = sr.i32(machine.uc, 0xA8ED84)
        after = machine.state()
        unchanged = {key: before[key] == after[key] for key in (
            'rng', 'navigation', 'cells', 'movement_admissions')}
        assert all(unchanged.values()), unchanged
        assert frame_before == frame_after
        assert [row['level'] for row in machine.rebuild_trace
                if row['entry'] == 0x581F90] == [2, 1, 0]
        assert [row['level'] for row in machine.rebuild_trace
                if row['entry'] == 0x588D60] == [2, 1, 0]
        assert sum(row['entry'] == 0x42C1C0 for row in machine.rebuild_trace) == 1
        assert sha(bytes(machine.uc.mem_read(0x401000, 0x3E0000))) == machine.code_hash
        cases.append(dict(stage=stage, before=before, after=after,
                          frozen_boundary_equal=True,
                          primitive_calls=machine.primitive_calls,
                          rebuild_trace=machine.rebuild_trace,
                          native_counters_delta=dict(machine.counters - counters_before),
                          returned_eax=returned_eax, return_contract='void; EAX recorded mechanically',
                          frame_before=frame_before, frame_after=frame_after,
                          unchanged=unchanged))
        print(stage, 'rebuild records', [len(g['records']) for g in before['graphs']],
              '->', [len(g['records']) for g in after['graphs']], flush=True)
        del machine
        gc.collect()
    return dict(schema=1, frozen_navigation_file=FROZEN.name,
                frozen_navigation_sha256=FROZEN_SHA256,
                frozen_navigation_payload_sha256=FROZEN_PAYLOAD_SHA256,
                source_evidence_sha256=sha(SOURCE_EVIDENCE.read_bytes()),
                native_inputs=readers.snapshot(), assets=assets,
                native_size=frozen['case']['size'], cases=cases)


def metadata():
    sources = {}
    for module in list(sys.modules.values()):
        if name := getattr(module, '__file__', None):
            path = Path(name).resolve()
            if path.suffix == '.py' and path.is_relative_to(ROOT / 'tools'):
                sources[path.relative_to(ROOT).as_posix()] = sha(path.read_bytes())
    result = provenance(
        scope=__doc__,
        assumptions=[
            'A fresh original full-map Navigation machine is created independently for each boundary. Its initial state and selected original damage/collapse/repair state and trace must equal the frozen Navigation packet exactly before rebuild.',
            'The existing Navigation.run driver executes preparations; the observer stops only after the chosen complete native57CCF0 or573540 returns. No bridge or graph algorithm is reproduced in this driver.',
            'Original LoadContent67E730 calls581F50 at67E8CD after MouseLoad and object restoration. This witness executes that complete581F50 wrapper on the frozen live source facts; it does not emulate SaveGame, MouseLoad or pointer swizzling.',
            'Native MouseLoad preserves Map+68 class/height/base-ID values and all13 movement-row arrays; this selected bridge boundary has identical saved-source facts in production. The hierarchy record vectors, ID planes and adjacency buckets are rebuilt.',
            'The selected stock span has no structural bridge/Tube records. Full post-load bridge record/dummy reconstruction and actor path invalidation are outside this bounded witness.',
        ],
        substitutions=[
            'Reuse all physical reader, cell/TMP/Terrain preparation, bounded allocation/free, display, shroud and waterfall animation seams declared by frozen Navigation. No new native gameplay seam is introduced during581F50.',
            'The successful LoadContent callsite is instruction-established and byte-checked against the pinned binary. This is a hierarchy-rebuild execution comparison, not a full native save/load run.',
        ],
        entry_points={'load_content': 0x67E730, 'load_hierarchy_call': 0x67E8CD,
                      'rebuild_all': 0x581F50, 'clear_vector': 0x588D60,
                      'build_level': 0x581F90, 'refresh_scratch': 0x42C1C0})
    result.update(harness_sha256=sha(Path(__file__).read_bytes()),
                  sources=dict(sorted(sources.items())))
    return result


def publish(data=generate, argv=None):
    """Reuse the compressed publication owner and the frozen payload guard."""
    packet_io.finish_vectors(data, HERE / 'anytown_navigation_restore.json.gz',
                             provenance=metadata, argv=argv,
                             promotion_path=HERE / 'anytown_navigation_restore.promotion.json')


def compare_restored_prefix(prefix):
    """Compare complete post-load graphs and base source facts for all three states."""
    native_path = HERE / 'anytown_navigation_restore.json.gz'
    native = packet_io.read_result(native_path)
    rows = []
    for case, suffix in zip(native['cases'], ('damaged', 'collapsed', 'repaired'), strict=True):
        path = Path(str(prefix) + '.restored_' + suffix + '.json')
        exported = json.loads(path.read_bytes())
        actual = exported.get('navigation', exported)
        expected = case['after']
        checks = {}
        for key, value in expected['navigation'].items():
            found = actual.get(key, actual.get('rust', {}).get(key))
            difference = first_difference(value, found)
            checks['navigation.' + key] = dict(equal=difference is None,
                                               first_difference=difference)
        for level, graph in enumerate(expected['graphs']):
            for key, value in graph.items():
                difference = first_difference(value, actual['graphs'][level][key])
                checks[f'graphs.{level}.{key}'] = dict(equal=difference is None,
                                                      first_difference=difference)
        difference = first_difference(native['native_size'], actual['native_size'])
        checks['native_size'] = dict(equal=difference is None, first_difference=difference)
        rows.append(dict(stage=case['stage'], production_file=path.name,
                         production_sha256=sha(path.read_bytes()), checks=checks,
                         native_graph_record_counts=[len(g['records']) for g in expected['graphs']],
                         production_graph_record_counts=[len(g['records']) for g in actual['graphs']],
                         all_compared_equal=all(row['equal'] for row in checks.values())))
    return dict(schema=1, native_file=native_path.name,
                native_sha256=sha(native_path.read_bytes()),
                native_payload_sha256=sha(_canonical(native)),
                scope='Complete class/height/base-ID planes,13 movement rows,base zone count,native map size and all ordered hierarchy IDs/padding/records/edges for three restored bridge states. No full native save/load or actor-world comparison.',
                states=rows, all_compared_equal=all(row['all_compared_equal'] for row in rows))


def main():
    import argparse
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument('--compare-prefix', type=Path)
    parser.add_argument('--comparison-output', type=Path)
    args, remaining = parser.parse_known_args()
    if args.compare_prefix is None:
        if args.comparison_output is not None:
            parser.error('--comparison-output requires --compare-prefix')
        publish(argv=remaining)
        return
    if remaining:
        parser.error('Unexpected comparison arguments: ' + ' '.join(remaining))
    result = compare_restored_prefix(args.compare_prefix)
    text = json.dumps(result, indent=2) + '\n'
    if args.comparison_output is not None:
        args.comparison_output.write_text(text)
    else:
        print(text, end='')
    assert result['all_compared_equal'], 'Native restored navigation comparison differs'


if __name__ == '__main__':
    main()

