//! Chosen-map controller within the existing hidden tactical capture lifecycle.
//! The launch is the production session DTO; this diagnostic owns no gameplay.

use super::super::integrity::{SealedJsonFile, parse_strict_json, read_stable_regular_bytes};
use super::super::manifest::{PublishFault, publish_transaction};
use super::*;
use crate::app::diagnostics::state::{MAP_PRESENTATION_CLOCK_POLICY, MAP_PRESENTATION_INTERVAL_MS};
use crate::app::presentation::render::GameRenderTimes;
use crate::skirmish_launch::{LaunchStartPosition, PreFillHouseRoster, SkirmishLaunchSession};
use serde::{Deserialize, Serialize};

const SCHEMA: &str = "vera20k.map-observation-profile.v1";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MapCaptureProfile {
    pub(crate) schema_version: String,
    pub(crate) launch: SkirmishLaunchSession,
    pub(crate) seed: u32,
    pub(crate) input_delay_ticks: u32,
    pub(crate) ticks: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) timeout_seconds: u32,
}

impl MapCaptureProfile {
    pub(crate) fn load(path: &Path) -> Result<SealedJsonFile<Self>> {
        let (bytes, digest) = read_stable_regular_bytes(path, "map observation profile")?;
        let value: Self = parse_strict_json(&bytes, "map observation profile")?;
        value.validate()?;
        Ok(SealedJsonFile {
            path: path.to_path_buf(),
            byte_length: digest.byte_length,
            sha256: digest.sha256,
            bytes,
            value,
        })
    }

    fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == SCHEMA,
            "unsupported map observation schema"
        );
        ensure!(
            (640..=4096).contains(&self.width) && (480..=4096).contains(&self.height),
            "capture extent must be 640..4096 by 480..4096"
        );
        ensure!(self.ticks <= 100_000, "capture tick budget exceeds 100000");
        ensure!(
            (1..=900).contains(&self.timeout_seconds),
            "timeout must be 1..900 seconds"
        );
        ensure!(
            matches!(
                crate::match_bootstrap::classify_startup_session(&self.launch),
                StartupSessionClassification::AcceptedExplicitFixedBattle(_)
            ),
            "map observation requires an explicit fixed Battle launch"
        );
        ensure!(
            self.launch.pre_fill_house_roster
                == PreFillHouseRoster::from_compact_skirmish(self.launch.opponents.len()),
            "map observation requires the ordinary one-human compact skirmish roster"
        );
        let mut starts = std::collections::BTreeSet::new();
        let mut colors = std::collections::BTreeSet::new();
        for (start, color) in std::iter::once((
            self.launch.local.start_position,
            self.launch.local.color_index,
        ))
        .chain(
            self.launch
                .opponents
                .iter()
                .map(|slot| (slot.start_position, slot.color_index)),
        ) {
            let LaunchStartPosition::Position(start) = start else {
                bail!("automatic start is unsupported")
            };
            ensure!(
                start < 8 && starts.insert(start),
                "invalid or duplicate explicit start"
            );
            ensure!(
                color < 8 && colors.insert(color),
                "invalid or duplicate house color"
            );
        }
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct MapObservation {
    pub(super) initial: Option<Value>,
    inputs: Option<Value>,
    loaded_session: Option<Value>,
    draws: Vec<MapDrawTime>,
}

#[derive(Debug, Serialize)]
struct MapDrawTime {
    completed_steps: u64,
    radar_ms: u64,
    tooltip_ms: u64,
    message_ms: u64,
}

impl MapObservation {
    fn observe_draw(
        &mut self,
        requested: u32,
        completed_steps: u64,
        times: GameRenderTimes,
    ) -> Result<()> {
        ensure!(self.initial.is_some(), "map draw precedes accepted L0");
        ensure!(
            self.draws.len() < requested.max(1) as usize,
            "extra map draw"
        );
        let expected_step = if requested == 0 {
            0
        } else {
            self.draws.len() as u64 + 1
        };
        ensure!(
            completed_steps == expected_step,
            "map draw skipped or repeated a committed step"
        );
        let expected_ms = expected_step
            .checked_mul(MAP_PRESENTATION_INTERVAL_MS)
            .context("map presentation time overflow")?;
        let message_ms = times
            .message_ms
            .context("map draw omitted message expiry update")?;
        ensure!(
            times.radar_ms == expected_ms
                && times.tooltip_ms == expected_ms
                && message_ms == expected_ms,
            "map draw consumed inconsistent presentation clocks"
        );
        self.draws.push(MapDrawTime {
            completed_steps,
            radar_ms: times.radar_ms,
            tooltip_ms: times.tooltip_ms,
            message_ms,
        });
        Ok(())
    }
}

