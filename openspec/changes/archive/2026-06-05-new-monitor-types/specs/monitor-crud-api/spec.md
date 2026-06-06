## ADDED Requirements

### Requirement: Create public-IP and border monitors via API
The system SHALL accept `publicip` and `border` monitors on `POST /api/monitors`, validating their type-specific fields and returning `422` with structured errors on failure (consistent with the existing add-monitor validation).

#### Scenario: Create a public-IP monitor
- **WHEN** `POST /api/monitors` is called with `{"name":"my-ip","protocol":"publicip"}` (optionally a `url`)
- **THEN** HTTP 201 is returned with the updated monitor list

#### Scenario: Create a border monitor
- **WHEN** `POST /api/monitors` is called with `{"name":"border","protocol":"border","upstream":"1.1.1.1"}` (optionally a `gateway`)
- **THEN** HTTP 201 is returned with the updated monitor list

#### Scenario: Invalid public-IP URL
- **WHEN** a `publicip` monitor is posted with a `url` that is not http(s)
- **THEN** HTTP 422 is returned with a validation error for `url`

#### Scenario: Invalid border target
- **WHEN** a `border` monitor is posted with a `gateway` or `upstream` that is not a valid IP/host
- **THEN** HTTP 422 is returned with a validation error for that field
