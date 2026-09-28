use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::app::match_runtime::sim_tick::SessionMode;
use crate::sim::command::{Command, CommandEnvelope};
use crate::sim::intern::InternedId;
use crate::sim::runtime::SimRuntime;
use crate::util::config::ExternalAiConfig;

use super::actions::validate_actions;
use super::observation::build_observation;
use super::protocol::{ExternalAiDecision, ExternalAiError};
use super::worker::{ExternalAiWorker, RequestTag, WorkerResult};

const FRAME_SERIAL_HALF_RANGE: u32 = 1 << 31;
const RETRY_BASE_FRAMES: u32 = 30;
const RETRY_MAX_FRAMES: u32 = 900;
const RETRY_MAX_EXPONENT: u32 = 5;
static NEXT_MATCH_GENERATION: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy)]
pub(in crate::app::match_runtime) struct CoordinatorAdmission {
    pub(in crate::app::match_runtime) mode: SessionMode,
    pub(in crate::app::match_runtime) game_mode_nonzero: bool,
    pub(in crate::app::match_runtime) replaying: bool,
    pub(in crate::app::match_runtime) ordinary_lane: bool,
}

pub(super) trait ExternalAiTransport {
    fn try_submit(
        &mut self,
        tag: RequestTag,
        observation: serde_json::Value,
        max_actions: usize,
    ) -> Result<(), ExternalAiError>;

    fn try_receive(&mut self) -> Result<Option<WorkerResult>, ExternalAiError>;
}

impl ExternalAiTransport for ExternalAiWorker {
    fn try_submit(
        &mut self,
        tag: RequestTag,
        observation: serde_json::Value,
        max_actions: usize,
    ) -> Result<(), ExternalAiError> {
        ExternalAiWorker::try_submit(self, tag, observation, max_actions)
    }

    fn try_receive(&mut self) -> Result<Option<WorkerResult>, ExternalAiError> {
        ExternalAiWorker::try_receive(self)
    }
}

#[derive(Debug, Clone, Copy)]
struct RetryState {
    failures: u8,
    retry_after_frame: u32,
}

/// Match-scoped app coordinator. All gameplay state remains in `Simulation`;
/// this owner retains only request identities and transport retry metadata.
pub(crate) struct ExternalAiCoordinator {
    enabled: bool,
    generation: u64,
    worker: Option<ExternalAiWorker>,
    request_interval_frames: u32,
    max_actions_per_response: usize,
    control_lease_frames: u32,
    in_flight: HashMap<InternedId, RequestTag>,
    last_request_frame: HashMap<InternedId, u32>,
    retry: HashMap<InternedId, RetryState>,
}

impl ExternalAiCoordinator {
    pub(in crate::app::match_runtime) fn new(config: &ExternalAiConfig) -> Self {
        let worker = if config.enabled {
            match ExternalAiWorker::start(config) {
                Ok(worker) => Some(worker),
                Err(error) => {
                    log::warn!("External AI worker is unavailable: {error}");
                    None
                }
            }
        } else {
            None
        };
        Self::with_worker(config, worker)
    }

    fn with_worker(config: &ExternalAiConfig, worker: Option<ExternalAiWorker>) -> Self {
        Self {
            enabled: config.enabled,
            generation: NEXT_MATCH_GENERATION.fetch_add(1, Ordering::Relaxed),
            worker,
            request_interval_frames: config.request_interval_frames.max(1),
            max_actions_per_response: config.max_actions_per_response,
            control_lease_frames: config.control_lease_frames.max(1),
            in_flight: HashMap::new(),
            last_request_frame: HashMap::new(),
            retry: HashMap::new(),
        }
    }