impl TacticalCaptureSession {
    fn map_state(&self) -> Result<&MapObservation> {
        match &self.controller {
            CaptureController::Map(map) => Ok(map),
            _ => bail!("map controller is absent"),
        }
    }

    fn map_state_mut(&mut self) -> Result<&mut MapObservation> {
        match &mut self.controller {
            CaptureController::Map(map) => Ok(map),
            _ => bail!("map controller is absent"),
        }
    }

    pub(super) fn prepare_map_observation(&mut self, state: &mut AppState) -> Result<()> {
        let profile = &self
            .request
            .map_profile()
            .context("map profile missing")?
            .value;
        ensure!(
            state.renderer.gpu.config.width == profile.width
                && state.renderer.gpu.config.height == profile.height,
            "map observation surface extent differs from request"
        );
        ensure!(
            matches!(
                state.renderer.gpu.config.format,
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
            ) && state
                .renderer
                .gpu
                .config
                .usage
                .contains(wgpu::TextureUsages::COPY_SRC),
            "map observation requires BGRA8 COPY_SRC surface"
        );
        ensure!(
            state.match_state.configured_input_delay_ticks == u64::from(profile.input_delay_ticks),
            "configured input delay differs from observation profile"
        );
        let config = state
            .platform
            .game_config
            .as_ref()
            .context("map observation requires config.toml")?;
        ensure!(
            !config.graphics.upscale && state.renderer.upscale_pass.is_none(),
            "map observation requires native-resolution rendering (upscale=false)"
        );
        let cwd = std::env::current_dir()?;
        let inputs = json!({
            "config": artifact(&cwd.join("config.toml"), "config.toml")?,
            "executable": artifact(&std::env::current_exe()?, "capture executable")?,
        });
        self.map_state_mut()?.inputs = Some(inputs);
        state.diag.use_map_presentation_clock()?;
        state.match_state.input.cursor_x = 0.0;
        state.match_state.input.cursor_y = 0.0;
        let now_ms =
            crate::app::match_runtime::sim_tick::monotonic_frame_pacer_ms(state, Instant::now());
        state.platform.frame_pacer.reanchor(now_ms);
        Ok(())
    }

