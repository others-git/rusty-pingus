-- Per-minute rollups of probe results, so wide chart windows and long uptime
-- windows aggregate ~minutes instead of millions of raw rows. Storing sum_ms +
-- up_count (not a pre-divided average) lets coarser re-bucketing stay exact.
CREATE TABLE IF NOT EXISTS probe_rollup_1m (
    monitor_name TEXT    NOT NULL,
    bucket_epoch INTEGER NOT NULL,   -- unix seconds at minute start (multiple of 60)
    count        INTEGER NOT NULL,   -- total probes in the minute
    up_count     INTEGER NOT NULL,   -- 'up' probes
    sum_ms       INTEGER NOT NULL,   -- sum of response_time_ms over 'up' probes
    min_ms       INTEGER,            -- over 'up' probes
    max_ms       INTEGER,
    PRIMARY KEY (monitor_name, bucket_epoch)
) WITHOUT ROWID;
