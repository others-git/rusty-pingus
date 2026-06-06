## ADDED Requirements

### Requirement: Add public-IP and border monitors from the UI
The add-monitor modal SHALL offer the public-IP and border monitor types alongside HTTP/TCP/ICMP, showing the fields relevant to each (public-IP: an optional service URL; border: an optional gateway and an upstream target), and submit them to `POST /api/monitors`.

#### Scenario: Public-IP type selectable
- **WHEN** a user opens the add-monitor modal and selects the public-IP type
- **THEN** the form shows the public-IP fields and can create a public-IP monitor

#### Scenario: Border type selectable
- **WHEN** a user selects the border type
- **THEN** the form shows gateway and upstream fields and can create a border monitor

### Requirement: Display per-probe detail
The dashboard and monitor detail views SHALL surface a probe's detail when present — e.g. the current public IP for a public-IP monitor, or the fault localization for a border monitor — without disrupting the existing card/detail layout.

#### Scenario: Public IP shown
- **WHEN** a public-IP monitor has a recorded IP
- **THEN** that IP is shown on its card / detail view

#### Scenario: Border status shown
- **WHEN** a border monitor has a fault classification (e.g. ISP down vs LAN down)
- **THEN** that classification is shown on its card / detail view

#### Scenario: No detail, no clutter
- **WHEN** a monitor has no detail (e.g. a plain HTTP check)
- **THEN** no extra detail element is shown for it