    pub(super) fn drive_map_observation(&mut self, state: &mut AppState) -> Result<()> {
        if self.map_state()?.initial.is_none() {
            self.failure_stage = "rust-l0".to_owned();
            let profile = &self
                .request
                .map_profile()
                .context("map profile missing")?
                .value;
            let startup = state
                .match_state
                .startup
                .startup()
                .context("accepted startup absent")?;
            let receipt = state
                .match_state
                .startup
                .receipt()
                .context("Rust L0 receipt absent")?;
            ensure!(
                crate::match_bootstrap::accepted_tick_is_admitted(Some(startup), Some(receipt)),
                "loaded startup does not admit ticks"
            );
            ensure!(
                receipt.session.launch_session() == &profile.launch
                    && receipt.seed == profile.seed
                    && receipt.seed_source == MatchSeedSource::Controlled
                    && receipt.seed_authority_certifying
                    && receipt.tick == 0
                    && receipt.total_sim_ms == 0
                    && receipt.binary_frame == 0,
                "Rust L0 differs from requested fixed Battle launch"
            );
            ensure!(
                state.match_state.local_player_owner() == Some(profile.launch.player_name.as_str()),
                "loaded local owner differs from requested launch"
            );
            validate_loaded_resources(state)?;
            let sim = &state
                .match_state
                .sim_runtime
                .as_ref()
                .context("simulation absent")?
                .simulation;
            ensure!(
                sim.session.tick == 0
                    && sim.session.total_sim_ms == 0
                    && sim.session.binary_frame == 0
                    && sim.session.seed == u64::from(profile.seed)
                    && sim.input_delay_ticks == u64::from(profile.input_delay_ticks),
                "live simulation is not the requested tick-zero state"
            );
            validate_game_options(sim, &profile.launch)?;
            let slots: Vec<_> = sim
                .session
                .start_slot_houses
                .iter()
                .map(|(slot, house_id)| {
                    let house = sim.houses.get(house_id);
                    json!({"slot": slot, "house": sim.interner.resolve(*house_id),
                    "waypoint": sim.session.mp_start_waypoints.get(slot),
                    "country": house.and_then(|h| h.country).map(|c| sim.interner.resolve(c)),
                    "human": house.map(|h| h.is_human), "difficulty": house.map(|h| h.difficulty)})
                })
                .collect();
            for start in std::iter::once(profile.launch.local.start_position).chain(
                profile
                    .launch
                    .opponents
                    .iter()
                    .map(|slot| slot.start_position),
            ) {
                let LaunchStartPosition::Position(index) = start else {
                    bail!("unresolved start")
                };
                ensure!(
                    sim.session
                        .mp_start_waypoints
                        .contains_key(&u32::from(index))
                        && sim
                            .session
                            .start_slot_houses
                            .contains_key(&u32::from(index)),
                    "requested start {index} was not installed in the loaded map"
                );
            }
            let loaded_session = json!({"map_name": sim.session.map_name, "theater": sim.session.theater,
                "options": sim.session.game_options, "start_slots": slots, "map_waypoints": sim.session.mp_start_waypoints});
            let source = state
                .match_state
                .loaded_map_source
                .as_ref()
                .context("loaded source absent")?;
            ensure!(
                matches!(
                    source,
                    crate::map::source::LoadedMapSource::Loose { .. }
                        | crate::map::source::LoadedMapSource::Mix { .. }
                ),
                "map observation requires consumed loose or MIX map bytes"
            );
            self.map_source_evidence = Some(serde_json::to_value(source)?);
            self.map_state_mut()?.initial = Some(self.map_fingerprint(state)?);
            self.map_state_mut()?.loaded_session = Some(loaded_session);
            self.post_l0_started_at = Some(Instant::now());
            self.failure_stage = "exact-steps".to_owned();
        }
        let requested = self
            .request
            .map_profile()
            .context("map profile missing")?
            .value
            .ticks as usize;
        ensure!(
            self.exact_step_receipts.len() <= requested,
            "map observation exceeded tick budget"
        );
        ensure!(
            self.map_state()?.draws.len() == self.exact_step_receipts.len(),
            "previous map step has no completed draw"
        );
        if self.exact_step_receipts.len() < requested {
            self.advance_exact_step(state)?;
        }
        if self.exact_step_receipts.len() == requested {
            self.capture_requested = true;
            self.failure_stage = "final-render".to_owned();
        }
        Ok(())
    }

    pub(super) fn observe_map_draw(
        &mut self,
        state: &AppState,
        output: &GameRenderOutput,
    ) -> Result<()> {
        let requested = self
            .request
            .map_profile()
            .context("map profile missing")?
            .value
            .ticks;
        let sim = &state
            .match_state
            .sim_runtime
            .as_ref()
            .context("map simulation absent")?
            .simulation;
        ensure!(
            sim.session.tick == self.exact_step_receipts.len() as u64
                && u64::from(sim.session.binary_frame) == sim.session.tick,
            "map draw differs from committed exact-step receipts"
        );
        ensure!(
            state.diagnostic_presentation_ms() == Some(output.times.radar_ms),
            "map diagnostic presentation policy is not active"
        );
        self.map_state_mut()?
            .observe_draw(requested, sim.session.tick, output.times)
    }

    pub(super) fn map_fingerprint(&self, state: &AppState) -> Result<Value> {
        let sim = &state
            .match_state
            .sim_runtime
            .as_ref()
            .context("simulation absent")?
            .simulation;
        Ok(json!(FinalFingerprint {
            simulation_tick: sim.session.tick,
            total_simulation_ms: sim.session.total_sim_ms,
            binary_frame: sim.session.binary_frame,
            deterministic_state_hash: sim.state_hash()
        }))
    }

