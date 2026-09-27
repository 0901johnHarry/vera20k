"""Failure and ownership contracts for the build runner (no game/retail required)."""
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from tools import cargo_run


class CargoRunTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)
        subprocess.run(['git', '-C', str(self.root), 'config', 'user.name', 'Test'], check=True)
        subprocess.run(['git', '-C', str(self.root), 'config', 'user.email', 'test@example.invalid'], check=True)
        (self.root / '.gitignore').write_text('/target/\n')
        (self.root / 'source.rs').write_text('first')
        subprocess.run(['git', '-C', str(self.root), 'add', '.'], check=True)
        subprocess.run(['git', '-C', str(self.root), 'commit', '-qm', 'initial'], check=True)
        self.env = patch.dict(os.environ)
        self.env.start()
        self.addCleanup(self.env.stop)
        os.environ.pop('CARGO_TARGET_DIR', None)

    def test_fingerprint_covers_dirty_deleted_and_untracked_sources(self):
        first = cargo_run.source_identity(self.root)
        (self.root / 'source.rs').write_text('second')
        second = cargo_run.source_identity(self.root)
        self.assertNotEqual(first['source_sha256'], second['source_sha256'])
        (self.root / 'source.rs').unlink()
        third = cargo_run.source_identity(self.root)
        self.assertNotEqual(second['source_sha256'], third['source_sha256'])
        (self.root / 'new.rs').write_text('new')
        self.assertNotEqual(third['source_sha256'], cargo_run.source_identity(self.root)['source_sha256'])

    def test_lock_excludes_other_process_and_releases_after_owner_exit(self):
        lock = self.root / 'build.lock'
        code = "from pathlib import Path; from tools.cargo_run import build_lock; " \
               "\nwith build_lock(Path(__import__('sys').argv[1]), 0): print('acquired')"
        with patch.object(cargo_run, 'build_processes', return_value=[]):
            with cargo_run.build_lock(lock, 0):
                result = subprocess.run([sys.executable, '-c', code, str(lock)], capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('Timed out', result.stderr)
            with cargo_run.build_lock(lock, 0):
                pass

    def test_unwrapped_cargo_blocks_build_and_inspection_errors_propagate(self):
        with patch.object(cargo_run, 'build_processes', return_value=['12 cargo']):
            with self.assertRaisesRegex(TimeoutError, '12 cargo'):
                with cargo_run.build_lock(self.root / 'lock', 0):
                    self.fail('entered while another compile was active')
        with patch.object(cargo_run, 'build_processes', side_effect=OSError('ps failed')):
            with self.assertRaisesRegex(OSError, 'ps failed'):
                with cargo_run.build_lock(self.root / 'lock', 0):
                    self.fail('entered without process inspection')

    def test_invalid_target_and_label_requests_never_build(self):
        for args in (['test'], ['build', '--target-dir=/tmp/shared'],
                     ['build', '--manifest-path', 'other.toml'], ['build', '--config=x']):
            with self.subTest(args=args), self.assertRaises(ValueError):
                cargo_run.cargo_args(args, None)
        with self.assertRaisesRegex(ValueError, 'requires build'):
            cargo_run.cargo_args(['test', '--lib'], 'bad')
        with self.assertRaisesRegex(ValueError, 'Label'):
            cargo_run.run(self.root, ['build'], '../escape', 0)

    def fake_cargo(self, *, code=0, mutate=False, artifact=True):
        # Emulates Cargo's *public JSON output*, including a misleading stale
        # conventional target path. The runner must use the emitted path.
        def start(command, **kwargs):
            target = Path(kwargs['env']['CARGO_TARGET_DIR'])
            path = target / 'debug' / 'deps' / 'actual-hash'
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b'current executable')
            (self.root / 'target' / 'stale').write_bytes(b'wrong checkout')
            if mutate:
                (self.root / 'source.rs').write_text('edited during build')
            child = unittest.mock.MagicMock()
            child.__enter__.return_value = child
            message = {'reason': 'compiler-artifact', 'executable': str(path)}
            child.stdout = io.StringIO(json.dumps(message) + '\n' if artifact else '')
            child.wait.return_value = code
            return child
        return start

    def invoke(self, label='build-a', **options):
        original = subprocess.check_output
        def output(command, **kwargs):
            if command[0] in {'rustc', 'cargo'}:
                return 'test toolchain\n'
            return original(command, **kwargs)
        actual_popen = subprocess.Popen
        fake = self.fake_cargo(**options)
        def start(command, **kwargs):
            return fake(command, **kwargs) if command[0] == 'cargo' else actual_popen(command, **kwargs)
        with patch.object(cargo_run, 'build_processes', return_value=[]), \
             patch.object(cargo_run.subprocess, 'check_output', side_effect=output), \
             patch.object(cargo_run.subprocess, 'Popen', side_effect=start):
            return cargo_run.run(self.root, ['build'], label, 0)

    def test_copies_only_emitted_executable_and_refuses_overwrite(self):
        self.assertEqual(self.invoke(), 0)
        output = self.root / '.git/owned-builds/artifacts/build-a'
        manifest = json.loads((output / 'manifest.json').read_text())
        entry, = manifest['artifacts']
        self.assertEqual((output / entry['file']).read_bytes(), b'current executable')
        self.assertEqual(Path(entry['file']).name, 'actual-hash')
        self.assertEqual(entry['sha256'], hashlib.sha256(b'current executable').hexdigest())
        self.assertEqual(manifest['source']['head'], cargo_run.git(self.root, 'rev-parse', 'HEAD'))
        with self.assertRaisesRegex(ValueError, 'already exists'):
            self.invoke()

    def test_binary_discovery_uses_recorded_output_not_stale_conventional_binary(self):
        stale = self.root / 'target/release/asset'
        stale.parent.mkdir(parents=True)
        stale.write_bytes(b'stale')
        self.assertEqual(cargo_run.resolve_binary(self.root, 'asset'), (None, None))
        store, namespace = cargo_run.build_store(self.root)
        target = self.root / 'target/shared-parent/owned-worktrees' / namespace
        binary = target / 'release/asset'
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b'owned')
        cargo_run.publish_binaries(store, namespace, target, {binary})
        self.assertEqual(cargo_run.resolve_binary(self.root, 'asset'), (binary, 'release'))
        binary.write_bytes(b'overwritten')
        self.assertEqual(cargo_run.resolve_binary(self.root, 'asset'), (None, None))

    def test_failed_or_mutating_build_never_publishes_label(self):
        self.assertEqual(self.invoke(code=7), 7)
        with self.assertRaisesRegex(ValueError, 'Source changed'):
            self.invoke(mutate=True)
        self.assertFalse((self.root / '.git/owned-builds/artifacts/build-a').exists())

    def test_success_without_artifact_is_not_a_preserved_build(self):
        with self.assertRaisesRegex(ValueError, 'no executable'):
            self.invoke(artifact=False)
        self.assertFalse((self.root / '.git/owned-builds/artifacts/build-a').exists())


if __name__ == '__main__':
    unittest.main()
