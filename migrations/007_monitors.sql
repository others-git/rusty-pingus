-- Monitors move from monitors.toml into the database, identified by a stable
-- integer id so a monitor's history survives a rename. The full config is stored
-- as a JSON blob; `name` is denormalized for the UNIQUE constraint and cheap
-- listing.
CREATE TABLE IF NOT EXISTS monitors (
    id     INTEGER PRIMARY KEY AUTOINCREMENT,
    name   TEXT    NOT NULL UNIQUE,
    config TEXT    NOT NULL   -- JSON-serialised MonitorConfig
);

-- History tables gain a stable monitor_id. It is nullable here and backfilled at
-- startup from the existing monitor_name (which is kept, denormalised, so the
-- NOT NULL column keeps working). All reads/keying switch to monitor_id.
ALTER TABLE probe_results   ADD COLUMN monitor_id INTEGER;
ALTER TABLE traceroute_runs ADD COLUMN monitor_id INTEGER;

CREATE INDEX IF NOT EXISTS idx_probe_results_mid_checked
    ON probe_results (monitor_id, checked_at DESC);
CREATE INDEX IF NOT EXISTS idx_traceroute_runs_mid_checked
    ON traceroute_runs (monitor_id, checked_at DESC);

-- Rollups are derived data, so rather than migrate rows we rebuild the table
-- keyed by monitor_id; the rollup loop repopulates it from probe_results on the
-- next startup (after monitor_id has been backfilled).
DROP TABLE IF EXISTS probe_rollup_1m;
CREATE TABLE probe_rollup_1m (
    monitor_id   INTEGER NOT NULL,
    bucket_epoch INTEGER NOT NULL,   -- unix seconds at minute start (multiple of 60)
    count        INTEGER NOT NULL,   -- total probes in the minute
    up_count     INTEGER NOT NULL,   -- 'up' probes
    sum_ms       INTEGER NOT NULL,   -- sum of response_time_ms over 'up' probes
    min_ms       INTEGER,            -- over 'up' probes
    max_ms       INTEGER,
    PRIMARY KEY (monitor_id, bucket_epoch)
) WITHOUT ROWID;