    pub(super) fn map_render_readiness(
        &self,
        state: &AppState,
        output: &GameRenderOutput,
    ) -> Result<(bool, Value)> {
        validate_loaded_resources(state)?;
        let unit_atlas = state
            .match_state
            .match_presentation
            .unit_atlas
            .as_ref()
            .context("unit atlas absent at final map render")?
            .statistics()?;
        let ready = output.sidebar_view.is_some()
            && !state.match_state.paused()
            && !state.match_state.match_presentation.show_save_load_panel
            && !state.main_menu_dialog_open()
            && !state.diag.debug_show_pathgrid
            && !state.diag.debug_unit_inspector
            && !state.diag.debug_show_cell_grid
            && !state.diag.debug_show_heightmap
            && !state.match_state.match_presentation.show_hotkey_help
            && self.focus_violations == 0
            && self.input_violations == 0;
        let static_default_cursor =
            crate::app::presentation::ui_overlays::static_default_cursor(state);
        let camera_input_idle = crate::app::input::camera::camera_input_idle(state);
        ensure!(
            ready && static_default_cursor && camera_input_idle,
            "map observation requires every draw ready with a static cursor and idle camera input"
        );
        Ok((
            ready,
            json!({"ready": ready, "sidebar_view_present": output.sidebar_view.is_some(),
                "instance_counts": super::super::evidence::RenderInstanceCountEvidence::from_counts(output.instance_counts)?,
                "internal_extent": [state.render_width(), state.render_height()],
                "surface_extent": [state.renderer.gpu.config.width, state.renderer.gpu.config.height],
                "ui_scale": state.match_state.match_presentation.ui_scale,
                "gpu": super::super::evidence::GpuAdapterEvidence::from_observation(state.renderer.gpu.capture_adapter_observation()),
                "unit_atlas": unit_atlas,
                "neutral_input": {"static_default_cursor": static_default_cursor, "camera_input_idle": camera_input_idle},
            }),
        ))
    }

    pub(super) fn publish_map_observation(
        &self,
        state: &AppState,
        format: wgpu::TextureFormat,
        pixels: &[u8],
    ) -> Result<()> {
        let profile = self.request.map_profile().context("map profile missing")?;
        ensure!(
            self.exact_step_receipts.len() == profile.value.ticks as usize,
            "incomplete exact-step sequence"
        );
        let frame = FrameArtifact::from_bgra(
            self.request.width(),
            self.request.height(),
            format!("{format:?}"),
            pixels,
        )?;
        let map = self.map_state()?;
        ensure!(
            map.draws.len() == profile.value.ticks.max(1) as usize,
            "incomplete map draw schedule"
        );
        let mut render = self
            .last_render_evidence
            .clone()
            .context("map render evidence absent")?;
        // Serialize the actual transcript only once, avoiding quadratic work
        // across the up-to-100000-step capture route.
        render["presentation_clock"] = json!({"policy": MAP_PRESENTATION_CLOCK_POLICY,
            "origin_ms": 0, "interval_ms": MAP_PRESENTATION_INTERVAL_MS, "draws": map.draws});
        let manifest = json!({
            "schema_version": "vera20k.map-observation.v3", "status": "COMPLETE",
            "profile": {"sha256": profile.sha256, "request": profile.value},
            "contract": {"sha256": self.request.sealed_contract().sha256},
            "inputs": self.map_state()?.inputs, "map_source": self.map_source_evidence,
            "startup": self.startup_evidence, "initial": self.map_state()?.initial,
            "loaded_session": self.map_state()?.loaded_session,
            "final": self.map_fingerprint(state)?, "exact_step_count": self.exact_step_receipts.len(),
            "first_exact_step": self.exact_step_receipts.first(), "last_exact_step": self.exact_step_receipts.last(),
            "frame": frame, "render": render,
            "lifecycle": {"window_hidden": state.platform.window.is_visible() == Some(false),
                "window_focused": state.platform.window.has_focus(), "focus_violations": self.focus_violations,
                "input_violations": self.input_violations},
            "native_comparator": "NONE", "parity_certification": "NONE",
            "evidence_limitations": ["Production loading, exact stepping and GPU readback only; no native pixel or gameplay equivalence is established.",
                "Radar and timed HUD presentation consume the recorded diagnostic exact-step clock; ordinary gameplay clocks are unchanged. Audio, menus, animated input and scenario exit are outside this comparison."],
        });
        publish_transaction(
            self.request.output_dir(),
            &manifest,
            Some(pixels),
            PublishFault::None,
        )
    }