    /// Process completed work and submit due immutable observations. This
    /// method never waits; both transport operations are try-only.
    pub(in crate::app::match_runtime) fn advance(
        &mut self,
        runtime: &SimRuntime,
        admission: CoordinatorAdmission,
    ) -> Vec<CommandEnvelope> {
        let Some(mut worker) = self.worker.take() else {
            return Vec::new();
        };
        let (commands, worker_usable) = self.advance_with_io(runtime, admission, &mut worker);
        if worker_usable {
            self.worker = Some(worker);
        }
        commands
    }

    fn advance_with_io(
        &mut self,
        runtime: &SimRuntime,
        admission: CoordinatorAdmission,
        transport: &mut impl ExternalAiTransport,
    ) -> (Vec<CommandEnvelope>, bool) {
        if !self.enabled
            || admission.mode != SessionMode::Skirmish
            || !admission.game_mode_nonzero
            || admission.replaying
            || !admission.ordinary_lane
        {
            return (Vec::new(), true);
        }

        let simulation = &runtime.simulation;
        let current_frame = simulation.session.binary_frame;
        let mut results = Vec::new();
        let mut worker_usable = true;
        loop {
            match transport.try_receive() {
                Ok(Some(result)) => results.push(result),
                Ok(None) => break,
                Err(_) => {
                    worker_usable = false;
                    break;
                }
            }
        }

        let mut commands = self.accept_results(runtime, current_frame, results);
        if !worker_usable {
            self.fail_all_in_flight(current_frame);
            return (commands, false);
        }

        let owners = eligible_ai_owners(runtime);
        let mut seen = HashSet::with_capacity(owners.len());
        for owner in owners {
            if !seen.insert(owner)
                || !current_frame.is_multiple_of(self.request_interval_frames)
                || self.in_flight.contains_key(&owner)
                || self.last_request_frame.get(&owner) == Some(&current_frame)
                || !self.retry_ready(owner, current_frame)
            {
                continue;
            }

            let tag = RequestTag {
                match_generation: self.generation,
                owner,
                source_frame: current_frame,
            };
            let observation = build_observation(runtime, owner);
            match transport.try_submit(tag, observation, self.max_actions_per_response) {
                Ok(()) => {
                    self.in_flight.insert(owner, tag);
                    self.last_request_frame.insert(owner, current_frame);
                }
                Err(error) => {
                    if error == ExternalAiError::WorkerStopped {
                        worker_usable = false;
                        break;
                    }
                    self.note_failure(owner, current_frame, Some(&error));
                }
            }
        }

        if !worker_usable {
            self.fail_all_in_flight(current_frame);
        }
        (std::mem::take(&mut commands), worker_usable)
    }

    fn accept_results(
        &mut self,
        runtime: &SimRuntime,
        current_frame: u32,
        results: Vec<WorkerResult>,
    ) -> Vec<CommandEnvelope> {
        let mut commands = Vec::new();
        for result in results {
            if result.tag.match_generation != self.generation
                || self.in_flight.get(&result.tag.owner) != Some(&result.tag)
            {
                continue;
            }
            self.in_flight.remove(&result.tag.owner);

            let age = current_frame.wrapping_sub(result.tag.source_frame);
            if age >= FRAME_SERIAL_HALF_RANGE {
                self.note_failure(result.tag.owner, current_frame, None);
                continue;
            }
            if age > self.control_lease_frames {
                self.note_failure(result.tag.owner, current_frame, None);
                continue;
            }
            if !eligible_ai_owners(runtime).contains(&result.tag.owner) {
                self.retry.remove(&result.tag.owner);
                continue;
            }

            let decision = match result.result {
                Ok(decision) => decision,
                Err(error) => {
                    self.note_failure(result.tag.owner, current_frame, Some(&error));
                    continue;
                }
            };
            match self.commands_for_decision(runtime, result.tag.owner, current_frame, decision) {
                Ok(mut accepted) => {
                    self.retry.remove(&result.tag.owner);
                    commands.append(&mut accepted);
                }
                Err(error) => {
                    self.note_failure(result.tag.owner, current_frame, Some(&error));
                }
            }
        }
        commands
    }

