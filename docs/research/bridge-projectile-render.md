# Ordinary Cannon flight across bridges

This change carries the ordinary **MTNK → 105mm → Cannon → Image=120MM** SHP
route from independently established retail inputs through retained projectile
state, live bridge geometry, atlas binding, Display ordering and body/shadow
pixels. Native comparisons, CPU/GPU correctness checks and production simulation
restore checks pass within the bounds below. The renderer remedy now passes the
previously stalled dense512 workload and all18 post-remedy GPU correctness
checks. After integration with main, all 9,552 retail library tests, Clippy,
focused asset/GPU checks, release map loading and visible save restoration pass.
The single fresh critic found no blocking defects; its two accepted cleanups
are implemented and revalidated. These results establish the bounded Cannon
render chain described here, not complete projectile rendering or the whole-bridge goal.

Previously the presentation adapter interpreted raw projectile Z leptons as a
clamped map level, registered the image as an animation effect, and submitted
projectiles in a separate pass without their native shadows. One original draw
fixture at XYZ (2688, 5248, 1040) projects to native Y 315, or VERA world Y 330 after
the existing row bias of 15; the old production helper returned Y 494. The current
adapter consumes the retained lepton coordinate and preserves the independent
body position, live shadow surface, palette and native depth decisions.

## Inputs and native evidence

All corpora pin active-retail gamemd.exe SHA-256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
The [input witness](../../tools/projectile_oracle/bridge_render_inputs.md)
executes original constructors/readers against physical lexical RULESMD,
absent LANGRULE, MPBattleMD and the exact inner Hills.mmx INI; ARTMD remains
fixed. Cached INI indexes, archive IO and allocator/TLS storage are supplied
boundaries. No VERA-interpreted scalar establishes its own native expectation.

MTNK's Primary is `105mm`, not the separately named `120mm` weapon. The original
Weapon body reader stores Speed 102 from physical `Speed=40`, before the later
7729F0 Weapon postpass; original Gravity is 6
after a constructor default of 3. Cannon resolves to one 24×24 physical 120MM.SHP
with a 4×4 frame at (10,10), twelve opaque indices and four transparent indices.
Shadow and Arcing are true; Inviso, Flat, Voxel, AnimPalette and FirersPalette
are false; inverse-Rotates is true, and all three animation bytes are zero.
The constructor-backed original frame getter therefore returns 0 and GetLayer
returns Air 3. Elite `105mmE` is not exercised by this witness.

At this render chain's merge, VERA retained the authored weapon Speed value (40), whereas native
ReadSpeed stores 102 before the Weapon postpass7729F0. That pass runs after
Weapon/Bullet/Warhead readers in ReadTypeData679A10 and recomputes ROT=0
stored Speed from Range/Gravity. The selected `ROT=0` GetSpeed arm ignores this field and
derives the compared launch speed from distance and gravity. Its joined test
passes the unmodified production value into that arm; it does not convert the
fixture to hide the broader reader/consumer residual listed below. The subsequent
[retained-speed prerequisite](../../tools/rules_oracle/weapon_speed.md) fixes the
reader and per-pass owner; guided flight remains a separate unfinished chain.

Original palette startup 52BE61..52BFCE loads and expands the physical palette
files, executes full Convert 48E740 and blitter initialization 48EBF0, and binds
ANIM.PAL to 87F6C0 and PALETTE.PAL to 87F6C4. Both have 53 shade rows. The fixture
supplies an initialized RGB565 surface/format and empty spare-capacity native
registries; it does not execute DirectDraw device negotiation. The ordinary
body consumes the latter Convert. Strict AnimPalette overrides it; the shadow
always supplies the default Convert, although its pixel operation consumes
the source-index stencil and existing destination rather than source colors.

The [draw corpus companion](../../tools/projectile_oracle/bridge_render.md)
records the exact endpoints and fixture substitutions:

| Corpus | Native behavior established | Limit of the comparison |
| --- | --- | --- |
| [bridge_render_inputs](../../tools/projectile_oracle/bridge_render_inputs.json) | Full BulletType constructor/reader/image consumer; selected Techno Primary block and full Weapon reader; Gravity; seven reader controls and twelve theater filename controls. | Techno reader prefix/admission, physical INI/archive loading, sound registry and AP warhead gameplay are not established. |
| [bridge_render_art_state](../../tools/projectile_oracle/bridge_render_art_state.md) | 22 constructor/control sequences, 52 retained layer reads, and 54 physically referenced retail projectile types through three present sources. Full original Image/ART readers, byte narrowing, Trailer allocation and physical120MM successful/failed load identity. | Original preceding D AnimType reader supplies the active retail cache boundary; complete RulesClass traversal, physical INI/archive loading, other image files and Voxel asset binding are outside the witness. |
| [bridge_render](../../tools/projectile_oracle/bridge_render.json) | 149 original Object Render 5F4B10→Bullet Draw 468090 argument captures, plus six same-object intact/collapsed/repaired structural-state captures. | Retained object, map, camera, dirty state and palette identities are prepared; final CC_Draw_Shape is an argument sink. Structural mutations are supplied, not actual collapse/repair calls. |
| [bridge_render_shape](../../tools/projectile_oracle/bridge_render_shape.json) | **40 full shape cases:** 32 at baseline 1024 and eight at the independently executed canonical 32768 baseline. Physical 120MM, original palette construction, frame access, clipping, Convert selector, plain rowwalker and actual RGB565 body/shadow leaves all execute. Z bytes remain unchanged. | Prepared 64×96 RGB565 surface, circular A/Z buffers, old color/depth and object/map state. No draw-time call is replaced, but native Display traversal, fog and whole scenes are outside the fixture. |
| [bridge_render_flight](../../tools/projectile_oracle/bridge_render_flight.json) | Native 105mm input readers, original distance/GetSpeed, launch and BulletFire, eight ordinary motion blocks and draw captures. The supplied 500-lepton shot produces launch speed 59 from the native inputs. | Source/target/FLH admission, collision commits, Display/map registration and cell mutations are supplied. The final shape call is a sink; this is not a whole FireAt/AI/frame comparison. |
| [bridge_render_pixels](../../tools/projectile_oracle/bridge_render_pixels.json) | 400 signed-depth/transparency/repeat cases, four multi-pixel stencils, and both native shadow leaves over all 65536 destination RGB565 words. | Synthetic body color and prepared leaf inputs. Plain/RLE leaves are covered; the physical complete shape witness is plain 120MM, not an exhaustive RLE projectile rowwalker comparison. |

