"""Compare two exact shared PE loaders; no gamemd function is executed.

Requires Git objects for the declared commits, Unicorn, and the original binary
selected by VERA20K_GAMEMD_EXE or RA2_DIR. No extracted map/game assets are used.
"""
import argparse
import ast
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import types

import unicorn
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
SOURCE = 'tools/native_oracle.py'
VERSIONS = (
    ('original', 'd77f60318578f31bee757b3a645de267c2e45ef7',
     'dd5dbf5f3532b0eb20d761b8637c18ca8033317b67e99481f633c45dec6b2de3'),
    ('upstream', '0be4174ef6649ed110bcb330c5bae25dbf7001e9',
     'f3f58e1a15a4fd3be9e3bc2030819f079ab9155bdb08833e228c1c661445e51e'),
)
NATIVE = '1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c'
MAPPED = 'c8dbc3d1c4671589ffa5d430f1be05ff5fe19bcfb51050374b04af6ccf9f657a'
TEXT = '4cd5557a7490debc493ff965afc4483d8d2f1065f434f6b665cbb8fc4835b0cc'


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def source_tree(raw):
    tree = ast.parse(raw)
    functions = {n.name: ast.dump(n, include_attributes=False)
                 for n in tree.body if isinstance(n, ast.FunctionDef)}
    tree.body = [n for n in tree.body if not isinstance(n, ast.FunctionDef)
                 or n.name not in ('_sections', 'file_span')]
    return functions, ast.dump(tree, include_attributes=False)


def generate():
    versions, images, sections, sources, regions, modules = [], [], [], [], [], []
    for label, commit, expected in VERSIONS:
        raw = subprocess.check_output(['git', 'show', commit + ':' + SOURCE], cwd=ROOT)
        assert digest(raw) == expected, (label, 'unexpected source')
        module = types.ModuleType('loader_compatibility_' + label)
        module.__file__ = str(ROOT / SOURCE)
        exec(compile(raw, SOURCE + '@' + commit, 'exec'), module.__dict__)
        original = module.image_bytes()
        assert digest(original) == NATIVE
        uc = Uc(UC_ARCH_X86, UC_MODE_32)
        table = list(module._sections(original))
        module.load_image(uc)
        mapped = bytes(uc.mem_read(module.IMAGE_BASE, module.IMAGE_SIZE))
        assert digest(mapped) == MAPPED
        assert digest(bytes(uc.mem_read(0x401000, 0x3E0000))) == TEXT
        versions.append(dict(label=label, commit=commit, source_sha256=expected))
        images.append(mapped)
        sections.append(table)
        sources.append(raw)
        regions.append([list(region) for region in uc.mem_regions()])
        modules.append(module)
    old_functions, old_rest = source_tree(sources[0])
    new_functions, new_rest = source_tree(sources[1])
    changed = [name for name in old_functions if old_functions[name] != new_functions.get(name)]
    added = sorted(set(new_functions) - set(old_functions))
    assert changed == ['_sections'] and added == ['file_span'] and old_rest == new_rest
    assert images[0] == images[1] and sections[0] == sections[1] and regions[0] == regions[1]
    module = modules[1]
    pe = struct.unpack_from('<I', original, 0x3C)[0]
    header = pe + 24 + struct.unpack_from('<H', original, pe + 20)[0]
    section_rows = []
    for index, (rva, raw_pointer, raw_size, virtual_size, flags) in enumerate(sections[1]):
        name = original[header + index * 40:header + index * 40 + 8].split(b'\0')[0].decode('ascii')
        if raw_size:
            offset, span = module.file_span(original, module.IMAGE_BASE + rva, raw_size)
            assert offset == raw_pointer and span == original[raw_pointer:raw_pointer + raw_size]
        section_rows.append(dict(name=name, rva=rva, raw_pointer=raw_pointer,
                                 raw_size=raw_size, virtual_size=virtual_size, flags=flags,
                                 mapped_extent_sha256=digest(images[1][rva:rva + max(raw_size, virtual_size)]),
                                 file_backed_sha256=digest(original[raw_pointer:raw_pointer + raw_size])))
    return dict(schema=1, kind='Pinned PE loader compatibility; no game-function replay',
                versions=versions, native_sha256=NATIVE, original_file_size=len(original),
                unicorn_binding=unicorn.__version__, unicorn_core=list(unicorn.uc_version()),
                image_base=module.IMAGE_BASE, image_size=module.IMAGE_SIZE,
                mapped_image_sha256=MAPPED, text=dict(start=0x401000, size=0x3E0000, sha256=TEXT),
                memory_regions=regions[0], sections=section_rows,
                comparison=dict(complete_image_bytes_equal=True, section_tables_equal=True,
                                memory_regions_and_permissions_equal=True, text_bytes_equal=True,
                                changed_existing_functions=changed, added_functions=added,
                                all_other_module_AST_equal=True,
                                upstream_full_file_backed_section_spans_equal=True),
                limits=['Only Python parsing/mapping executes; no gamemd function or bridge corpus is replayed.',
                        'Equality applies to this exact valid binary; malformed-file rejection intentionally changes.',
                        'Original native executions retain their original source identity; current pins use the separate compatibility receipt.'])


def at_path(value, path):
    for key in path:
        value = value[key]
    return value


def verify_source_maps(value):
    if isinstance(value, dict):
        for key, item in value.items():
            if (key.startswith('tools/') and key.endswith('.py')
                    and isinstance(item, str) and len(item) == 64):
                assert digest((ROOT / key).read_bytes()) == item, key
            verify_source_maps(item)
    elif isinstance(value, list):
        for item in value:
            verify_source_maps(item)


def verify_pins():
    """Check current artifacts and historical facts without gameplay execution."""
    receipt = json.loads((HERE / 'loader_compatibility_receipt.json').read_bytes())
    assert digest((ROOT / SOURCE).read_bytes()) == VERSIONS[1][2]
    for group in ('proof_artifacts', 'refreshed_artifacts', 'immutable_artifacts'):
        for path, expected in receipt[group].items():
            assert digest((ROOT / path).read_bytes()) == expected, (group, path)
    for path in receipt['refreshed_artifacts']:
        verify_source_maps(json.loads((ROOT / path).read_bytes()))
    for row in receipt['original_pin_files']:
        original = subprocess.check_output(['git', 'show', receipt['pre_refresh_commit'] + ':' + row['path']], cwd=ROOT)
        assert digest(original) == row['sha256'], row['path']
        old = json.loads(original)
        current = json.loads((ROOT / row['path']).read_bytes())
        for path in row['source_pin_paths']:
            assert at_path(old, path) == VERSIONS[0][2], (row['path'], path)
            assert at_path(current, path) == VERSIONS[1][2], (row['path'], path)
    for row in receipt['preserved_historical_nodes']:
        current = json.loads((ROOT / row['file']).read_bytes())
        assert at_path(current, row['path']) == row['value'], row
    print('PASS current source/artifact pins, immutable results and original execution identities; no game-function replay')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument('--write', action='store_true')
    mode.add_argument('--check', action='store_true')
    parser.add_argument('--verify-pins', action='store_true')
    args = parser.parse_args()
    result = generate()
    output = HERE / 'loader_compatibility.json'
    if args.write:
        output.write_text(json.dumps(result, indent=2) + '\n')
        print('WROTE loader_compatibility.json; actual loader mappings are byte-identical')
    else:
        assert json.loads(output.read_bytes()) == result, 'Loader comparison changed'
        print('PASS loader_compatibility.json; 10485760 mapped bytes identical; no game-function replay')
    if args.verify_pins:
        verify_pins()


if __name__ == '__main__':
    main()