    fn commands_for_decision(
        &self,
        runtime: &SimRuntime,
        owner: InternedId,
        current_frame: u32,
        decision: ExternalAiDecision,
    ) -> Result<Vec<CommandEnvelope>, ExternalAiError> {
        let actions = validate_actions(
            runtime,
            owner,
            &decision.actions,
            self.max_actions_per_response,
        )
        .map_err(|_| ExternalAiError::TooManyActions)?;
        let execute_tick = runtime.simulation.session.tick.saturating_add(1);
        let until_frame = current_frame.wrapping_add(self.control_lease_frames);
        let mut commands = Vec::with_capacity(actions.len() + 1);
        commands.push(CommandEnvelope::new(
            owner,
            execute_tick,
            Command::RenewExternalAiLease { until_frame },
        ));
        commands.extend(
            actions
                .into_iter()
                .map(|action| CommandEnvelope::new(owner, execute_tick, action)),
        );
        Ok(commands)
    }

    fn retry_ready(&self, owner: InternedId, current_frame: u32) -> bool {
        self.retry.get(&owner).is_none_or(|state| {
            current_frame.wrapping_sub(state.retry_after_frame) < FRAME_SERIAL_HALF_RANGE
        })
    }

    fn note_failure(
        &mut self,
        owner: InternedId,
        current_frame: u32,
        error: Option<&ExternalAiError>,
    ) {
        let state = self.retry.entry(owner).or_insert(RetryState {
            failures: 0,
            retry_after_frame: current_frame,
        });
        state.failures = state
            .failures
            .saturating_add(1)
            .min((RETRY_MAX_EXPONENT + 1) as u8);
        let exponent = u32::from(state.failures.saturating_sub(1));
        let delay = RETRY_BASE_FRAMES
            .checked_shl(exponent)
            .unwrap_or(u32::MAX)
            .min(RETRY_MAX_FRAMES);
        state.retry_after_frame = current_frame.wrapping_add(delay);
        if let Some(error) = error {
            log::warn!("External AI decision failed; local AI remains available: {error}");
        }
    }

    fn fail_all_in_flight(&mut self, current_frame: u32) {
        let owners = self
            .in_flight
            .drain()
            .map(|(owner, _)| owner)
            .collect::<Vec<_>>();
        for owner in owners {
            self.note_failure(owner, current_frame, Some(&ExternalAiError::WorkerStopped));
        }
    }

    #[cfg(test)]
    fn new_for_test(config: &ExternalAiConfig) -> Self {
        Self::with_worker(config, None)
    }

    #[cfg(test)]
    fn generation(&self) -> u64 {
        self.generation
    }

    #[cfg(test)]
    fn in_flight_is_empty(&self) -> bool {
        self.in_flight.is_empty()
    }
}