The shape ABI is significant: CC_Draw_Shape 4AED70 receives destination surface
in ECX and Convert in EDX. The tracked recorder corrects an earlier research
prototype that confused them. Ordinary body flags are 0x2E00; shadow flags 0x2601.
Both use gradient 0 and brightness 1000. Pixel admission is signed candidate
`<` zero-extended stored u16 Z. Equality rejects. Neither body nor shadow
writes Z. A passing repeated shadow halves destination RGB565 repeatedly with
`(word >> 1) & 0x7BEF`; source index zero preserves both attachments.

## State and production owners

[`native_processing::ProcessedType`](../../src/rules/native_processing.rs) owns
retained Bullet ART state across source passes and registry handoff. Object's
previous-default Image25 read precedes Bullet's empty-default read. Theater,
NewTheater, Voxel, inverse Rotates, Flat, animation bytes/palette, Trailer and
SpawnDelay are projected into the runtime definition without a final-ART reread.
The current Image text is separate from the last actual image-load attempt:
constructor text alone cannot admit a shape; visible omission clears it, whereas
Inviso can preserve the base-prefix load. Retained Inviso and the descriptor are
included in configuration hash version8. The [reader companion](../../tools/projectile_oracle/bridge_render_art_state.md)
records original cache lifetime, per-field identities and the production tests.

[`Projectile` and `ProjectileVisualState`](../../src/sim/projectile.rs) remain
the owners of flight coordinates, velocity, animation bytes and lifecycle.
The existing frame getter now observes the retained inverse-Rotates type byte;
the runtime animation override still wins. Cannon needs no facing calculation.
The synthetic rotating-frame samples do not certify every custom heading.
No new projectile timer, RNG stream or persistent presentation coordinate is
introduced. The selected render chain has no gameplay RNG draws, timer writes
or detach calls; existing firing, retirement and snapshot owners retain those
responsibilities.

The simulation's five retained
[`DisplayLayers`](../../src/sim/world/display_layers.rs) remain the registration
and order authority. Flat selects Surface 1; ordinary Cannon selects Air 3.
Logical retirement removes Display membership even while storage awaits the
common deletion drain. The frame-local `NativeDisplayOrder` is derived from
those vectors; it neither registers objects nor queries a new sort key.
Ground's existing retained sorting and persistence are preserved.

[`projectiles.rs`](../../src/app/presentation/instances/projectiles.rs) is the
read-only production adapter. It admits a projectile through retained Display
membership, consumes its exact frame and committed XYZ, applies the padded
projection gate, and emits one parent containing shadow then body. The adapter
uses `absolute_leptons_to_screen` and the existing AdjustForZ owner. It probes
the live resolved Cell level/slope and structural flag 0x100 without invoking
the gameplay probe that stamps the shared Cell dummy.

Native ground-relative height subtracts ground Z and the independent OnBridge 416
term. For !OnBridge over a live structural bridge, height ≥ 416 raises the shadow
surface by 416 and subtracts 416 from shadow height. Shadow is emitted only when
the remaining height is positive and the type permits it. Body Z-adjust is
−30−AdjustForZ(rawZ); shadow is −10−AdjustForZ(surfaceZ). These terms remain
separate when bridge flags change beneath an already flying projectile.

[`SpriteAtlas`](../../src/render/sprite_atlas.rs) owns projectile image binding
and cached source pixels. Keys use ProjectileType identity, full physical frame
number and explicit Bullet palette context. Bullet default PALETTE.PAL and
strict ANIM.PAL contexts do not consult AnimType AltPalette or Image aliases.
The image resolver uses the original selected filename/theater/G-retry rules
with the AlternateArcticArt prefix false (unset across the selected retail Bullet
inputs); the generic Snow-specific Image mutation is outside this claim. It
does not apply animation shadow-half counts or XDrawOffset/YDrawOffset.
Frame lookup neither wraps nor substitutes a facing frame. Original source
indices accompany RGBA in the existing atlas pages and growth path, so body and
shadow share exactly the same transparency stencil. Ordinary palette resolution
uses `PaletteLight::plain(53,1000)`, not precomposed color or a LightConvert hue.

[`draw_plan_lowering`](../../src/app/presentation/render/draw_plan_lowering.rs)
now lowers all five retained object layers. Represented SHP, VXL, AnimClass and
Bullet parents preserve their relative slots and contiguous internal pieces;
atlas family/page and floating depth never become ordering keys. Surface
precedes Ground, Air follows Ground and Top remains distinct. The ordinary
game replay uses these same lowered runs through
[`draw_native_object_pass`](../../src/app/presentation/render/merge_passes.rs).
Particle/Wave family residuals below prevent a claim that every Air consumer
has been migrated.

The existing [`TerrainDrawRenderer`](../../src/render/terrain_draw.rs) owns
both destination-edit policies and atlas selection. ReadWrite Terrain pieces
retain their dependency waves. The renderer remedy collects each maximal
ReadOnly interval, bounded by ordinary-draw and ReadWrite fences, into four
passes per command chunk and row band: snapshot/reset, last admitted body,
later admitted shadows, then final destination resolution. It uses the same
source-index admission and signed native Z comparison; Bullet never writes Z.
The exact reduction and submission bounds are described in the performance
section below. Post-remedy GPU correctness and stress checks pass within their
recorded fixtures, followed by the release/visible checks below. There is no separate
Bullet framebuffer or competing Z/order authority.

