use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::thread;
use std::time::Duration;

use crate::sim::intern::InternedId;
use crate::util::config::ExternalAiConfig;

use super::client::ExternalAiClient;
use super::protocol::{ExternalAiDecision, ExternalAiError};

const API_KEY_ENV: &str = "VERA20K_AI_API_KEY";
/// The game supports at most 30 players, so one queued/in-flight job per house
/// fits without allowing the app to accumulate an unbounded network backlog.
const WORKER_QUEUE_CAPACITY: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RequestTag {
    pub(super) match_generation: u64,
    pub(super) owner: InternedId,
    pub(super) source_frame: u32,
}

pub(super) struct WorkerResult {
    pub(super) tag: RequestTag,
    pub(super) result: Result<ExternalAiDecision, ExternalAiError>,
}

struct WorkerJob {
    tag: RequestTag,
    observation: serde_json::Value,
    max_actions: usize,
}

pub(super) struct ExternalAiWorker {
    jobs: SyncSender<WorkerJob>,
    results: Receiver<WorkerResult>,
    cancelled: Arc<AtomicBool>,
}

impl ExternalAiWorker {
    /// Start the app-owned worker. The environment credential and HTTP client
    /// are moved directly to the thread and never enter simulation state.
    pub(super) fn start(config: &ExternalAiConfig) -> Result<Self, ExternalAiError> {
        if !config.enabled {
            return Err(ExternalAiError::InvalidConfiguration);
        }
        config
            .validate()
            .map_err(|_| ExternalAiError::InvalidConfiguration)?;

        let endpoint = config.endpoint.clone();
        let model = config.model.clone();
        let timeout = Duration::from_secs(u64::from(config.request_timeout_secs));
        let api_key = std::env::var(API_KEY_ENV)
            .ok()
            .filter(|key| !key.trim().is_empty());
        let (jobs, job_receiver) = mpsc::sync_channel(WORKER_QUEUE_CAPACITY);
        let (result_sender, results) = mpsc::sync_channel(WORKER_QUEUE_CAPACITY);
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);

        thread::Builder::new()
            .name("external-ai-api".to_string())
            .spawn(move || {
                let client = ExternalAiClient::new(&endpoint, &model, timeout, api_key.as_deref());
                run_worker(job_receiver, result_sender, worker_cancelled, client);
            })
            .map_err(|_| ExternalAiError::WorkerStart)?;

        Ok(Self {
            jobs,
            results,
            cancelled,
        })
    }

    /// Enqueue without waiting for either a free slot or network I/O.
    pub(super) fn try_submit(
        &self,
        tag: RequestTag,
        observation: serde_json::Value,
        max_actions: usize,
    ) -> Result<(), ExternalAiError> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(ExternalAiError::WorkerStopped);
        }
        match self.jobs.try_send(WorkerJob {
            tag,
            observation,
            max_actions,
        }) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(ExternalAiError::WorkerQueueFull),
            Err(TrySendError::Disconnected(_)) => Err(ExternalAiError::WorkerStopped),
        }
    }

    /// Collect one completed result without waiting for the worker.
    pub(super) fn try_receive(&self) -> Result<Option<WorkerResult>, ExternalAiError> {
        match self.results.try_recv() {
            Ok(result) => Ok(Some(result)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(ExternalAiError::WorkerStopped),
        }
    }
}

impl Drop for ExternalAiWorker {
    fn drop(&mut self) {
        // Do not join on the event thread. An active call leaves the worker
        // after its configured ureq timeout; dropping the channels releases
        // queued jobs and any result send blocked on a full channel.
        self.cancelled.store(true, Ordering::Release);
    }
}

fn run_worker(
    jobs: Receiver<WorkerJob>,
    results: SyncSender<WorkerResult>,
    cancelled: Arc<AtomicBool>,
    client: ExternalAiClient,
) {
    while let Ok(job) = jobs.recv() {
        if cancelled.load(Ordering::Acquire) {
            break;
        }
        let result = client.request(&job.observation, job.max_actions);
        if cancelled.load(Ordering::Acquire) {
            break;
        }
        if results
            .send(WorkerResult {
                tag: job.tag,
                result,
            })
            .is_err()
        {
            break;
        }
    }
}
