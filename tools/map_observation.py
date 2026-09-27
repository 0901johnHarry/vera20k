"""Run and check a chosen-map production observation; never certify native parity.

The Rust map_observation profile and capture manifest own the runtime schema.
This wrapper binds their receipts to immutable inputs and actual frame bytes.
"""
from __future__ import annotations

import argparse
from pathlib import Path
import sys
from typing import Any, Mapping

from tools.cargo_run import resolve_binary
from tools.child_process import run_child
from tools.tactical_certification.core import (
    FileSnapshot, ValidationError, assert_snapshot_unchanged,
    create_directory_exclusive, load_json_file, require_directory, require_int,
    require_object, require_regular_file, require_sha256, require_string,
    require_value, sha256_bytes, utc_now, write_bytes_exclusive, write_json_exclusive,
)
from tools.tactical_certification.profile import load_contract, reject_denied_environment

ROOT = Path(__file__).resolve().parents[1]


def _integer(document: Mapping[str, Any], key: str, minimum: int = 0) -> int:
    value = require_int(document.get(key), key)
    if value < minimum:
        raise ValidationError(f'{key} must be at least {minimum}')
    return value


def _identity(value: Any, snapshot: FileSnapshot, label: str) -> None:
    document = require_object(value, label)
    for key, expected in snapshot.public_identity().items():
        require_value(document.get(key), expected, f'{label}.{key}')


def validate_capture(directory: Path, profile: Mapping[str, Any],
                     snapshots: Mapping[str, FileSnapshot]) -> dict[str, Any]:
    """Check the completed child transaction against this invocation's inputs."""
    require_directory(directory, 'child output')
    manifest_snapshot, manifest = load_json_file(directory / 'capture.json', 'capture manifest')
    require_value(manifest.get('schema_version'), 'vera20k.map-observation.v1', 'schema_version')
    if manifest.get('status') != 'COMPLETE':
        raise ValidationError(f'child did not complete: {manifest.get("failure", manifest.get("status"))}')
    if {path.name for path in directory.iterdir()} != {'capture.json', 'frame.bgra'}:
        raise ValidationError('child output must contain exactly capture.json and frame.bgra')
    for key in ('native_comparator', 'parity_certification'):
        require_value(manifest.get(key), 'NONE', key)
    child_profile = require_object(manifest.get('profile'), 'profile')
    require_value(child_profile.get('sha256'), snapshots['profile'].sha256, 'profile.sha256')
    require_value(child_profile.get('request'), dict(profile), 'profile.request')
    inputs = require_object(manifest.get('inputs'), 'inputs')
    contract = require_object(manifest.get('contract'), 'contract')
    require_value(contract.get('sha256'), snapshots['contract'].sha256, 'contract.sha256')
    for name in ('config', 'executable'):
        _identity(inputs.get(name), snapshots[name], f'inputs.{name}')

    ticks = _integer(profile, 'ticks')
    require_value(manifest.get('exact_step_count'), ticks, 'exact_step_count')
    initial = require_object(manifest.get('initial'), 'initial')
    final = require_object(manifest.get('final'), 'final')
    for name in ('simulation_tick', 'binary_frame', 'total_simulation_ms'):
        require_value(initial.get(name), 0, f'initial.{name}')
        _integer(final, name)
    for state in (initial, final):
        _integer(state, 'deterministic_state_hash')
    for name in ('simulation_tick', 'binary_frame'):
        require_value(final.get(name), ticks, f'final.{name}')
    if ticks == 0:
        require_value(dict(final), dict(initial), 'zero-step final state')
        for name in ('first_exact_step', 'last_exact_step'):
            require_value(manifest.get(name), None, name)
    else:
        if final['total_simulation_ms'] <= initial['total_simulation_ms']:
            raise ValidationError('simulation time did not advance')
        for name, before in (('first_exact_step', 0), ('last_exact_step', ticks - 1)):
            receipt = require_object(manifest.get(name), name)
            for key in ('tick', 'binary_frame'):
                require_value(receipt.get(f'{key}_before'), before, f'{name}.{key}_before')
                require_value(receipt.get(f'{key}_after'), before + 1, f'{name}.{key}_after')

    startup = require_object(manifest.get('startup'), 'startup')
    for key, expected in (('seed', profile.get('seed')), ('seed_source', 'Controlled'),
                          ('seed_authority_certifying', True),
                          ('classification', 'AcceptedExplicitFixedBattle')):
        require_value(startup.get(key), expected, f'startup.{key}')
    _integer(startup, 'correlation', 1)
    source = require_object(manifest.get('map_source'), 'map_source')
    require_sha256(source.get('source_sha256'), 'map_source.source_sha256')
    _integer(source, 'payload_len', 1)
    if source.get('kind') == 'loose':
        path = require_string(source.get('path'), 'map_source.path')
        if not path:
            raise ValidationError('empty map source path')
    elif source.get('kind') == 'mix':
        for name in ('logical_name', 'source_archive'):
            if not require_string(source.get(name), f'map_source.{name}'):
                raise ValidationError(f'empty map_source.{name}')
        require_int(source.get('entry_id'), 'map_source.entry_id')
    else:
        raise ValidationError('map source must be loaded loose or MIX bytes')
    lifecycle = require_object(manifest.get('lifecycle'), 'lifecycle')
    for key, expected in (('window_hidden', True), ('window_focused', False),
                          ('focus_violations', 0), ('input_violations', 0)):
        require_value(lifecycle.get(key), expected, f'lifecycle.{key}')
    render = require_object(manifest.get('render'), 'render')
    for key in ('ready', 'sidebar_view_present'):
        require_value(render.get(key), True, f'render.{key}')
    width, height = _integer(profile, 'width', 1), _integer(profile, 'height', 1)
    for key in ('internal_extent', 'surface_extent'):
        require_value(render.get(key), [width, height], f'render.{key}')
    frame = require_object(manifest.get('frame'), 'frame')
    frame_snapshot = require_regular_file(directory / 'frame.bgra', 'frame',
                                          exact_length=width * height * 4)
    for key, expected in (('file_name', 'frame.bgra'), ('width', width), ('height', height),
                          ('row_stride', width * 4), ('byte_length', frame_snapshot.byte_length),
                          ('sha256', frame_snapshot.sha256), ('pixel_layout', 'BGRA8')):
        require_value(frame.get(key), expected, f'frame.{key}')
    if frame.get('surface_format') not in ('Bgra8Unorm', 'Bgra8UnormSrgb'):
        raise ValidationError('unsupported frame.surface_format')
    assert_snapshot_unchanged(manifest_snapshot, 'capture manifest')
    assert_snapshot_unchanged(frame_snapshot, 'frame')
    return {'manifest': manifest_snapshot.public_identity(),
            'frame': frame_snapshot.public_identity(), 'map_source': dict(source),
            'initial': dict(initial), 'final': dict(final), 'exact_step_count': ticks}


