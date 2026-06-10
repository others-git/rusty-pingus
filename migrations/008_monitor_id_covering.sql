-- Reads have keyed probe_results by monitor_id since migration 007, but the
-- covering index from 002/006 still keys by monitor_name: every aggregation
-- over monitor_id (uptime, series, rollup backfill) pays a table-heap lookup
-- per row to fetch status/response_time_ms — the exact cost migration 002
-- eliminated — while every insert maintains a wide name-keyed index no query
-- uses anymore. Replace both monitor-keyed indexes with one covering index on
-- monitor_id so those aggregations are index-only again.
--
-- idx_probe_results_checked is kept: the rollup loop aggregates by a bare
-- checked_at range (no monitor filter), which needs the time-keyed index.
-- (Space freed by the drops is reclaimed by the post-migration VACUUM in
-- db::init.)

DROP INDEX IF EXISTS idx_probe_results_monitor_covering;
DROP INDEX IF EXISTS idx_probe_results_mid_checked;

CREATE INDEX IF NOT EXISTS idx_probe_results_mid_covering
    ON probe_results (monitor_id, checked_at, status, response_time_ms);
