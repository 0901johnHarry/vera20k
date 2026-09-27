"""Portable Anim oracle lifecycle checks; synthetic opcodes are not native goldens."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from tools import native_oracle
from tools.anim_oracle import boundary
from tools.tests.pe_fixture import pe_image


ROOT = Path(__file__).resolve().parents[2]
ENTRY = 0x42468C
# HLT succeeds at the emulator level without reaching a declared endpoint.
# EB FE exhausts the instruction budget; UD2 faults. None models Anim behavior.
FAILURES = (
    ('early stop', b'\xf4', 'Incomplete execution'),
    ('instruction budget', b'\xeb\xfe', 'Incomplete execution'),
    ('invalid instruction', b'\x0f\x0b', 'faulted'),
)


def synthetic_image(opcodes):
    """Exercise the shared PE mapping too, with deliberately unequal RVA/raw."""
    return pe_image([(ENTRY - native_oracle.IMAGE_BASE, 0x400, opcodes,
                      len(opcodes), 0x60000020)])


def absent_retail_environment():
    environment = dict(os.environ)
    environment['VERA20K_GAMEMD_EXE'] = str(ROOT / 'deliberately-absent-anim-retail.exe')
    environment.pop('RA2_DIR', None)
    environment['PYTHONDONTWRITEBYTECODE'] = '1'
    return environment


class AnimBoundaryCompletionTests(unittest.TestCase):
    def test_early_stop_budget_and_fault_never_become_native_rows(self):
        for label, opcodes, message in FAILURES:
            with self.subTest(failure=label), patch.object(
                    native_oracle, 'image_bytes', return_value=synthetic_image(opcodes)):
                with self.assertRaisesRegex(native_oracle.OracleError, message):
                    boundary.generate()

    def test_failed_write_preserves_reference_and_creates_no_sidecar(self):
        for label, opcodes, _ in FAILURES:
            for existing in (False, True):
                with self.subTest(failure=label, existing=existing), \
                     tempfile.TemporaryDirectory() as directory:
                    output = Path(directory) / 'boundary.json'
                    original = b'{"existing-reference":"must remain byte-identical"}\n'
                    if existing:
                        output.write_bytes(original)
                    with patch.object(native_oracle, 'image_bytes',
                                      return_value=synthetic_image(opcodes)):
                        with self.assertRaises(native_oracle.OracleError):
                            boundary.main(['--write', '--output', str(output)])
                    self.assertFalse(output.with_suffix('.meta.json').exists())
                    if existing:
                        self.assertEqual(output.read_bytes(), original)
                        self.assertEqual({p.name for p in output.parent.iterdir()}, {'boundary.json'})
                    else:
                        self.assertEqual(list(output.parent.iterdir()), [])

    def test_failure_and_publication_gates_survive_optimized_python(self):
        result = subprocess.run(
            [sys.executable, '-O', '-m', 'unittest',
             __name__ + '.AnimBoundaryCompletionTests.test_early_stop_budget_and_fault_never_become_native_rows',
             __name__ + '.AnimBoundaryCompletionTests.test_failed_write_preserves_reference_and_creates_no_sidecar'],
            cwd=ROOT, env=absent_retail_environment(), capture_output=True, text=True,
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


class AnimBoundaryLifecycleTests(unittest.TestCase):
    def test_import_and_help_are_inert_without_retail_in_normal_and_optimized_python(self):
        script = r'''
import importlib
import runpy
import sys
from pathlib import Path
from unittest.mock import patch
from tools import native_oracle
with patch.object(native_oracle, 'image_bytes', side_effect=RuntimeError('native bytes requested')), \
     patch.object(native_oracle, 'run_checked', side_effect=RuntimeError('native execution requested')), \
     patch.object(Path, 'write_text', side_effect=RuntimeError('unexpected text publication')), \
     patch.object(Path, 'write_bytes', side_effect=RuntimeError('unexpected binary publication')):
    if sys.argv[1] == 'import':
        importlib.import_module('tools.anim_oracle.boundary')
    else:
        sys.argv = ['tools.anim_oracle.boundary', '--help']
        runpy.run_module('tools.anim_oracle.boundary', run_name='__main__')
'''
        for optimized in (False, True):
            for operation in ('import', 'help'):
                with self.subTest(optimized=optimized, operation=operation):
                    command = [sys.executable] + (['-O'] if optimized else [])
                    result = subprocess.run(
                        command + ['-c', script, operation], cwd=ROOT,
                        env=absent_retail_environment(), capture_output=True, text=True,
                        timeout=30,
                    )
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    if operation == 'help':
                        self.assertIn('--check', result.stdout)
                        self.assertIn('--write', result.stdout)
                        self.assertIn('--output', result.stdout)


if __name__ == '__main__':
    unittest.main()
