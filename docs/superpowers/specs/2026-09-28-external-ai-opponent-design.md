# External AI opponent design

## Goal and scope

Let a player use an OpenAI-compatible HTTP API as the strategic controller for
computer houses in a single-player skirmish. The API chooses production and
ordinary unit orders; VERA20k remains the authority for rules, legality,
simulation state, and command execution. Multiplayer and independent API calls
from multiple lockstep peers are out of scope.

The feature is opt-in. With external AI disabled or no endpoint configured,
the existing deterministic computer AI runs unchanged. Native-derived house
strategy, automatic base construction, and other existing owners remain in
place; this feature replaces only the placeholder strategic production and
attack-wave decisions in `sim::ai::tick_ai` while a valid remote-control lease
is active.

## Approaches considered

1. **Call the API from the simulation frame.** This is small but a slow or
   unavailable service would stall gameplay and make frame advancement depend
   on network timing. It also violates the `sim/` dependency boundary.
2. **Require a separate local proxy process.** This allows custom providers,
   but adds installation, lifecycle, and IPC requirements the user did not
   request.
3. **Use an app-owned background worker (selected).** The app sends immutable
   observations to one worker, which performs HTTP requests off the frame
   thread. Valid actions return through VERA20k's existing command queue.

## Runtime architecture

- Add an optional `external_ai` section to `GameConfig` and its example. It
  selects the compatible chat-completions endpoint, model, request cadence,
  request timeout, maximum actions per response, and control-lease length. It
  defaults to disabled; initial defaults are 225 game frames per request,
  15 seconds per request, 16 actions per response, and a 450-frame control
  lease.
- Read an optional API key from a named process environment variable (default
  `VERA20K_AI_API_KEY`); do not store the secret in `GameConfig`, log it, or
  include it in replay/save data. Omit the authorization header when the
  configured endpoint does not need a key. The endpoint and model remain in
  the machine-local, gitignored `config.toml`.
- Put the API coordinator and HTTP worker in `app/`, using a bounded channel
  and the existing worker/channel pattern. The worker receives owned request
  data and never reads or mutates `Simulation`. There is at most one request
  in flight per computer house. Network work never blocks a simulation frame.
- Build an immutable, deterministically ordered observation on the app thread
  from the current `SimRuntime`. Include the AI house's credits, production
  queues, owned units/buildings, and currently visible enemy facts. Do not send
  hidden enemy identities/positions, machine paths, local-player profile data,
  or API credentials.
- Request decisions at a configurable game-frame cadence (initial default:
  225 `binary_frame`s per house), not once per render or simulation tick.
  Tag each request with a match generation, owner, and source frame. Drop
  results from a replaced/ended match and reject results older than their
  control lease.
- Parse only a bounded JSON action list. The first version supports production
  queue choices and existing unit-order commands (`Move`, `AttackMove`, and
  `Guard`). No shell, file, tool, or arbitrary code actions are accepted.
- Validate every action against the current owner, live entity state, current
  vision, map bounds, available production choices, and a per-response action
  limit. Invalid actions are rejected individually; malformed responses renew
  no lease. The simulation's normal command admission remains the final
  authority.
- Queue a successful response's lease marker and accepted commands as
  `CommandEnvelope`s for the next normal ingress. The app's existing replay log
  records that exact batch and resulting hash. Replay uses the recorded batch
  and never calls the API.
- Store the lease in the owning AI player's simulation state. A replayable
  lease command extends it; while active, `tick_ai` skips that owner. If
  requests fail, time out, or return invalid output, no lease is renewed; when
  it expires, the existing deterministic AI automatically resumes. Before the
  first valid remote response, the existing AI continues to play.
- Tear down the worker at match replacement/exit and invalidate its generation
  token. On save restore, restore the simulation lease and start a fresh
  app-side coordinator; do not persist in-flight HTTP work.

## Request protocol and failures

Use the configured OpenAI-compatible chat-completions endpoint and read the
assistant message's content as JSON. Do not require provider-specific tool
calling or structured-output extensions. The prompt contains only the
observation and the allowed action schema. Treat HTTP errors, timeouts,
non-success status codes, malformed JSON, oversized responses, and stale
results as failed decisions. Log a redacted provider error and continue the
match; never log authorization headers or response secrets. Apply bounded
retry backoff so an unavailable service does not generate a request every
frame.

The external API is opt-in because match observations leave the local machine.
The first version targets the configured endpoint and all eligible AI houses
in a single-player skirmish; request cadence and timeout are local settings.
The sample config must make the opt-in and API destination clear before the
user enables it.

## Ownership and touched areas

- `src/util/config.rs` owns optional endpoint/model/cadence/timeout settings.
- `src/app/match_runtime/` owns the worker lifecycle, observation creation,
  response validation, and handoff into command ingress.
- `src/sim/ai.rs` and the existing simulation-owned AI state own per-house
  lease suppression and expiry; the app does not create parallel gameplay
  state.
- `src/sim/command.rs`, command application, state hashing, snapshots, and
  replay handling own the serialized lease event and the resulting command
  history.

The HTTP client dependency must be selected only after checking its current
cross-platform support and minimum Rust version against the project's Rust
1.88 baseline. The full API key must not enter any `Debug` representation.

## Validation

- Config tests cover missing/disabled settings, valid endpoint values, defaults,
  and missing credentials without affecting ordinary startup.
- Protocol tests cover valid responses, malformed/oversized responses,
  unexpected action variants, and request/response identity.
- Observation tests prove deterministic ordering and exclusion of hidden enemy
  state and local machine data.
- Action tests cover ownership, stale entity IDs, visibility, map bounds,
  production eligibility, and action-count limits.
- Lease tests prove that a valid API batch suppresses only its owner's
  placeholder AI, and that expiration restores it.
- Replay tests prove recorded lease/action batches reproduce state hashes
  without starting or contacting the API worker.
- A local mock HTTP endpoint exercises the production request client, timeout,
  and redacted error path; no live API key is required.
- Run the repository-required Rust validation on the final candidate, including
  retail-INI library tests and Clippy. Do not claim external-provider parity;
  provider output is nondeterministic, while a recorded command batch is
  replayable.

## Risks and limits

- Provider latency means remote control can be delayed; the local AI continues
  until it receives a valid lease and resumes after lease expiry.
- Different compatible providers can vary in response envelope details. Keep
  parsing limited to the documented chat-completions response shape and report
  unsupported variants without crashing the match.
- Hidden-state filtering and action validation are correctness boundaries,
  not prompt instructions; they must be enforced by local code.
- This feature does not establish deterministic AI behavior across clients.
  Multiplayer would require one authoritative command producer and distribution
  of its recorded command batches.
