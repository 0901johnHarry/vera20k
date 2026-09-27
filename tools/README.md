# Repository tools

Start here before writing a session-local helper. Run commands from the checkout
root with Python 3.11 or newer. Individual native tools also require Unicorn;
see [native setup](native_oracle.md). This index currently covers the shared entry
points; the exhaustive oracle/tool inventory remains tracked in issue #746.

| Job | Owner / entry point |
| --- | --- |
| Wait for builds; test, check, lint or build the current checkout; preserve A/B binaries | `python -m tools.cargo_run` (below) |
| Inspect/extract/render assets | [asset browser](asset_browser/README.md), `asset` binary |
| Run pinned native executable comparisons | [native oracle runner](native_oracle.md) |
| Compare shell captures | `python -m tools.shell_capture_diff --help` |
| Capture and certify shell routes | [shell certification](shell_certification/README.md) |
| Capture and certify tactical routes | [tactical certification](tactical_certification/README.md) |
| Check shell UI matrices | [exact shell matrix](exact_shell_ui_matrix/README.md) |
| Synchronize authoritative skill sources | `python tools/skill_sync.py --write`, then `--check` |

## Cargo ownership and labeled builds

```sh
python -m tools.cargo_run -- test -p vera20k --lib
python -m tools.cargo_run -- clippy -p vera20k --lib
python -m tools.cargo_run --label release-before -- build --locked --release -p vera20k --bin vera20k
python -m tools.cargo_run --label tests-before -- test -p vera20k --lib --no-run
python -m unittest tools.tests.test_cargo_run -v
```

`--wait-seconds 60` bounds the wait (default one hour). Ctrl-C interrupts the
runner. It does not kill other owners. Failed commands retain their exit code,
produce no label and leave compiler diagnostics visible. Source edits during a
run fail validation even if Cargo succeeds.

One kernel lock in the shared Git directory serializes cooperating worktrees.
The runner also waits for any observed `cargo` or `rustc` process on this host.
Unwrapped Cargo can still start after that observation: all sessions must use the
runner to eliminate the check/start race. This coordinates one repository's
worktrees; independent repository clones do not share the lock.

Each worktree has a separate cache under
`<CARGO_TARGET_DIR or checkout/target>/owned-worktrees/<checkout-path-hash>`.
The first build compiles dependencies into this namespace, costing time and disk;
subsequent builds reuse it. Existing shared caches are neither deleted nor trusted.
`--target-dir`, `--manifest-path`, `--config` and `--message-format` are reserved
by the runner. Other Cargo/test arguments and the caller's environment pass through.
Tests must select `--lib`. Confirm ignored `ini/`, config and retail assets in a new
worktree as usual; the runner does not copy them from someone else's checkout.

A label preserves every executable reported by **that Cargo invocation**, including
fresh cache hits, under `<git-common-dir>/owned-builds/artifacts/<label>`.
Executables retain their original basename inside numbered subdirectories.
The printed directory's `manifest.json` gives the executable filenames, SHA-256s,
checkout, commit, dirty status, combined tracked/nonignored source hash, command,
Cargo/Rust versions and common build environment overrides. Labels cannot be
replaced. This supports builds of selected binaries and `test --lib --no-run`;
labels are not attached to checks or executed tests. Run a preserved test binary
from its checkout so relative fixtures still resolve.

The manifest identifies source and executable bytes; it is **not** a hermetic
reproducibility claim. Ignored/local retail inputs, external dependencies, Cargo
user configuration and arbitrary build-script environment inputs are not sealed.
Capture and native-oracle tools retain responsibility for their own input evidence.
Debug-symbol sidecars are not copied. Keep the source checkout and its cache when
debugging a preserved binary; the executable alone suffices for ordinary A/B runs.

The asset-browser MCP uses `cargo_run.resolve_binary` to discover emitted host
release/debug executables from the per-checkout record in
`<git-common-dir>/owned-builds/latest/`. That record is updated atomically after
successful builds, and discovery verifies executable bytes. Cross-target and
custom-profile builds still support explicit paths/labels; they are not auto-run
by host tooling. Conventional target paths are not a fallback.