def capture(*, profile_path: Path, contract_path: Path, output: Path,
            working_directory: Path, executable: Path | None = None) -> dict[str, Any]:
    profile_snapshot, profile = load_json_file(profile_path, 'map observation profile')
    require_value(profile.get('schema_version'), 'vera20k.map-observation-profile.v1',
                  'profile.schema_version')
    # Only wrapper resource budgets are interpreted here. Rust owns launch admission.
    timeout = _integer(profile, 'timeout_seconds', 1)
    contract = load_contract(contract_path)
    reject_denied_environment(contract)
    if timeout > contract.document['absolute_max_child_timeout_seconds']:
        raise ValidationError('profile timeout exceeds checked contract maximum')
    cwd = require_directory(working_directory, 'working directory')
    if executable is None:
        executable, _ = resolve_binary(ROOT, 'vera20k', 'release')
        if executable is None:
            raise ValidationError('no verified release vera20k; build with tools.cargo_run first')
    snapshots = {'profile': profile_snapshot, 'contract': contract.snapshot,
                 'config': require_regular_file(cwd / 'config.toml', 'config'),
                 'executable': require_regular_file(executable, 'executable')}
    run = create_directory_exclusive(output, 'observation output')
    write_bytes_exclusive(run / 'profile.json', profile_snapshot.raw)
    child_output = run / 'child-output'
    command = [str(snapshots['executable'].path), '--tactical-capture', 'map-observe-v1',
               '--profile', str(profile_snapshot.path), '--contract', str(contract.path),
               '--output', str(child_output)]
    started = utc_now()
    child = run_child(command, cwd=cwd, temporary_directory=run, timeout_seconds=timeout)
    errors = list(child.errors)
    if child.timed_out:
        errors.append('observation child timed out')
    if child.exit_status != 0:
        errors.append(f'observation child exit status: {child.exit_status}')
    for name, snapshot in snapshots.items():
        try:
            assert_snapshot_unchanged(snapshot, name)
        except ValidationError as exc:
            errors.append(str(exc))
    capture_evidence = None
    try:
        capture_evidence = validate_capture(child_output, profile, snapshots)
    except (ValidationError, OSError) as exc:
        errors.append(str(exc))
    unexpected = {path.name for path in run.iterdir()} - {'profile.json', 'child-output'}
    if unexpected:
        errors.append(f'unexpected wrapper output: {sorted(unexpected)}')
    # A child must not alter the retained request copy either.
    try:
        copy = require_regular_file(run / 'profile.json', 'profile copy')
        require_value(copy.sha256, profile_snapshot.sha256, 'profile copy sha256')
    except ValidationError as exc:
        errors.append(str(exc))
    artifacts = {}
    for name, raw in (('stdout.log', child.stdout), ('stderr.log', child.stderr)):
        write_bytes_exclusive(run / name, raw)
        artifacts[name] = {'byte_length': len(raw), 'sha256': sha256_bytes(raw)}
    report = {'schema_version': 'vera20k.map-observation-run.v1',
              'status': 'VALID' if not errors else 'INVALID', 'errors': errors,
              'started_at_utc': started, 'finished_at_utc': utc_now(),
              'command': command, 'working_directory': str(cwd),
              'inputs': {name: value.public_identity() for name, value in snapshots.items()},
              'child': {'pid': child.pid, 'exit_status': child.exit_status,
                        'timed_out': child.timed_out, 'timeout_seconds': timeout,
                        'cleanup_scope': 'exact-child-pid-only'},
              'logs': artifacts, 'capture': capture_evidence,
              'native_comparator': 'NONE', 'parity_certification': 'NONE'}
    write_json_exclusive(run / 'run.json', report)
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profile', type=Path, required=True)
    parser.add_argument('--contract', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--cwd', type=Path, default=Path.cwd())
    parser.add_argument('--executable', type=Path)
    args = parser.parse_args(argv)
    try:
        report = capture(profile_path=args.profile, contract_path=args.contract,
                         output=args.output, working_directory=args.cwd,
                         executable=args.executable)
    except (OSError, ValueError) as exc:
        print(f'map observation: {exc}', file=sys.stderr)
        return 2
    print(f'{report["status"]}: {args.output / "run.json"}')
    return 0 if report['status'] == 'VALID' else 1


if __name__ == '__main__':
    raise SystemExit(main())
