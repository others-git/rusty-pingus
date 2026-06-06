-- Traceroute monitor storage, kept separate from probe_results to avoid bloating
-- the hot status table. Designed for compactness on high-volume, route-stable
-- data: hop addresses are interned (stored once, referenced by id) and RTTs are
-- stored as integer microseconds. A lightweight summary row is still written to
-- probe_results so the monitor appears on the dashboard/live stream.

-- Interned distinct hop/router addresses. The same handful of addresses recur
-- across every run of a stable route, so storing them once is the dominant win.
CREATE TABLE IF NOT EXISTS traceroute_addrs (
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    addr TEXT    NOT NULL UNIQUE
);

-- One row per traceroute run.
CREATE TABLE IF NOT EXISTS traceroute_runs (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    monitor_name TEXT    NOT NULL,
    checked_at   TEXT    NOT NULL,
    reached      INTEGER NOT NULL,   -- 1 if the destination replied, else 0
    hop_count    INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_traceroute_runs_monitor_checked
    ON traceroute_runs (monitor_name, checked_at DESC);

-- One row per hop per run. A non-responding hop has addr_id and rtt_us NULL and
-- loss reflecting the missing replies. RTT statistics are integer microseconds.
CREATE TABLE IF NOT EXISTS traceroute_hops (
    run_id  INTEGER NOT NULL REFERENCES traceroute_runs(id) ON DELETE CASCADE,
    hop_no  INTEGER NOT NULL,
    addr_id INTEGER REFERENCES traceroute_addrs(id),
    rtt_us  INTEGER,                 -- representative (avg) RTT for the hop, microseconds
    min_us  INTEGER,
    avg_us  INTEGER,
    max_us  INTEGER,
    loss    INTEGER NOT NULL DEFAULT 0, -- queries with no reply at this hop, this run
    PRIMARY KEY (run_id, hop_no)
);

CREATE INDEX IF NOT EXISTS idx_traceroute_hops_run ON traceroute_hops (run_id);
CREATE INDEX IF NOT EXISTS idx_traceroute_hops_addr ON traceroute_hops (addr_id);
