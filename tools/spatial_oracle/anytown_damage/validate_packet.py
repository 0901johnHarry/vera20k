"""Replay twelve bounded Anytown witnesses and check their portable manifest."""
import argparse
import importlib
import json
import os
from pathlib import Path
import subprocess
import sys

from tools.native_oracle import NATIVE_SHA256, _canonical, first_difference
from tools.spatial_oracle.shrapnel_repair import map_facts, packet_io
from tools.spatial_oracle.shrapnel_repair.validate_packet import sha

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
PACKAGE = 'tools.spatial_oracle.anytown_damage'
RUNNERS = ('next_family_native', 'anytown_geometry', 'anytown_resident',
           'anytown_admission', 'anytown_occupants', 'mtnk_attack', 'navigation', 'hut', 'hut_cursor',
           'mission', 'mission_counter', 'mission_interleaving')
PROJECTIONS = ('hut_test_vectors', 'mission_test_vectors')
PROMOTIONS = ('promotion.json', 'navigation_promotion.json', 'hut_promotion.json',
              'hut_cursor_promotion.json', 'mission_promotion.json')
FROZEN_RECEIPTS = ('navigation_receipt.json', 'hut_receipt.json', 'hut_cursor_receipt.json',
                   'mission_receipt.json')


def inputs(root_key='roots'):
    # Default return stays identical for the original six witnesses and hut.
    # Whole-map navigation additionally needs the separate physical MIX roots.
    roots = json.loads((HERE / 'retail_manifest.json').read_bytes())[root_key]
    actual = {}
    for variable, declared in roots.items():
        location = os.environ.get(variable, declared.get('default_directory'))
        assert location, f'Set {variable} for the frozen {root_key} inputs'
        directory = Path(location)
        actual[variable] = {}
        for name, expected in declared['files'].items():
            path = directory / name
            if expected.get('absent'):
                assert not path.exists(), f'Frozen input requires absent {variable}/{name}'
                actual[variable][name] = {'absent': True}
                continue
            assert path.is_file(), f'Extract {name} into {variable}'
            found = {'bytes': path.stat().st_size, 'sha256': sha(path)}
            assert all(found[k] == expected[k] for k in found), f'Retail input mismatch: {name}'
            actual[variable][name] = found
    return actual


def check_frozen_receipts():
    for name in FROZEN_RECEIPTS:
        receipt = json.loads((HERE / name).read_bytes())
        for path, expected in receipt['owned'].items():
            assert sha(HERE / path) == expected, f'{name}: artifact changed: {path}'
        for path, expected in receipt['imported_sources'].items():
            assert sha(ROOT / path) == expected, f'{name}: source changed: {path}'


def manifest(retail, archives):
    import capstone
    import unicorn
    for runner in RUNNERS + PROJECTIONS:
        importlib.import_module(f'{PACKAGE}.{runner}')
    owned = {p.relative_to(HERE).as_posix(): sha(p) for p in sorted(HERE.iterdir())
             if p.is_file() and p.name != 'receipt.json' and p.suffix != '.log'}
    dependencies = {}
    for module in list(sys.modules.values()):
        file = getattr(module, '__file__', None)
        if file:
            path = Path(file).resolve()
            if path.suffix == '.py' and path.is_relative_to(ROOT / 'tools') and not path.is_relative_to(HERE):
                dependencies[path.relative_to(ROOT).as_posix()] = sha(path)
    return dict(schema=1, native_sha256=NATIVE_SHA256,
                runtime=dict(python_requires='>=3.10', unicorn=unicorn.__version__,
                             capstone=capstone.__version__, lzo_version=map_facts.lib.lzo_version()),
                runners=[f'{PACKAGE}.{runner}' for runner in RUNNERS],
                projections=[f'{PACKAGE}.{runner}' for runner in PROJECTIONS], owned=owned,
                dependencies=dict(sorted(dependencies.items())), external_inputs=retail,
                archive_inputs=archives,
                validation='Separate --write and --check runs execute twelve native witnesses. '
                           'Ten compressed witnesses guard their original unprojected payload '
                           'and lexical-input projection. The v7 counter comparison guards its '
                           'frozen original projection; v8 interleaving checks original execution '
                           'against recorded production inputs. Both modes verify the frozen '
                           'navigation/hut/cursor/mission receipts and two Rust projections. '
                           'Manifest-only skips native execution and does not repeat the saved '
                           'navigation production comparison.',
                scope='Native family identity, damage/repair geometry, selected Recalc, admission, '
                      'MTNK occupants/Detach, host-scheduled FireAt/flight/impact, full physical '
                      'Anytown navigation builders/updates, bounded concrete hut sweeps and '
                      'physical concrete repair-cursor queries, original live MTNK command/Logic '
                      'continuation, counter ordering and selected v8 first-shot RNG interleaving. '
                      'Saved navigation production comparison covers loaded/first-damaged/collapsed '
                      'and repaired states. Mission evidence excludes other-world scheduling '
                      'except recorded v8 Terrain Next prefixes; no ordinary movement/head, '
                      'full hut lifecycle or whole-bridge parity claim.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument('--write', action='store_true')
    mode.add_argument('--check', action='store_true')
    parser.add_argument('--manifest-only', action='store_true',
                        help='check or refresh source/input/artifact hashes without native replay')
    args = parser.parse_args()
    assert sys.version_info >= (3, 10)
    retail = inputs()
    archives = inputs('archive_roots')
    flag = '--write' if args.write else '--check'
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE='1', PYTHONPATH=str(ROOT))
    if not args.manifest_only:
        for runner in RUNNERS:
            print(f'{runner} {flag}', flush=True)
            subprocess.run([sys.executable, '-m', f'{PACKAGE}.{runner}', flag],
                           cwd=ROOT, env=env, check=True)
    for runner in PROJECTIONS:
        subprocess.run([sys.executable, '-m', f'{PACKAGE}.{runner}',
                        '--check' if args.manifest_only else flag], cwd=ROOT, env=env, check=True)
    for source in PROMOTIONS:
        promotion = json.loads((HERE / source).read_bytes())['results']
        for name, expected in promotion.items():
            payload = _canonical(packet_io.read_result(HERE / (name + '.gz')))
            assert packet_io.digest(payload) == expected['published_payload_sha256'], name
    check_frozen_receipts()
    actual = manifest(retail, archives)
    path = HERE / 'receipt.json'
    if args.write:
        path.write_text(json.dumps(actual, indent=2, sort_keys=True) + '\n', encoding='utf-8')
        print('WROTE receipt; run an independent --check before citing the packet.', flush=True)
    else:
        difference = first_difference(json.loads(path.read_bytes()), actual)
        assert difference is None, f'Source/input/artifact manifest changed: {difference}'
        label = 'manifest-only' if args.manifest_only else 'twelve native outputs and manifest'
        print(f'PASS {label}; no files written.', flush=True)


if __name__ == '__main__':
    main()
