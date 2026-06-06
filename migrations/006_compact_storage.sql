-- Compact storage: integer-epoch timestamps, no per-row duplication of
-- per-monitor-constant fields, and no redundant traceroute RTT column.
-- The history/status/series APIs still return RFC3339 strings and protocol/
-- endpoint (supplied from config), so the response contract is unchanged.
-- (VACUUM is run once after migrations in db::init — it cannot run inside the
-- transaction that wraps a migration.)

-- ── probe_results: checked_at TEXT → INTEGER epoch ms; drop constant columns ──
-- Indexes referencing checked_at must be dropped before the column is dropped.
DROP INDEX IF EXISTS idx_probe_results_monitor_covering;
DROP INDEX IF EXISTS idx_probe_results_checked;

ALTER TABLE probe_results ADD COLUMN checked_at_ms INTEGER;
-- julianday→Unix ms: (jd - 2440587.5) * 86400000.
UPDATE probe_results
   SET checked_at_ms = CAST(round((julianday(checked_at) - 2440587.5) * 86400000.0) AS INTEGER);
ALTER TABLE probe_results DROP COLUMN checked_at;
ALTER TABLE probe_results DROP COLUMN protocol;
ALTER TABLE probe_results DROP COLUMN endpoint;
ALTER TABLE probe_results RENAME COLUMN checked_at_ms TO checked_at;

CREATE INDEX IF NOT EXISTS idx_probe_results_monitor_covering
    ON probe_results (monitor_name, checked_at, status, response_time_ms);
CREATE INDEX IF NOT EXISTS idx_probe_results_checked
    ON probe_results (checked_at DESC);

-- ── traceroute_runs: checked_at TEXT → INTEGER epoch ms ──────────────────────
DROP INDEX IF EXISTS idx_traceroute_runs_monitor_checked;

ALTER TABLE traceroute_runs ADD COLUMN checked_at_ms INTEGER;
UPDATE traceroute_runs
   SET checked_at_ms = CAST(round((julianday(checked_at) - 2440587.5) * 86400000.0) AS INTEGER);
ALTER TABLE traceroute_runs DROP COLUMN checked_at;
ALTER TABLE traceroute_runs RENAME COLUMN checked_at_ms TO checked_at;

CREATE INDEX IF NOT EXISTS idx_traceroute_runs_monitor_checked
    ON traceroute_runs (monitor_name, checked_at DESC);

-- ── traceroute_hops: drop redundant rtt_us (it duplicated avg_us) ────────────
ALTER TABLE traceroute_hops DROP COLUMN rtt_us;
