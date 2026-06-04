-- Performance: make uptime and series aggregations index-only on large datasets.
--
-- The previous (monitor_name, checked_at) index forced table-heap lookups to read
-- `status` and `response_time_ms` for every row in a range (millions, at low poll
-- intervals). A covering index that also carries those columns lets COUNT/SUM and
-- AVG/MIN/MAX run entirely from the index. Its prefix supersedes the old index
-- (reverse scans serve history's ORDER BY checked_at DESC), so we drop that one.

DROP INDEX IF EXISTS idx_probe_results_monitor_checked;

CREATE INDEX IF NOT EXISTS idx_probe_results_monitor_covering
    ON probe_results (monitor_name, checked_at, status, response_time_ms);
