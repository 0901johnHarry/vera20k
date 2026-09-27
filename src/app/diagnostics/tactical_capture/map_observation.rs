//! Chosen-map controller within the existing hidden tactical capture lifecycle.
//! The launch is the production session DTO; this diagnostic owns no gameplay.

use super::super::integrity::{SealedJsonFile, parse_strict_json, read_stable_regular_bytes};
use super::super::manifest::{PublishFault, publish_transaction};
use super::*;
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
                    crate::app::frontend::list_maps::LoadedMapSource::Loose { .. }
                        | crate::app::frontend::list_maps::LoadedMapSource::Mix { .. }
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
        if self.exact_step_receipts.len() < requested {
            self.advance_exact_step(state)?;
        }
        if self.exact_step_receipts.len() == requested {
            self.capture_requested = true;
            self.failure_stage = "final-render".to_owned();
        }
        Ok(())
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
        let ready = output.sidebar_view.is_some()
            && !state.match_state.paused()
            && !state.match_state.match_presentation.show_save_load_panel
            && !state.main_menu_dialog_open()
            && !state.diag.debug_show_pathgrid
            && !state.diag.debug_unit_inspector
            && self.focus_violations == 0
            && self.input_violations == 0;
        Ok((
            ready,
            json!({"ready": ready, "sidebar_view_present": output.sidebar_view.is_some(),
                "instance_counts": super::super::evidence::RenderInstanceCountEvidence::from_counts(output.instance_counts)?,
                "internal_extent": [state.render_width(), state.render_height()],
                "surface_extent": [state.renderer.gpu.config.width, state.renderer.gpu.config.height],
                "ui_scale": state.match_state.match_presentation.ui_scale,
                "gpu": super::super::evidence::GpuAdapterEvidence::from_observation(state.renderer.gpu.capture_adapter_observation()),
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
        let manifest = json!({
            "schema_version": "vera20k.map-observation.v1", "status": "COMPLETE",
            "profile": {"sha256": profile.sha256, "request": profile.value},
            "contract": {"sha256": self.request.sealed_contract().sha256},
            "inputs": self.map_state()?.inputs, "map_source": self.map_source_evidence,
            "startup": self.startup_evidence, "initial": self.map_state()?.initial,
            "loaded_session": self.map_state()?.loaded_session,
            "final": self.map_fingerprint(state)?, "exact_step_count": self.exact_step_receipts.len(),
            "first_exact_step": self.exact_step_receipts.first(), "last_exact_step": self.exact_step_receipts.last(),
            "frame": frame, "render": self.last_render_evidence,
            "lifecycle": {"window_hidden": state.platform.window.is_visible() == Some(false),
                "window_focused": state.platform.window.has_focus(), "focus_violations": self.focus_violations,
                "input_violations": self.input_violations},
            "native_comparator": "NONE", "parity_certification": "NONE",
            "evidence_limitations": ["Production loading, exact stepping and GPU readback only; no native pixel or gameplay equivalence is established.",
                "Simulation determinism is compared separately from wall-clock presentation and audio."],
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
        let manifest = json!({"schema_version": "vera20k.map-observation.v1", "status": "FAILED",
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
