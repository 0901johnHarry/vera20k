# Chosen-map production observation

`python -m tools.map_observation` launches the normal production loader through
`--tactical-capture map-observe-v1`, advances the requested exact simulation steps,
and retains a hidden-window GPU readback. This is a production observation, not a
native comparator or a gameplay/pixel parity certification.

The explicit profile schema belongs to
`src/app/diagnostics/tactical_capture/map_observation.rs`. Start from
[`map_observation.example.json`](map_observation.example.json), whose launch is the
existing Rust `SkirmishLaunchSession` DTO. Change `launch.selected_map_file` to the
chosen retail map. Keep explicit countries, colors and distinct start slots, and
keep `pre_fill_house_roster` consistent with the opponents. Rust validates launch
admission; Python does not maintain a second launch parser or synthesize defaults.

Build through the shared Cargo owner, then run from the checkout with an existing
`config.toml` pointing at retail assets. Set `graphics.upscale = false` for the
native-resolution capture. All supplied paths must be absolute without symlink
ancestors; use canonical temporary paths on macOS. The output parent must exist,
and the output directory itself must not exist.

```sh
python -m tools.cargo_run -- build -p vera20k --release --bin vera20k
env -u RA2_DIR python -m tools.map_observation \
  --profile /absolute/checkout/tools/map_observation.example.json \
  --contract /absolute/checkout/src/app/diagnostics/tactical_capture/contract.v2.json \
  --cwd /absolute/checkout \
  --output /absolute/evidence/new-map-observation
```

The default executable is the unchanged release `vera20k` recorded for this
checkout by `tools.cargo_run`; it never guesses a target directory. Recorded byte
identity does not establish source freshness: build the intended revision first.
`--executable /absolute/path/to/vera20k` selects an explicit binary instead.
The supplied contract must match the repository contract bytes. Every environment
variable in its denylist must be absent, even if set to an empty string. The
wrapper reports denied variables and does not silently change the environment.
After sourcing the native development environment, explicitly remove `RA2_DIR`
for this command as above; asset loading uses the working directory's config.

A new bundle contains `profile.json`, `stdout.log`, `stderr.log`, `run.json` and
the child's atomically published `child-output/{capture.json,frame.bgra}`.
`run.json` records exact input hashes, command, child PID/status, timeout, receipt
validation and capture artifact identities. A valid observation requires unchanged
profile/config/executable/contract files, matching profile and contract receipts,
a v2 child manifest with resident UnitAtlas statistics, zero initial tick/frame/time,
the requested final tick/frame and endpoint step
receipts, a loaded loose/MIX map digest, hidden unfocused rendering without input
violations, and correctly sized/hashed BGRA bytes. Simulation time must advance
for nonzero steps; its scheduling formula remains owned by Rust. Zero steps must
retain the initial fingerprint. Map hashes attest the bytes reported consumed by
the loader; this wrapper does not independently extract MIX entries or reimplement
the loader. Compare deterministic fingerprints separately from presentation pixels.

The shared `tools.child_process` owner bounds spawn/wait/cleanup and collects
finite output snapshots. On timeout it kills only its exact child. Existing
outputs are never overwritten. A failed child, failed manifest, missing artifact,
changed input or invalid receipt produces an invalid report when publication is
possible. Exit codes: `0` valid observation, `1` retained invalid observation,
`2` invalid inputs or publication failure. This tool never claims native parity.

Portable validation:

```sh
python -m unittest tools.tests.test_map_observation tools.tests.test_child_process
```

The saved [validation receipt](map_observation.validation.json) records the checked
release binary and source snapshot, repeated MIX-map state, loose-map and zero-step
captures, missing-map diagnostics, and the explicit coverage limits. It is a run
summary, not a native oracle golden.

## Resident unit-atlas measurement

The final rendered frame records `render.unit_atlas` in the v2 child manifest;
the validated wrapper retains it as `capture.unit_atlas` in `run.json`. The
`UnitAtlas` owner reads actual wgpu texture descriptors and resident entries.
It reports resident sprite count, the last actual build's rasterized sprite count,
and each page's extent, format, dimensions, mip/sample counts and texel payload
bytes, plus their total. Page count is the length of `pages`.

Pages are currently single-layer, single-mip, single-sample D2 `R8Uint`, so the
payload is width × height bytes, including unused page space. Unsupported
descriptors fail observation rather than retaining stale byte arithmetic. The
last-build count includes rasterized shadow sprites, can differ from admitted
resident entries, and stays unchanged on a no-op refresh.

This is resident UnitAtlas texture payload for the captured scenario. It excludes
CPU caches, `VxlPoseFrameCache`, `VxlSlopeTransitionCache`, other atlases, palettes,
driver overhead and peak allocation; it does not establish 30-player saturation.
Statistics are captured with the final render evidence before readback and kept
outside deterministic simulation fingerprints. GPU allocation may differ across
adapters even for identical simulation state.

This replaces the retired `measure-atlas` binary, which estimated a hardcoded
roster and tile sizes without constructing an atlas. The unused
`bridge-oracle-compare` binary was also retired: its trace schema had no repository
producer, and its five bin-only tests covered that abandoned comparison schema,
not active bridge behavior. Current native bridge comparison owners remain in
[`anytown_damage`](spatial_oracle/anytown_damage/README.md) and
[`shrapnel_damage`](spatial_oracle/shrapnel_damage/navigation.md); their native
outputs and Rust regression witnesses are preserved.

For a headless retail growth/no-op/fresh-pack comparison on a real GPU:

```sh
VERA20K_REQUIRE_RETAIL_INI=1 python -m tools.cargo_run -- test -p vera20k --lib --release \
  render::atlas_refresh_retail_tests::retail_atlas_refresh_costs -- --ignored --nocapture
```

Set `RA2_DIR` or provide the local config for that test. It emits the same owner's
statistics at each allocation stage; fresh and grown totals need not match.

The [atlas observation validation](atlas_observation.validation.json) records
before/after production captures and the explicit retail GPU atlas refresh test.
The older `map_observation.validation.json` remains historical v1 evidence; it
has not been retroactively given statistics or new source identities.
