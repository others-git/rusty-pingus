## Purpose

Defines the JSON API for managing monitor definitions at runtime: listing configured monitors, adding new ones with input validation, and deleting existing ones.

## Requirements

### Requirement: List configured monitors via API
The system SHALL expose `GET /api/monitors/config` returning the full list of configured monitors (from the monitor store, not probe results).

#### Scenario: Returns all configured monitors
- **WHEN** `GET /api/monitors/config` is requested
- **THEN** a JSON array of all monitor definitions is returned, each containing `name`, `protocol`, and protocol-specific fields

#### Scenario: Returns empty array when no monitors configured
- **WHEN** no monitors are defined in monitors.toml
- **THEN** `GET /api/monitors/config` returns `[]` with HTTP 200

### Requirement: Add a monitor via API
The system SHALL expose `POST /api/monitors` accepting a JSON body describing a new monitor. Timing fields in the body SHALL be `interval_ms` and `timeout_ms` (milliseconds). On success, the monitor is added to the store, persisted, and the updated list returned.

#### Scenario: Successful HTTP monitor creation
- **WHEN** `POST /api/monitors` is called with `{"name":"my-api","protocol":"http","url":"https://example.com","interval_ms":60000}`
- **THEN** HTTP 201 is returned with the full updated monitor list

#### Scenario: Successful TCP monitor creation
- **WHEN** `POST /api/monitors` is called with `{"name":"my-db","protocol":"tcp","host":"db.example.com","port":5432,"interval_ms":30000}`
- **THEN** HTTP 201 is returned with the updated monitor list

#### Scenario: Successful ICMP monitor creation
- **WHEN** `POST /api/monitors` is called with `{"name":"gateway","protocol":"icmp","host":"192.168.1.1","interval_ms":30000}`
- **THEN** HTTP 201 is returned with the updated monitor list

### Requirement: Input validation on monitor creation
The system SHALL validate all fields on `POST /api/monitors` and return `422 Unprocessable Entity` with structured errors if validation fails. Timing bounds SHALL be expressed in milliseconds: `interval_ms` at least 5000, `timeout_ms` at least 1000, and `timeout_ms` less than `interval_ms`.

#### Scenario: Missing required name field
- **WHEN** `POST /api/monitors` is called without a `name` field
- **THEN** HTTP 422 is returned with `{"errors":["name is required"]}`

#### Scenario: Duplicate monitor name rejected
- **WHEN** `POST /api/monitors` is called with a name that already exists in the store
- **THEN** HTTP 422 is returned with `{"errors":["a monitor named '<name>' already exists"]}`

#### Scenario: Invalid URL for HTTP monitor
- **WHEN** `POST /api/monitors` is called with `"url":"not-a-url"` for an HTTP monitor
- **THEN** HTTP 422 is returned with `{"errors":["url must be a valid http or https URL"]}`

#### Scenario: Invalid port for TCP monitor
- **WHEN** `POST /api/monitors` is called with `"port":0` or `"port":99999` for a TCP monitor
- **THEN** HTTP 422 is returned with `{"errors":["port must be between 1 and 65535"]}`

#### Scenario: Interval too short
- **WHEN** `POST /api/monitors` is called with `"interval_ms":3000`
- **THEN** HTTP 422 is returned with `{"errors":["interval_ms must be at least 5000"]}`

#### Scenario: Timeout too short
- **WHEN** `POST /api/monitors` is called with `"timeout_ms":500`
- **THEN** HTTP 422 is returned with `{"errors":["timeout_ms must be at least 1000"]}`

#### Scenario: Timeout exceeds interval
- **WHEN** `POST /api/monitors` is called with `"timeout_ms":60000,"interval_ms":30000`
- **THEN** HTTP 422 is returned with `{"errors":["timeout_ms must be less than interval_ms"]}`

#### Scenario: Multiple validation errors returned together
- **WHEN** a request has multiple invalid fields
- **THEN** all validation errors are returned in a single `errors` array in the 422 response

### Requirement: Delete a monitor via API
The system SHALL expose `DELETE /api/monitors/:name` to remove a monitor by name. On success, the monitor is removed from the store, persisted, its stored probe results are purged from the database, and the updated list is returned. After deletion the monitor SHALL NOT reappear in the dashboard's status list.

#### Scenario: Successful deletion
- **WHEN** `DELETE /api/monitors/my-api` is called for an existing monitor
- **THEN** HTTP 200 is returned with the updated monitor list (not including the deleted monitor)

#### Scenario: Deleted monitor's history is purged
- **WHEN** `DELETE /api/monitors/my-api` is called for a monitor that has stored probe results
- **THEN** all `probe_results` rows for `my-api` are removed, and `GET /api/monitors` no longer includes `my-api`

#### Scenario: Delete non-existent monitor
- **WHEN** `DELETE /api/monitors/nonexistent` is called
- **THEN** HTTP 404 is returned with `{"error":"monitor not found"}`
