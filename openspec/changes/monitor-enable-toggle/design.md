## Context

`MonitorConfig` is a `#[serde(tag = "protocol")]` enum of five variants, each a struct with `name`/`interval_ms`/`timeout_ms`/etc., deserialized through a `Raw*` shim. `MonitorStore` holds the list behind an `RwLock`, persists to `monitors.toml`, and broadcasts changes on a `watch` channel. The scheduler spawns one task per monitor and, on a config change, `diff_and_reload` cancels tasks for removed names and spawns tasks for added names — it keys purely on the set of names. The dashboard derives each card from the configured monitor joined with its latest probe row.

## Goals / Non-Goals

**Goals:**
- A persisted per-monitor `enabled` flag, default true, back-compatible with existing config.
- Disabled monitors are not probed; toggling takes effect live (no restart).
- Disabling preserves config and history; enabling resumes.
- The dashboard shows the toggle and a clear "paused" state distinct from "down".

**Non-Goals:**
- Scheduling/automation of enable/disable (no cron windows) — manual toggle only.
- Purging or archiving a disabled monitor's history.
- A bulk enable/disable-all control (could follow later).

## Decisions

### Decision: `enabled: bool` on every variant, default true, skipped when true
Add `enabled` to each config struct and its `Raw*`, defaulting to `true` (`#[serde(default = "default_true")]`), and `skip_serializing_if` "is true" so enabled monitors don't write the key — existing `monitors.toml` files (no key) load as enabled and the file stays clean.
- *Why:* Backward-compatible, minimal noise in config, explicit only when paused.
- *Trade-off:* The flag is repeated across five structs (consistent with how `interval_ms`/`timeout_ms` already are). A `MonitorConfig::enabled()` accessor mirrors the existing `name()`/`interval_ms()` pattern.

### Decision: Scheduler skips disabled monitors and reacts to toggles in `diff_and_reload`
`spawn`-time: only start a task if `enabled()`. `diff_and_reload` currently diffs by name; extend it to also diff by *enabled state* for monitors present in both old and new lists: if a monitor is now disabled and has a running task, cancel it; if now enabled and has no task, spawn it. The hot-reload watch already fires on any `MonitorStore` mutation, so a toggle flows through this path.
- *Why:* Reuses the existing reload mechanism; no new control channel. Live effect without restart.
- *Trade-off:* `diff_and_reload` must track which names currently have a (running) task vs. which are merely configured — it already holds `task_tokens` keyed by name, so "has a task" is `task_tokens.contains_key(name)`, and the desired state is `enabled()`. Reconcile the two.
- *Alternative:* A dedicated per-monitor pause signal — rejected; the watch/reload path already exists and carries full config.

### Decision: Dedicated toggle endpoint; `enabled` in the status list
Add `POST /api/monitors/:name/enabled` taking `{ "enabled": bool }` (or a `PATCH`), which calls `MonitorStore::set_enabled` (persist + notify) and returns the updated list. `MonitorStatus` gains an `enabled` field sourced from config (like protocol/endpoint).
- *Why:* The existing API has only add/delete; a small explicit endpoint is clearer than overloading add. Exposing `enabled` in the list lets the dashboard render state without another call.
- *Alternative:* Reuse `POST /api/monitors` (add) with upsert semantics — rejected; add is create-only and validates uniqueness.

### Decision: Disabled = "paused" presentation, not "down"
A disabled monitor's card is de-emphasized (dimmed) with a "paused" badge, and the live status dot/strip does not show red. Status semantics: `enabled = false` is surfaced as its own state in the UI, independent of the last probe's up/down. Uptime and history are left as-is (no new probes accrue while paused; the gauges simply stop advancing).
- *Why:* A paused monitor isn't failing; showing it as down would be misleading. Keeping history means re-enabling resumes a continuous record (with a visible gap during the pause).

## Risks / Trade-offs

- **Reconciling task state on reload** → If `diff_and_reload` mis-tracks which monitors have tasks, a toggle could leave a task running while disabled (still probing) or a gap when enabled. Mitigation: compute desired (`enabled()`) vs. actual (`task_tokens` membership) per monitor and converge; cover with a focused check during verification.
- **Live updates for a paused monitor** → The dashboard SSE only updates on probe completion, which won't happen while paused — the card must reflect "paused" from the list/poll, not wait for an event. Mitigation: drive the disabled state from the `enabled` flag in the list response (poll), not from live events.
- **A monitor disabled while a probe is in flight** → The in-flight probe completes and writes one more row. Acceptable; the task is cancelled before the next tick.
- **Config round-trip** → `skip_serializing_if` must not drop `enabled = false`. Mitigation: only skip when true; verify a paused monitor persists across reload.

## Migration Plan

Frontend + config-field change; ships with embedded assets and the binary. Existing `monitors.toml` files load unchanged (missing `enabled` ⇒ enabled). No data migration. Rollback is reverting the change; any `enabled = false` keys left in config are simply ignored by the old binary (it would resume probing them).

## Open Questions

- Endpoint shape: dedicated `POST /api/monitors/:name/enabled` vs. a general `PATCH /api/monitors/:name`. Proposed: dedicated enabled endpoint now; a general PATCH can subsume it later if more editable fields appear.
- Should the detail page also carry the toggle, or only the dashboard? Proposed: dashboard card for now; detail page can mirror it later.