    pub(super) fn publish_map_failure(&self, error: &str) -> Result<()> {
        let profile = self.request.map_profile().context("map profile missing")?;
        let manifest = json!({"schema_version": "vera20k.map-observation.v3", "status": "FAILED",
            "profile": {"sha256": profile.sha256, "request": profile.value},
            "contract": {"sha256": self.request.sealed_contract().sha256},
            "failure": {"stage": self.failure_stage, "message": error}, "frame": null,
            "exact_step_count": self.exact_step_receipts.len(), "map_source": self.map_source_evidence,
            "native_comparator": "NONE", "parity_certification": "NONE"});
        publish_transaction(
            self.request.output_dir(),
            &manifest,
            None,
            PublishFault::None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn initialized_map() -> MapObservation {
        MapObservation {
            initial: Some(json!({"tick": 0})),
            ..Default::default()
        }
    }

    fn times(ms: u64) -> GameRenderTimes {
        GameRenderTimes {
            radar_ms: ms,
            tooltip_ms: ms,
            message_ms: Some(ms),
        }
    }

    #[test]
    fn capture_retains_only_actual_draws_in_committed_order() {
        let mut zero = initialized_map();
        zero.observe_draw(0, 0, times(0)).unwrap();
        assert!(zero.observe_draw(0, 0, times(0)).is_err());
        let mut map = initialized_map();
        assert!(map.observe_draw(3, 0, times(0)).is_err());
        map.observe_draw(3, 1, times(22)).unwrap();
        assert!(map.observe_draw(3, 1, times(22)).is_err());
        assert!(map.observe_draw(3, 3, times(66)).is_err());
        map.observe_draw(3, 2, times(44)).unwrap();
        map.observe_draw(3, 3, times(66)).unwrap();
        let evidence = serde_json::to_value(&map.draws).unwrap();
        assert_eq!(evidence[0]["completed_steps"], 1);
        assert_eq!(evidence[2]["radar_ms"], 66);
        assert_eq!(map.draws.len(), 3);
        assert!(map.observe_draw(3, 4, times(88)).is_err());
    }

    #[test]
    fn capture_rejects_missing_hud_update_or_any_wall_clock_sample() {
        assert!(
            MapObservation::default()
                .observe_draw(1, 1, times(22))
                .is_err()
        );
        let mut map = initialized_map();
        for sample in [
            GameRenderTimes {
                radar_ms: 9_876,
                ..times(22)
            },
            GameRenderTimes {
                tooltip_ms: 9_876,
                ..times(22)
            },
            GameRenderTimes {
                message_ms: Some(0),
                ..times(22)
            },
            GameRenderTimes {
                message_ms: None,
                ..times(22)
            },
        ] {
            assert!(map.observe_draw(1, 1, sample).is_err());
            assert!(
                map.draws.is_empty(),
                "rejected observation must not advance schedule"
            );
        }
        map.observe_draw(1, 1, times(22)).unwrap();
    }

    fn example() -> MapCaptureProfile {
        serde_json::from_str(include_str!(
            "../../../../tools/map_observation.example.json"
        ))
        .unwrap()
    }

    #[test]
    fn example_preserves_existing_accepted_launch_and_allows_chosen_map() {
        let mut profile = example();
        profile.validate().unwrap();
        let radar: super::super::super::profile::TacticalCaptureProfile =
            serde_json::from_str(include_str!(
                "../../../../tools/tactical_certification/profiles/soviet-radar-online-v2.json"
            ))
            .unwrap();
        assert_eq!(profile.launch, radar.launch_session());
        profile.launch.selected_map_file = Some("mp01t4.map".to_owned());
        profile.validate().unwrap();
        profile.ticks = 0;
        profile.validate().unwrap();
    }

    #[test]
    fn unresolved_launch_duplicate_slots_and_inconsistent_roster_fail() {
        let mut profile = example();
        profile.launch.local.start_position = LaunchStartPosition::Auto;
        assert!(profile.validate().is_err());
        let mut profile = example();
        profile.launch.opponents[0].color_index = profile.launch.local.color_index;
        assert!(profile.validate().is_err());
        let mut profile = example();
        profile.launch.opponents.clear();
        assert!(profile.validate().is_err());
    }

    #[test]
    fn invalid_budgets_and_unknown_request_fields_fail() {
        let mut profile = example();
        profile.timeout_seconds = 0;
        assert!(profile.validate().is_err());
        profile.timeout_seconds = 180;
        profile.ticks = 100_001;
        assert!(profile.validate().is_err());
        let mut json = serde_json::to_value(example()).unwrap();
        json["launch"]["ignored_option"] = json!(true);
        assert!(serde_json::from_value::<MapCaptureProfile>(json).is_err());
    }
}
