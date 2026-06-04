## ADDED Requirements

### Requirement: Aggregated probe series query
The system SHALL provide a query that aggregates a monitor's probe results over a `[from, to]` time range into a bounded number of time buckets. Each bucket SHALL report the bucket start time, average/min/max response time, sample count, and the fraction of `up` results (up-ratio). The number of buckets returned SHALL NOT exceed the requested target, regardless of how many raw results fall in the range.

#### Scenario: Results bucketed over a range
- **WHEN** an aggregated series is requested for a monitor over a `[from, to]` range with a target bucket count N
- **THEN** results are grouped into at most N time buckets, each reporting bucket start, avg/min/max response time, sample count, and up-ratio

#### Scenario: Bounded output regardless of volume
- **WHEN** a monitor has a very large number of raw results in the range (e.g. sub-second polling over many days)
- **THEN** the query still returns at most the requested number of buckets

#### Scenario: Empty range
- **WHEN** no results exist in the requested range
- **THEN** the query returns an empty set

## MODIFIED Requirements

### Requirement: Result retention policy
The system SHALL support a configurable retention period. Probe results older than the retention period SHALL be automatically purged. When no retention period is configured, the system SHALL apply a default retention of 90 days rather than retaining results indefinitely. An operator MAY set an explicit retention period (including a larger value) to override the default.

#### Scenario: Old results pruned
- **WHEN** the retention period is set to `90d` and results older than 90 days exist
- **THEN** those results are deleted from the database during a periodic cleanup task

#### Scenario: Default retention applied when unconfigured
- **WHEN** no retention period is configured
- **THEN** the system prunes results older than the default retention (90 days) during the periodic cleanup task

#### Scenario: Explicit override
- **WHEN** an operator sets an explicit retention period
- **THEN** that value is used instead of the default