fn eligible_ai_owners(runtime: &SimRuntime) -> Vec<InternedId> {
    let simulation = &runtime.simulation;
    let game_mode_nonzero = simulation.session.game_mode_nonzero;
    simulation
        .ai_players
        .iter()
        .filter_map(|ai| {
            let house = simulation.houses.get(&ai.owner)?;
            (!house.is_defeated
                && !house.multiplay_passive
                && !house.is_controlled_by_human(game_mode_nonzero))
            .then_some(ai.owner)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crate::app::match_runtime::sim_tick::SessionMode;
    use crate::map::entities::EntityCategory;
    use crate::sim::ai::AiPlayerState;
    use crate::sim::command::{Command, CommandEnvelope};
    use crate::sim::runtime::SimRuntime;
    use crate::util::config::ExternalAiConfig;

    use super::super::protocol::{ExternalAiAction, ExternalAiDecision, ExternalAiError};
    use super::super::test_support::{add_entity, runtime};
    use super::{
        CoordinatorAdmission, ExternalAiCoordinator, ExternalAiTransport, RequestTag, WorkerResult,
    };

    struct FakeTransport {
        received: VecDeque<Result<Option<WorkerResult>, ExternalAiError>>,
        submitted: Vec<(RequestTag, serde_json::Value, usize)>,
        receive_calls: usize,
    }

    impl FakeTransport {
        fn new() -> Self {
            Self {
                received: VecDeque::new(),
                submitted: Vec::new(),
                receive_calls: 0,
            }
        }

        fn result(&mut self, result: WorkerResult) {
            self.received.push_back(Ok(Some(result)));
        }
    }

    impl ExternalAiTransport for FakeTransport {
        fn try_submit(
            &mut self,
            tag: RequestTag,
            observation: serde_json::Value,
            max_actions: usize,
        ) -> Result<(), ExternalAiError> {
            self.submitted.push((tag, observation, max_actions));
            Ok(())
        }

        fn try_receive(&mut self) -> Result<Option<WorkerResult>, ExternalAiError> {
            self.receive_calls += 1;
            self.received.pop_front().unwrap_or(Ok(None))
        }
    }

    fn config() -> ExternalAiConfig {
        ExternalAiConfig {
            enabled: true,
            endpoint: "http://127.0.0.1:1/v1/chat/completions".into(),
            model: "fixture-model".into(),
            request_interval_frames: 225,
            request_timeout_secs: 1,
            max_actions_per_response: 16,
            control_lease_frames: 450,
        }
    }

    fn ai_runtime() -> (SimRuntime, crate::sim::intern::InternedId) {
        let (mut runtime, owner) = runtime();
        runtime.simulation.session.game_mode_nonzero = true;
        runtime
            .simulation
            .ai_players
            .push(AiPlayerState::new(owner));
        (runtime, owner)
    }

    fn admission() -> CoordinatorAdmission {
        CoordinatorAdmission {
            mode: SessionMode::Skirmish,
            game_mode_nonzero: true,
            replaying: false,
            ordinary_lane: true,
        }
    }

    fn next_tag(transport: &FakeTransport) -> RequestTag {
        transport.submitted.last().expect("request was submitted").0
    }

    #[test]
    fn request_cadence_uses_binary_frames_and_prevents_duplicate_owner_jobs() {
        let (mut runtime, owner) = ai_runtime();
        let mut coordinator = ExternalAiCoordinator::new_for_test(&config());
        let mut transport = FakeTransport::new();

        runtime.simulation.session.binary_frame = 224;
        coordinator.advance_with_io(&runtime, admission(), &mut transport);
        assert!(transport.submitted.is_empty());

        runtime.simulation.session.binary_frame = 225;
        coordinator.advance_with_io(&runtime, admission(), &mut transport);
        coordinator.advance_with_io(&runtime, admission(), &mut transport);
        runtime.simulation.session.binary_frame = 450;
        coordinator.advance_with_io(&runtime, admission(), &mut transport);

        assert_eq!(transport.submitted.len(), 1);
        assert_eq!(transport.submitted[0].0.owner, owner);
        assert_eq!(transport.submitted[0].0.source_frame, 225);
        assert_eq!(transport.submitted[0].2, 16);
    }

    #[test]
    fn disabled_non_skirmish_replay_and_non_ordinary_frames_do_no_io() {
        let (mut runtime, _) = ai_runtime();
        let mut coordinator = ExternalAiCoordinator::new_for_test(&config());
        let mut transport = FakeTransport::new();
        runtime.simulation.session.binary_frame = 225;

        let mut denied = vec![
            CoordinatorAdmission {
                replaying: true,
                ..admission()
            },
            CoordinatorAdmission {
                mode: SessionMode::Campaign,
                ..admission()
            },
            CoordinatorAdmission {
                game_mode_nonzero: false,
                ..admission()
            },
            CoordinatorAdmission {
                ordinary_lane: false,
                ..admission()
            },
        ];
        let mut disabled = ExternalAiCoordinator::new_for_test(&ExternalAiConfig::default());
        for gate in denied.drain(..) {
            coordinator.advance_with_io(&runtime, gate, &mut transport);
        }
        disabled.advance_with_io(&runtime, admission(), &mut transport);

        assert!(transport.submitted.is_empty());
        assert!(transport.received.is_empty());
        assert_eq!(transport.receive_calls, 0);
    }

    #[test]
    fn a_late_but_fresh_response_renews_lease_and_emits_validated_actions() {
        let (mut runtime, owner) = ai_runtime();
        add_entity(
            &mut runtime,
            2,
            "Americans",
            "E1",
            EntityCategory::Infantry,
            5,
            5,
        );
        let mut coordinator = ExternalAiCoordinator::new_for_test(&config());
        let mut transport = FakeTransport::new();
        runtime.simulation.session.binary_frame = 225;
        coordinator.advance_with_io(&runtime, admission(), &mut transport);
        let tag = next_tag(&transport);

        runtime.simulation.session.binary_frame = 227;
        transport.result(WorkerResult {
            tag,
            result: Ok(ExternalAiDecision {
                actions: vec![ExternalAiAction::Guard {
                    entity_id: 2,
                    target_id: None,
                }],
            }),
        });
        let (commands, _) = coordinator.advance_with_io(&runtime, admission(), &mut transport);

        assert_eq!(
            commands,
            vec![
                CommandEnvelope::new(
                    owner,
                    runtime.simulation.session.tick + 1,
                    Command::RenewExternalAiLease { until_frame: 677 },
                ),
                CommandEnvelope::new(
                    owner,
                    runtime.simulation.session.tick + 1,
                    Command::Guard {
                        entity_id: 2,
                        target_id: None,
                    },
                ),
            ]
        );
    }

    #[test]
    fn old_generation_and_expired_responses_cannot_emit_commands() {
        let (mut runtime, _) = ai_runtime();
        let mut previous = ExternalAiCoordinator::new_for_test(&config());
        let mut old_transport = FakeTransport::new();
        runtime.simulation.session.binary_frame = 225;
        previous.advance_with_io(&runtime, admission(), &mut old_transport);
        let old_tag = next_tag(&old_transport);

        let mut replacement = ExternalAiCoordinator::new_for_test(&config());
        let mut transport = FakeTransport::new();
        transport.result(WorkerResult {
            tag: old_tag,
            result: Ok(ExternalAiDecision { actions: vec![] }),
        });
        assert!(
            replacement
                .advance_with_io(&runtime, admission(), &mut transport)
                .0
                .is_empty()
        );
        assert_ne!(previous.generation(), replacement.generation());

        let mut stale = ExternalAiCoordinator::new_for_test(&config());
        let mut stale_transport = FakeTransport::new();
        stale.advance_with_io(&runtime, admission(), &mut stale_transport);
        let stale_tag = next_tag(&stale_transport);
        runtime.simulation.session.binary_frame = 676;
        stale_transport.result(WorkerResult {
            tag: stale_tag,
            result: Ok(ExternalAiDecision { actions: vec![] }),
        });
        assert!(
            stale
                .advance_with_io(&runtime, admission(), &mut stale_transport)
                .0
                .is_empty()
        );
    }

    #[test]
    fn provider_failure_falls_back_and_uses_bounded_retry_backoff() {
        let mut config = config();
        config.request_interval_frames = 1;
        let (mut runtime, _) = ai_runtime();
        runtime.simulation.session.binary_frame = 1;
        let mut coordinator = ExternalAiCoordinator::new_for_test(&config);
        let mut transport = FakeTransport::new();
        coordinator.advance_with_io(&runtime, admission(), &mut transport);
        let tag = next_tag(&transport);
        runtime.simulation.session.binary_frame = 2;
        transport.result(WorkerResult {
            tag,
            result: Err(ExternalAiError::Transport),
        });

        assert!(
            coordinator
                .advance_with_io(&runtime, admission(), &mut transport)
                .0
                .is_empty()
        );
        assert_eq!(transport.submitted.len(), 1);

        runtime.simulation.session.binary_frame = 31;
        coordinator.advance_with_io(&runtime, admission(), &mut transport);
        assert_eq!(transport.submitted.len(), 1);
        runtime.simulation.session.binary_frame = 32;
        coordinator.advance_with_io(&runtime, admission(), &mut transport);
        assert_eq!(transport.submitted.len(), 2);
    }

    #[test]
    fn restored_simulation_keeps_lease_but_starts_without_in_flight_work() {
        let (mut runtime, _) = ai_runtime();
        runtime.simulation.ai_players[0].external_control_until_frame = Some(900);
        let coordinator = ExternalAiCoordinator::new_for_test(&config());

        assert_eq!(
            runtime.simulation.ai_players[0].external_control_until_frame,
            Some(900)
        );
        assert!(coordinator.in_flight_is_empty());
    }

    fn replay_runtime() -> (SimRuntime, crate::sim::intern::InternedId) {
        let (mut runtime, owner) = ai_runtime();
        runtime.simulation.session.binary_frame = 225;
        add_entity(
            &mut runtime,
            2,
            "Americans",
            "E1",
            EntityCategory::Infantry,
            5,
            5,
        );
        (runtime, owner)
    }

    #[test]
    fn accepted_batch_uses_ingress_is_recorded_exactly_and_replays_without_http() {
        let (mut runtime, owner) = replay_runtime();
        let mut coordinator = ExternalAiCoordinator::new_for_test(&config());
        let mut transport = FakeTransport::new();
        coordinator.advance_with_io(&runtime, admission(), &mut transport);
        let tag = next_tag(&transport);
        transport.result(WorkerResult {
            tag,
            result: Ok(ExternalAiDecision {
                actions: vec![ExternalAiAction::Guard {
                    entity_id: 2,
                    target_id: None,
                }],
            }),
        });
        let (accepted, _) = coordinator.advance_with_io(&runtime, admission(), &mut transport);
        runtime.simulation.queue_commands(accepted);
        let due = runtime.simulation.take_due_commands();
        let tick = runtime.simulation.advance_tick(
            &due,
            Some(&runtime.resources.rules),
            &runtime.resources.height_map,
            None,
            Some(&runtime.resources.overlay_registry),
            33,
        );

        let mut log = crate::sim::replay::ReplayLog::new(crate::sim::replay::ReplayHeader {
            pixel_conversion_bounds: Default::default(),
            version: 1,
            tick_hz: 30,
            seed: runtime.simulation.session.seed,
            map_name: runtime.simulation.session.map_name.clone(),
            rules_hash: runtime.resources.rules.simulation_config_hash(),
        });
        log.record_tick(tick.tick, due.clone(), tick.state_hash);
        assert_eq!(log.ticks[0].commands, due);
        assert_eq!(
            due.iter()
                .map(|command| &command.payload)
                .collect::<Vec<_>>(),
            vec![
                &Command::RenewExternalAiLease { until_frame: 675 },
                &Command::Guard {
                    entity_id: 2,
                    target_id: None,
                },
            ]
        );

        let (mut replayed, _) = replay_runtime();
        let hashes = crate::sim::replay::ReplayRunner::run_fixture(
            &mut replayed.simulation,
            &log,
            Some(&replayed.resources.rules),
            &replayed.resources.height_map,
            None,
            33,
        );
        assert_eq!(hashes, vec![tick.state_hash]);
        assert_eq!(
            replayed.simulation.state_hash(),
            runtime.simulation.state_hash()
        );
        assert_eq!(runtime.simulation.ai_players[0].owner, owner);
        assert_eq!(transport.submitted.len(), 1, "replay does not call the API");
    }
}
