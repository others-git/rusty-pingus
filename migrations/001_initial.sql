CREATE TABLE IF NOT EXISTS probe_results (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    monitor_name    TEXT    NOT NULL,
    protocol        TEXT    NOT NULL,
    endpoint        TEXT    NOT NULL,
    status          TEXT    NOT NULL CHECK (status IN ('up', 'down')),
    response_time_ms INTEGER,
    failure_reason  TEXT,
    checked_at      TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_probe_results_monitor_checked
    ON probe_results (monitor_name, checked_at DESC);

CREATE INDEX IF NOT EXISTS idx_probe_results_checked
    ON probe_results (checked_at DESC);
