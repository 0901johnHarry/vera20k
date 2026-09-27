"""Golden generators must be inspectable without emulating or rewriting references."""
import os
from pathlib import Path
import subprocess
import sys
import unittest


MODULES = ('fireat_launch', 'directed_launch', 'building_pitch', 'voxel_launch', 'arc_second_probe', 'fireat_runtime', 'load_timers')
ROOT = Path(__file__).resolve().parents[2]


class ProjectileGeneratorLifecycleTests(unittest.TestCase):
    def environment(self):
        env = dict(os.environ)
        env['VERA20K_GAMEMD_EXE'] = str(ROOT / 'deliberately-absent-retail.exe')
        env.pop('RA2_DIR', None)
        env['PYTHONDONTWRITEBYTECODE'] = '1'
        return env

    def test_imports_do_not_load_native_bytes_or_write_goldens(self):
        script = '''
import importlib
from pathlib import Path
from unittest.mock import patch
from tools import native_oracle
with patch.object(native_oracle, "image_bytes", side_effect=AssertionError("native load on import")), \\
     patch.object(Path, "write_text", side_effect=AssertionError("golden write on import")):
    for name in __import__('sys').argv[1:]:
        importlib.import_module("tools.projectile_oracle." + name)
'''
        result = subprocess.run([sys.executable, '-c', script, *MODULES], cwd=ROOT,
                                env=self.environment(), capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_cli_help_works_without_retail_files_even_under_optimized_python(self):
        for module in MODULES:
            with self.subTest(module=module):
                result = subprocess.run(
                    [sys.executable, '-O', '-m', f'tools.projectile_oracle.{module}', '--help'],
                    cwd=ROOT, env=self.environment(), capture_output=True, text=True,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn('--check', result.stdout)
                self.assertIn('--write', result.stdout)


if __name__ == '__main__':
    unittest.main()
