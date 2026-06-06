-- Per-probe, type-specific context (nullable). Holds free-form human-readable
-- detail like a public-IP monitor's observed address or a border monitor's fault
-- localization. Existing rows and types that record no context stay NULL.
ALTER TABLE probe_results ADD COLUMN detail TEXT;
