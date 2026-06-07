## Context

`probe/traceroute.rs` returns `ProbeResult::down(&name, "traceroute", &endpoint, "privilege_error")` when the raw socket can't be opened (and other clean reasons like `dns_error`, `ipv6_unsupported`). That `failure_reason` is persisted and exposed in the monitor-list (`MonitorStatus.failure_reason`) and history responses. The dashboard card renders the status badge and the type-specific `detail`, but not `failure_reason`; the detail page header shows status/detail. So a privilege failure currently looks like a plain "down".

## Goals / Non-Goals

**Goals:**
- A traceroute that can't run due to privileges is visibly distinct from a network-down monitor, with a short explanation.
- Real failures (timeout/unreachable) are unaffected.
- Frontend-only if possible (the reason is already available).

**Non-Goals:**
- Auto-acquiring privileges or changing how the probe runs.
- A new status enum value in storage (keep status `down`; surface via the reason).
- Surfacing for non-traceroute monitors (their failures are genuine network states).

## Decisions

### Decision: Map the privilege/socket failure reason to a distinct UI state (no backend change)
The frontend detects, for a `traceroute` monitor whose latest status is down with `failure_reason` indicating a socket/privilege problem (`privilege_error`, and the related `socket_error`/`join_error`), and renders a distinct "unavailable — requires elevated privileges (CAP_NET_RAW)" treatment: a badge/icon variant on the dashboard card (with a tooltip) and an explanatory notice on the detail page. The check keys on the stable reason token already produced by the probe.
- *Why:* The signal already exists end-to-end; this is purely a presentation refinement. Keeping status as `down` avoids storage/contract changes.
- *Trade-off:* The frontend hard-codes the set of "unavailable" reason tokens; if the probe adds new ones they must be added here. Mitigation: centralize the check in one helper.
- *Alternative:* Introduce a dedicated `unavailable` status persisted in the DB — rejected as over-engineering for a presentation need.

### Decision: Distinct but not alarming styling
Render the unavailable state in a neutral/warning tone (e.g. amber/slate with a key/lock or warning icon) rather than the red "down" treatment, since it's a configuration issue, not an outage.
- *Why:* Distinguishes "you need to fix privileges" from "the target is down".

## Risks / Trade-offs

- **Reason-token drift** → If `probe/traceroute.rs` changes its reason strings, the UI mapping breaks. Mitigation: one shared helper listing the unavailable tokens; note the coupling in a comment near the probe's reasons.
- **Only the latest probe is considered** → If it later succeeds, the state clears naturally (driven by the latest status). Acceptable.
- **Visual verification only** → Needs a browser/privilege-denied run to see; can be exercised by running unprivileged (the probe already returns `privilege_error`).

## Migration Plan

Frontend-only; ships with embedded assets. No data/API/config change; trivially revertable.

## Open Questions

- Should the same "needs privileges" hint also appear in the add-monitor flow when creating a traceroute monitor (proactive), or only after a failed probe (reactive)? Proposed: reactive only for now.