The migration removes the obsolete clamped-level projectile projection helper,
generic animation-effect registration for projectile images, residual
projectile emission/pass, fallback frame modulo and upper-object family/page
submission paths replaced by retained layer lowering. Shared particle and Wave
paths remain explicitly bounded rather than being represented as migrated.

## Graphics API and output constraints

Cargo.lock pins wgpu 27.0.1, core 27.0.3, hal 27.0.4 and types 27.0.1. No dependency
change is required. The following claims were checked against the matching
installed published crate sources, rather than assuming a newer API:

- [`DepthStencilState`](https://docs.rs/wgpu/27.0.1/wgpu/struct.DepthStencilState.html),
  `wgpu-types-27.0.1/src/lib.rs:4803`: depth-write enablement is independent of
  comparison. The Bullet pipeline disables writes and uses Always for hardware
  comparison; the shader compares signed native candidates before any u16 store
  conversion. Hardware Less on a wrapped candidate would not be equivalent.
- [`TextureViewDescriptor`](https://docs.rs/wgpu/27.0.1/wgpu/type.TextureViewDescriptor.html),
  types source line 6242: alternate formats must be allowed and view usage must be a
  subset. Existing non-sRGB views preserve encoded color bytes for RGB565
  destination edits; sampled linear sRGB values cannot be packed by bit shifts.
- [`wgpu-core 27.0.3 render source`](https://docs.rs/crate/wgpu-core/27.0.3/source/src/command/render.rs),
  line 1148: attachment/resource sharing requires read-only depth/stencil and
  the corresponding downlevel capability. Snapshot passes do not attach live Z;
  edit passes reload the live attachments.
- [`wgpu-core 27.0.3 transfer source`](https://docs.rs/crate/wgpu-core/27.0.3/source/src/command/transfer.rs),
  line 500: depth/stencil copies cannot select partial XY extents. The renderer
  retains scissored render snapshots into scratch rather than adding a
  full-target depth copy for every shell.

## Validation receipts by candidate

Native-established behavior, Rust regression results, GPU comparisons and
ordinary application output are separate evidence levels. Passing a reader or
leaf comparison does not establish the composed production result.

The focused candidate is `/tmp/bridge-projectile-libtests`, SHA-256
`b5ab342e9e77cf9d4b0fcf84d80e33d955b98b81c541533f9e560a3ebb6851a6`,
on base `789f48d550186f7b1738a94ea827029176167db5`. The CPU groups below passed
3 geometry/live-state/flight tests, 3 retained ART tests, 6 atlas tests (one
retail-archive test ignored), and 8 retained-layer tests. All 15 GPU checks
passed on Apple M4 / Metal: 1 full-shape, 4 Ground replay, 3 Bullet leaf, and
7 existing Terrain checks. The separately invoked physical Hills asset test
also passes: `/tmp/bridge-projectile-hills-assets-v2.log` (1 test, 1.04 seconds).
These focused receipts do not certify a complete native scene or all projectile
families.

The later full-suite/restore candidate is `/tmp/bridge-projectile-final-libtests`,
SHA-256 `894b5296f5deb5bef42c910d0d7f3c65b45f0190875e683096db8a29cf4e1728`,
on base `409e3dd8eb4e48ceb51f7518ea5d0285c95dba3b`. With retail INIs required,
its full lib suite passes 9542, fails 0 and ignores 154; Clippy exits successfully
with 954 warnings (17.33 seconds elapsed). The ignored live-chain restore test
passes separately in 176.97 seconds. These results all precede the required
renderer remedy. They cannot approve that future code or dismiss the reproduced
Metal stall. Receipt: `/tmp/bridge-projectile-final-cpu.json`.

The post-remedy executable is `/tmp/bridge-projectile-reduced-libtests`, SHA-256
`7ce9fef42209c6b5b9a053ceaab044e8ec246487bd5fc03c2921ee26d78c7eab`,
on base `57a865e8`. Compilation completed in62 seconds with115 warnings.
All18 GPU correctness tests pass: 5 Bullet leaf/ordered checks, 1 submission
check, 1 full-shape check, 4 Ground checks and 7 Terrain checks. Logs are
`/tmp/bridge-projectile-reduced-gpu-{leaves,submission,shape,ground,terrain}.log`.
The final owned CPU recompile on the same current source is retained as
`/tmp/bridge-projectile-reduced-final-libtests`, SHA-256
`aa2a24571aa76d3ff45e876c3ccae03176db75e8c4a9bda565f2ca780acd012b`, on base
`57a865e84a35cc378a3e47f9dc06c1db0903bbf4`. It passes9544 tests, fails0 and
ignores157 with retail INIs required (56.25 seconds test runtime,117.29 seconds
including compilation). Clippy exits0 with953 warnings in16.58 seconds.
Receipt: `/tmp/bridge-projectile-reduced-final-cpu.json`. Renderer source is
unchanged from the7ce9 GPU executable; the binary hash differs after the forced
recompile. GPU/timing and CPU receipts therefore retain their distinct executable
identities without claiming a new GPU run. The earlier live-restore receipt
remains explicitly tied to its pre-remedy executable.

The reviewed candidate is `/tmp/bridge-projectile-reviewed-final-libtests`,
SHA-256 `efbb13206651da858f23d795f580de1b8cf3b52a4f51e79b07507f12be2cac6a`,
on base57a865e84a35cc378a3e47f9dc06c1db0903bbf4. After the two critic cleanups,
the full required-retail suite passes9544/0/157 ignored (56.20 seconds test
runtime,85.41 seconds total); Clippy exits0 with953 warnings in16.65 seconds
elapsed. Receipt: `/tmp/bridge-projectile-reviewed-final-cpu.json`. The actual
Hills asset test passes1 in1.11 seconds and the physical40-shape comparison
passes both formats in0.60 seconds: `/tmp/bridge-projectile-reviewed-assets.log`
and `/tmp/bridge-projectile-reviewed-shape.log`. The reduction/shaders and timing
workload are unchanged, so their earlier18-check/36-sample receipts retain their
original binary identity. Code/oracle data are committed as4626c633, and main12b6f4d4
merged without conflicts as `8ade15f87907c1205c4f6a1b230dfc3762195823`.

The integrated candidate is `/tmp/bridge-projectile-integrated-final-libtests`,
SHA-256 `630ea66cd93ef2ad42fe3aa4ca55679f7743c53d8aa8d0c57139cead7cbd3c2f`.
Its required-retail suite passes9552, fails0 and ignores157 (58.87 seconds test
runtime,125.75 seconds total). Clippy exits0 with953 warnings in16.22 seconds
elapsed. `/tmp/bridge-projectile-integrated-final-cpu.json` records commands,
identity and logs. The actual retail Hills asset check passes1 in1.01 seconds;
the40 physical shapes pass both formats in0.56 seconds; the640 ordinary-fence
check passes both formats in1.23 seconds with24 submissions and exact full
color/depth readback. Receipt: `/tmp/bridge-projectile-integrated-checks.json`;
logs `/tmp/bridge-projectile-integrated-{assets,shape,submission}.log`.
Fresh integrated release/production validation also passes below; historical
release saves/captures retain their earlier executable identity.

| Check | Current receipt | Scope/artifact to record |
| --- | --- | --- |
| Native input and palette `--check` | PASS, input owner and root independently | `/tmp/bridge-projectile-root-native-{inputs,palette}-check.log`. [input sidecar](../../tools/projectile_oracle/bridge_render_inputs.meta.json); physical source/binary hashes and supplied boundaries in companion. |
| Native draw/shape/flight/pixel `--check` | PASS, recorded by native owner; root independently reproduced geometry and all 40 shapes | Corresponding JSON/meta files above. Root logs: `/tmp/bridge-projectile-root-native-{geometry,shape}-check.log`. The saved native payloads and supplied boundaries remain unchanged by the renderer remedy. |
| Retained ART/image `--check` | PASS, input owner and root independently | 22 sequences / 52 layer rows / 54 referenced retail types; `/tmp/bridge-projectile-root-native-art-check.log`. The three production ART tests pass; 39 registered production types compare against the 54 physically referenced native types. Log: `/tmp/bridge-projectile-cpu-art.log`. |
| Initial `cargo check -p vera20k` | PASS, root: 17.54 seconds | Historical initial compile; superseded for focused testing by the retained candidate above. The later full lib and same-source release compilations pass below. |
| Focused CPU geometry/retained-order checks | PASS, geometry/flight group 3 and layer group 8 | `/tmp/bridge-projectile-cpu-{geometry-flight,layers}.log`. `retained_bullet_geometry_and_read_only_live_bridge_probe_match_native`, `projectile_bridge_height_matches_original_object_projection`, `native_non_entity_lifetimes_reach_their_layers_without_family_or_page_sorting` (fixture layers 1, 2 and 3). |
| Joined launch/motion/live bridge draw regression | PASS, included in the three-test geometry/flight group | `/tmp/bridge-projectile-cpu-geometry-flight.log`. [projectile_flight_tests](../../src/app/presentation/instances/projectile_flight_tests.rs): `original_cannon_launch_motion_and_live_bridge_draw_form_one_production_chain`; preserve the native upstream/collision boundaries. |
| Production rules/atlas checks | PASS, ART 3; atlas 6 with 1 ignored | `/tmp/bridge-projectile-cpu-{art,atlas}.log`. [sprite_atlas_projectile_tests](../../src/render/sprite_atlas_projectile_tests.rs). Original reader outputs, physical source bytes, palette contexts and frame/stencil binding pass. The actual Hills archive-load test also passes when invoked separately: `/tmp/bridge-projectile-hills-assets-v2.log`. The integrated candidate also passes this asset test: `/tmp/bridge-projectile-integrated-assets.log` (1.01 seconds runtime). This does not replace fresh integrated release/visible validation. |
| Production builder→lowering→upload→replay versus 40 physical native shapes | PASS again on integrated8ade15f8, all40 cases ×2 formats | `/tmp/bridge-projectile-integrated-shape.log` (0.56 seconds runtime). [projectile_shape_gpu_tests](../../src/app/presentation/render/projectile_shape_gpu_tests.rs): `retail_bullet_atlas_geometry_and_layer_replay_match_original_shape_pixels`; Bgra8UnormSrgb and Rgba8UnormSrgb color/depth readbacks match the bounded native shapes, including clipping, canonical baseline, live bridge changes, physical atlas source and the controlled AnimPalette reader path. |
| Signed-depth/stencil/exhaustive shadow GPU checks | PASS after remedy, 3 original checks plus 2 ordered/bounded checks | `/tmp/bridge-projectile-reduced-gpu-leaves.log`. [projectile_draw_gpu_tests](../../src/render/projectile_draw_gpu_tests.rs): `production_bullet_body_shadow_matches_original_blitters`, `production_bullet_shadow_matches_every_original_destination_word`, `production_bullet_decoded_zero_runs_preserve_color_and_depth`; both Bgra/Rgba sRGB formats pass on Apple M4 / Metal. |
| Existing Ground/Terrain/mixed atlas replay regression | PASS after remedy, Ground 4 and Terrain 7 | `/tmp/bridge-projectile-reduced-gpu-{ground,terrain}.log`. [terrain_ground_gpu_tests](../../src/app/presentation/render/terrain_ground_gpu_tests.rs), including Terrain write policy and ordinary-unit scheduling fences. |
| Live Cannon, retirement and snapshot continuation | PASS, ignored retail live-chain test on the later candidate | `/tmp/bridge-projectile-live-restore.log`. An already-moved Cannon restores exact fields and all five Display vectors; two restored futures match through retirement after 9 frames. Ordinary Hills fire collapses the bridge after 121 shells at frame 7327 and releases the target. Three restored Bouncers then match all 200 state hashes through flight/expiry, final `219282f5d7a0f8dd`. This is production composition/restore proof, not a native whole-frame comparison. |
| Presentation read-only state | PASS in bounded geometry/flight fixtures | Existing assertions preserve simulation hash, shared Cell dummy and Scenario RNG around rendering; the native source/target/collision boundaries remain as recorded above. |
| Full retail lib suite and Clippy | PASS on integrated8ade15f8 | `VERA20K_REQUIRE_RETAIL_INI=1`;9552 passed,0 failed,157 ignored;58.87 seconds runtime,125.75 seconds total. Clippy exit0,953 warnings,16.22 seconds elapsed. Logs: `/tmp/bridge-projectile-integrated-final-{full-lib,clippy}.log`; integrated candidate identity above. |
| Release retail map load and visible ordinary fire | PASS within the release/UI bounds below | Exact-source app/example build, production live-flight/collapse snapshots,121-shot collapse,200-frame debris continuation, and ordinary menu/load/input with intact/broken/restored bridge captures. Individual tiny projectile pixels are established by the40 physical GPU cases, not inferred from screenshots. |
| Single fresh critic | COMPLETE, no blocking defects | Accepted fallback-only Firer atlas registration and an explicit fragment-storage capability guard. Both implemented; reviewed CPU, actual assets and40-shape checks pass. No second critic. |
| Ghidra annotations saved and read back | PASS, original five plates plus retained-reader additions | 468000 created as `BulletClass__GetAnimFrame`; 468090 renamed `BulletClass__DrawSHPOrVoxel`. Plates 4664C0, 46BEE0, 468000, 468090 and 4AED70 read back exactly after explicit save; old ABI notes preserved with correction appended. Raw 468000 bytes matched the original. Receipt: `/tmp/bridge-projectile-ghidra.json`. Additional `/tmp/bridge-projectile-ghidra-art.json` records the saved/read-back 46BEE0, 427D00 and 5F9070 plates and rename `ObjectTypeClass__LoadSHPImage` at 5F9070, with the AlternateArcticArt limit preserved. |

The native reproduction commands and extraction environment are documented in
the two tool companions. Rust/GPU commands must use `cargo test -p vera20k --lib`
and coordinate the shared build/GPU slot. Same-content production restore now
passes, and same-source release/visible checks now pass within the receipt
below. There is no matching native whole-scene capture in the current evidence set.

## Critic disposition and integration

The one fresh read-only critic ran after the completed implementation and
validation. It found no blocking defect in the bounded Cannon chain. The owner
accepted two narrow improvements:

- `register_projectile_frames` now registers only the ANIM.PAL fallback actually
  requested by the production FirersPalette path. It no longer packs unused
  per-house BulletFirer variants; its unused house-map parameter is removed.
  The explicit supplied-scheme lookup test remains a mechanism test, not a claim
  that live Bullet/House scheme authority exists. Ordinary Cannon is unchanged.
- [`gpu.rs`](../../src/render/gpu.rs) rejects adapters missing
  `FRAGMENT_WRITABLE_STORAGE` before creating the fragment-atomic pipelines,
  with an explicit initialization error instead of later pipeline validation
  failure. No backend fallback or unsupported-adapter rendering is claimed.

The reviewed full suite, Clippy, actual retail asset load and40-shape GPU checks
pass as recorded above. Newer main12b6f4d4 touches connected presentation/rules files and has now merged
without conflicts. Integrated full-suite/Clippy, retail assets,40-shape GPU and
640-fence GPU checks pass. Fresh integrated release/production checks also pass.
Code/oracle data were committed as4626c633. Earlier release/capture receipts
retain their own binary identities. The single critic pass is complete.

## Integrated release and visible validation

The final implementation on `8ade15f87907c1205c4f6a1b230dfc3762195823`
was built as the ordinary release app and `bridge_forcefire` example. Build
receipt `/tmp/bridge-projectile-reviewed-release/receipt.json` records 57.36
seconds, exit 0 and 42 warnings. Retained SHA-256 identities:

- `/tmp/bridge-projectile-reviewed-release/vera20k`: `91c247d839d8d03e3a9d879e288e5c31ceb47ab475e504d6188fb8d925654557`.
- `/tmp/bridge-projectile-reviewed-release/bridge_forcefire`: `4c17297be190dd2826b4eea54be15566b87f1e86950b0f103c2fbafc6c029c6f`.

The release example loaded the unchanged Hills map, saved already-moving Cannon
962 at binary frame 2, destroyed the bridge with 121 shots at frame 7327, and
observed all three metallic debris objects fly and expire over the following
200 frames. It exited 0 in 15.23 seconds; final state hash `342b75a3e9f6cc32`.
Map/rules hashes remain `c045c269668aa87e` / `f0c4434bfbe7e0df`.
Receipt `/tmp/bridge-projectile-reviewed-release-witness.json` pins the binary,
physical map and fresh current-format saves:

- `/Users/halvor/Documents/vera20k/saves/bridge-projectile-flight-20260926-215532.bin` (2233913 bytes): `d053b9212776ff5400b317cd0d5698b353c52cbb8eb373e63579eea79ddf2e8a`.
- `/Users/halvor/Documents/vera20k/saves/bridge-projectile-collapse-20260926-215532.bin` (2237035 bytes): `351524ab600be52b5cfddbeae81a4dbbaa6dbe6ecc66996e8067e0894de64ccb`.

The exact app ran with the primary checkout as cwd, only `RA2_DIR` among RA2
variables, and the ordinary menu. Single Player → Skirmish → Hills loaded the
actual map. In-game Load restored the flight save, then collapse, then flight
again; the same view showed intact, broken and restored deck geometry. The
explicit fragment-storage adapter guard admitted Apple M4/Metal normally.
The owned process was stopped after verification. Launch/log receipt:
`/tmp/bridge-projectile-reviewed-release/visible-launch.json` and `visible.log`.
Ordinary Shift+S PCX captures, inspected after lossless PNG conversion:

- `/Users/halvor/Documents/vera20k/SCRN0011.pcx`: `7024904af864b0d7f937ee0904ef6e2b67b38a1b074010d4824e0193546ac2d3`; PNG `/tmp/bridge-projectile-reviewed-release/visible-intact.png`.
- `/Users/halvor/Documents/vera20k/SCRN0012.pcx`: `e13528575b63d98ff6ce7a4972fde2cbc4c087ed70ed884ee74e9512eefa5d6d`; PNG `/tmp/bridge-projectile-reviewed-release/visible-collapse.png`.
- `/Users/halvor/Documents/vera20k/SCRN0013.pcx`: `3269b95dba138150eb685d68ccb70795cd7c3f1845d6d0cae9d543d2a1cc7b98`; PNG `/tmp/bridge-projectile-reviewed-release/visible-restored-flight.png`.

`/tmp/bridge-projectile-reviewed-release/visible-captures.json` records these
1024×768 images. They establish visible state restoration and production
rendering, not individual shell pixels, native whole-scene parity or game FPS.

## Earlier-source release and visible receipt

The release app and `bridge_forcefire` example were built together from the
validated source on base57a865e8. Build receipt:
`/tmp/bridge-projectile-release/receipt.json`; elapsed53.85 seconds with42
warnings. Retained executable identities:

- App `/tmp/bridge-projectile-release/vera20k`: SHA-256
  `ebb8452e61cc8e877f00e2aae12dde59c67e4ec89aa0059cd7f185a939a639a5`.
- Example `/tmp/bridge-projectile-release/bridge_forcefire`: SHA-256
  `d624ff16ea1c6b3836aa03a6eac707ad5d16daf662fd6e5f28074e350b7d12d1`.

The actual production load of loose `bridge-debris-hills-20260926.yrm` reports
map fingerprint `c045c269668aa87e` and rules fingerprint `f0c4434bfbe7e0df`.
Physical map SHA-256 is
`780d5d6e6d3c81ac0d510a5df326848dd426b3d840ac290114f44faf8eab9e9e`.
The live-flight save retains Cannon962 at XYZ(16512,18346,1153), launched from
(16512,18410,1140), in the logic vector and not OnBridge, while its target bridge
is intact. Ordinary fire collapses that bridge after121 Cannon shots at frame7327
and releases the target. Three DBRIS Bouncers fly and expire over the200-frame
continuation; the final simulation hash is `818d1342a839bdbe`, with SMOKEY2,
TWLT026 and TWLT036 follow-ups observed. This unchanged seed did not select D;
its native/focused coverage remains separate. The release example exits0 in
22.46 seconds. This is production composition, not native whole-frame proof.

Fresh save artifacts, recorded in `/tmp/bridge-projectile-release-witness.json`:

| State | Save path | SHA-256 |
| --- | --- | --- |
| Moving Cannon / intact bridge | `/Users/halvor/Documents/vera20k/saves/bridge-projectile-flight-20260926-212935.bin` | `53a92b95a629ca68619c489302e62686e8e9f58b3bf33ccb7b0f7399076fb64b` |
| Collapsed bridge / live debris | `/Users/halvor/Documents/vera20k/saves/bridge-projectile-collapse-20260926-212935.bin` | `8df7ff85fc54167ba4836cdde4a4540c9d584da87cad46db75af4e0cfcdae99e` |

The ordinary visible app uses the same app binary, packaged in
`/tmp/bridge-projectile-release/VERA20k Bridge Candidate.app`, launched from the
primary checkout with only RA2_DIR retained among RA2 variables. Receipt
`/tmp/bridge-projectile-release/visible-launch.json` records PID64947 and the
matching app hash. The normal menu route was Single Player → Skirmish → Start
on the loose Hills scenario, then Escape/Load of the fresh flight save. Home
and controlled edge pans aligned the bridge; tank selection/F exercised ordinary
input. Loading the fresh collapse save showed the broken middle deck and debris;
loading flight again restored the intact deck in the same view.

The1024×768 captures are primary-checkout `SCRN0008.pcx`, `SCRN0009.pcx` and
`SCRN0010.pcx`. Their source hashes and PNG paths are saved in
`/tmp/bridge-projectile-release/visible-captures.json`:
`visible-intact.png`, `visible-collapse.png`, and `visible-restored-flight.png`
under `/tmp/bridge-projectile-release/`. These demonstrate visible map/state
binding and ordinary loading/input. They do **not** establish individual tiny
projectile pixels; the production GPU comparison against40 physical native shape
cases owns that evidence. No native whole-scene or release-FPS claim is made.

## Performance receipt — reproduced stall and remedy

The production `draw_span` stress test uses synthetic 4×4 body/shadow pairs at
1024×768. Debug completed-submission wall times include CPU preparation/encoding,
submission, GPU execution and waiting; they are **not** GPU-only time or release
FPS. The reversed GPU timestamp marker was discarded. WindowServer and user
applications remained active; other owned game/build/native workloads were absent.

| Workload | Pieces / waves / render passes | Observed result before remedy |
| --- | --- | --- |
| 512 separated pairs, one/two atlas pages | 1024 / 2 / 4 | Completed-submission wall time about 1.6–1.84 ms. |
| 128 dense pairs | 256 / 256 / 512 | About 33 ms completed-submission wall time. |
| 512 dense pairs, one atlas page | 1024 / 1024 / 2048 | **Stalled during warm-up before submission.** The 36-sample benchmark did not complete. |
| Ordinary mixed retained layers and 20k non-destination-edit unit fence | Separate correctness fixture passes | This does not validate dense destination-edit throughput. |
| Normal release Hills tank fire near a bridge | Release/visible correctness PASS | Receipts above; no release frame-time/FPS measurement claimed. |

The captured stack reaches `CommandEncoder::finish` → wgpu render-pass encoding
→ Metal `begin_encoding` → `AGXCommandQueue::commandBuffer` → `semaphore_wait`.
The installed wgpu-core 27.0.3 allocates two Metal command buffers per render pass;
wgpu-hal 27.0.4 caps its pool at 4096. The observed 2048-pass boundary agrees with
exhausting that pool before the encoder can submit and retire work. This diagnosis
rests on the captured stack and pinned source, not an instrumented driver count.
A later `device.poll` timeout cannot protect this earlier finish operation.

Receipts: `/tmp/bridge-projectile-gpu-timing-final.log`,
`/tmp/bridge-projectile-dense512-sample.txt`, and
`/tmp/bridge-projectile-metal-stall.md`. The owner retained the sample and stopped
only the known benchmark process. The implementation below now passes the
original stress count and the added correctness/fence checks. This validates
the reproduced stall remedy within those fixtures; it does not establish normal
release performance or full-scene equivalence. Final CPU/Clippy and release/visible
checks pass; the single fresh critic is complete. Integrated CPU/Clippy and fresh release validation pass.

### Renderer remedy and post-remedy checks

The selected native ReadOnly leaves leave Z unchanged. An admitted body replaces
the destination word; only admitted shadows after the last such body affect the
final word. The renderer uses this property to remove one overlapping wave per
Bullet piece while preserving the retained command sequence:

1. Snapshot the original RGB565 word and immutable depth over the covered clips;
   reset the two scratch atomics for each covered pixel.
2. For admitted bodies, `atomicMax` selects the packed command ordinal and
   RGB565 word of the last body. The ordinal comes from command sequence,
   independently of instance storage, atlas page or piece type.
3. For admitted shadows, count only ordinals later than the selected body. With
   no admitted body, all admitted shadows count against the original word.
4. Resolve that body's word, or the snapshot when no body exists, through the
   original `(word >> 1) & 0x7BEF` operation. Six repetitions reach zero for every
   RGB565 word, so the loop caps at six. If no operation was admitted, discard
   instead of quantizing untouched original 8-bit attachment bytes.

[`terrain_read_only.wgsl`](../../src/render/terrain_read_only.wgsl) and
[`terrain_read_only_resolve.wgsl`](../../src/render/terrain_read_only_resolve.wgsl)
share the existing admission, palette and attachment owners. The 16-bit ordinal
limits each chunk to 65,535 commands. Larger intervals finish a chunk and start
another from its resulting attachment. Scratch uses eight bytes per pixel;
row bands fit the actual device `max_storage_buffer_binding_size`. Thus four
passes apply per nonempty chunk/band, not necessarily per entire frame.
ReadWrite Terrain waves remain separate because their Z writes change later
admission.

The reduction alone cannot bound a frame containing many ordinary-draw fences.
[`terrain_submission.rs`](../../src/render/terrain_submission.rs) therefore
submits completed replay groups once the closed-pass count reaches 128, creating
a continuation encoder on the same queue. It never splits an open render pass.
Subsequent passes load the same attachments in queue order. The threshold leaves
headroom beneath the pinned Metal pool rather than treating its limit as a
portable API guarantee. [`merge_passes.rs`](../../src/app/presentation/render/merge_passes.rs)
and [`draw_passes.rs`](../../src/app/presentation/render/draw_passes.rs) count
production ordinary Unit/SHP and other replay fences as well as destination
passes, preventing those fences from recreating an unsubmitted oversized frame.
This scheduler changes submission grouping, not retained parent/piece order or
the presentation boundary.

| Post-remedy check | Current receipt | Bounded coverage |
| --- | --- | --- |
| Ordered native sequences | PASS in both sRGB formats | [projectile_ordered_gpu_tests.rs](../../src/render/projectile_ordered_gpu_tests.rs): 157 native sequence prefixes and13 full three-row band cases. Overwrite order, nonmonotone instance/page storage and source/depth rejection compare to original leaf outputs. |
| No-op bytes, chunks and row bands | PASS in both sRGB formats | The same leaf tests preserve untouched 8-bit attachment bytes and compare continuation across command65535, plus the forced storage-band boundary. Log: `/tmp/bridge-projectile-reduced-gpu-leaves.log`. |
| Many actual ordinary fences | PASS in both sRGB formats | [projectile_submission_gpu_tests.rs](../../src/app/presentation/render/projectile_submission_gpu_tests.rs):640 actual Unit/SHP fences,2564 destination passes plus640 ordinary passes,24 bounded submissions. Full attachment color/depth readbacks match. Latest integrated rerun: `/tmp/bridge-projectile-integrated-submission.log`,1.23 seconds; earlier reduction receipt remains separate. |
| Prior40-shape, native-leaf and Ground/Terrain checks | PASS after remedy | All18 correctness tests pass on the post-remedy executable; log set and exact binary are above. |
| Dense512 and complete timing matrix | PASS, all36 samples complete | Production owner, original32/128/512 counts, separated/dense layouts, one/two pages, one warm-up then three measured frames per workload. Results below. |
| Final CPU/Clippy | PASS after integration | Integrated630ea66c… on8ade15f8:9552/0/157 and Clippy exit0. Older reduction/timing evidence remains tied to7ce9…; prior live-restore evidence remains tied to894b…. |
| Release and visible play | PASS on integrated8ade15f8 | Fresh app/example/menu/load/input and intact/collapsed/restored captures pass; exact artifacts below. No native whole-scene or release-FPS claim. |
| Single fresh critic | COMPLETE | No blocking defects; two accepted cleanups implemented and revalidated. One critic pass; accepted cleanups revalidated. |

The timing receipt is `/tmp/bridge-projectile-reduced-gpu-timing.json`, with raw
log `/tmp/bridge-projectile-reduced-gpu-timing.log`. All36 samples complete on
Apple M4 / Metal using Bgra8UnormSrgb at1024×768. Each sample now uses one reduced
interval, four passes and zero tile dependencies, for both separated and dense
layouts. Source/input sizes are unchanged from the earlier stress fixture.

| Pairs/layout, one and two pages | CPU preparation/encoding, ms | Completed-submission wall, ms | Earlier observation |
| --- | ---: | ---: | --- |
| 32 separated | 0.016–0.033 | 1.348–1.437 | No stall established at this size. |
| 32 dense | 0.014–0.015 | 1.163–1.347 | No stall established at this size. |
| 128 separated | 0.039–0.048 | 1.402–1.480 | No stall established at this size. |
| 128 dense | 0.041–0.226 | 1.447–1.491 | Approximately33–40 ms before reduction. |
| 512 separated | 0.121–0.162 | 1.690–1.947 | Approximately1.60–1.84 ms before reduction. |
| 512 dense | 0.127–0.152 | 1.677–3.154 | Before remedy, warm-up stalled before submission. |

Intervals include encoder creation, clears, scheduling and encoding; completed
wall time additionally includes finish, submission, GPU work and poll/wait.
Setup/uploads are excluded. These are debug-fixture observations, not GPU-only
durations, release FPS or a full-scene benchmark. Unreliable timestamp markers
remain excluded. Before and after the measured window there were no competing
Cargo/rustc/Clippy/libtest or owned-game workloads; other user applications and
WindowServer remained active.

The quiet fence rerun also passes:
`/tmp/bridge-projectile-reduced-gpu-fence-quiet.log`. One and20000 offscreen
UnitAtlas instances each remain a single ordinary run, with the same two
destination pieces/transactions. Warmed encoding samples for20000 instances
are5.292–8.958 µs in Ground and7.209–11.666 µs in Air; they exclude lowering,
upload, submission and waiting. This demonstrates that those ordinary instances
do not enter destination dependency planning, not20k visible-unit frame speed.

The passing GPU and stress receipts establish the remedy only within these
bounds. Integrated CPU/Clippy and release/visible checks also pass; the single
critic is complete. Whole-scene parity does not follow from these bounded receipts.

## Required residual mechanisms

The earlier Weapon Speed reader/consumer residual is now addressed by the
[IFV guided bridge chain](../../tools/projectile_oracle/ifv_launch.md), including
retained native ReadSpeed, complete Process/postpass history and selected launch/
guidance comparisons. That ledger owns current Speed evidence and its limits;
the Cannon receipts above remain historical candidates. Physical DRAGON drawing
now has its own bounded comparison there and does not inherit the Cannon proof.

| Trigger and frequency | Missing/unproven behavior and current treatment | Downstream risk |
| --- | --- | --- |
| FirersPalette on retail JUMP, DOGJUMP, ADOGJUMP, GiantNukeUp or GiantNukeDown; whenever those projectiles appear. DredMissile also authors it but is a dormant definition in the selected data | BulletConstruct 466519..46653B snapshots source House+16054 into Bullet+114. VERA lacks the retained House scheme authority and saved Bullet scheme. `projectile_sprite(None)` explicitly preserves the previous ANIM.PAL appearance through the shared Bullet path; `Some` can look up an explicitly supplied scheme entry, but production registration now packs only its used fallback. Cannon does not enter this path. | Wrong house color/Convert after ownership change, source deletion or restore; current-player fallback also requires its real owner. This is a required House initialization/capture/persistence chain, not an index-zero approximation claimed as native. |
| Admitted projectile ART Trailer/SpawnDelay on emitting families; absent on ordinary Cannon | The retained reader now preserves native identity/defaults, ordering and configuration hash. Selected Cannon has no Trailer, so this change does not implement or establish their runtime emission cadence, RNG, child attachment or cleanup. | Requires the emitting projectile lifecycle chain; matching reader values cannot certify emitted child timing, visibility or bridge-relative contacts. |
| Custom source stacks with no preceding AnimType or other ART read; absent from the selected retail predecessor | Original INI pointer-key cache lifetime can outlive the isolated Bullet body. The witness executes the actual preceding D reader, not a synthetic reset; the shared cache lifetime without that active-retail prerequisite is unproven. | Different ART section selection on such custom iteration/pass histories; the whole reader mechanism remains bounded. |
| Nuclear/deferred impact paths associated with NukeMaker/NUKEBALL; special nuclear deliveries, not ordinary 105mm | The native hidden/deferred Bullet byte+158 is exercised only as a supplied draw gate. Its producer, updates, child lifecycle and cleanup are not established or newly represented here; existing NukeMaker production remains a separate open mechanism. | Wrong visible lifetime/order around deferred impact and connected effects. A gate sample cannot establish the nuclear gameplay or cleanup chain. |
| Particle emitters sharing a viewport with shells; smoke/fire/trails and other emitting events | Production still builds particle-system sprites by page and depth and draws that family after represented Air parents. Individual native ParticleClass retained membership/order is not migrated. Isolated Cannon witnesses do not populate this mechanism. | Wrong overlap/blending or shadow destination when a particle should interleave with a Bullet. Requires particle lifecycle/order and drawing together. |
| Live Wave weapons sharing the viewport with shells; weapon-dependent Sonic/Magnetic-style events | Wave geometry still emits the existing polygon/white-texture family and is replayed separately after represented Air parents. This change does not port Wave drawing, native per-pixel effects or complete cross-family retained order. | Wrong occlusion, destination edits and interaction with bridges/other Air objects; a complete Wave consumer chain is required. |
| Voxel projectile types, physical RLE projectile shapes, or unrepresented BuildingLight children | Voxel Bullet drawing is not emitted by this SHP adapter. RLE leaf comparisons do not establish every RLE rowwalker/Z-shape input. BuildingLight still lacks its evolving child-coordinate/angle authority. | These cannot inherit the ordinary plain 120MM proof; their respective source, depth, lifetime and ordering dependencies remain open. |
| Fog/Display admission outside the prepared witnesses, or arbitrary full scenes | The native scenario helper's constant-false branch and padded projection gates are covered, but complete native tactical traversal/fog admission and a matched full-frame capture are not. VERA keeps its existing later shroud composition. | Do not generalize the bounded visible scene or constructor/draw samples to all observer, shroud or bridge configurations. |

These residuals keep the exhaustive bridge goal open. The bounded Cannon render
chain has passed integrated validation and its single critic. The comparisons
remain bounded to their separately recorded candidates and inputs.
