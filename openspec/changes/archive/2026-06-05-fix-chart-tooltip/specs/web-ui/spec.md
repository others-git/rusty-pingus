## ADDED Requirements

### Requirement: Chart tooltip is robust at any hover position
The monitor detail chart tooltip SHALL render without error at any hover position, including over outage gaps where there is no data point. It SHALL show the hovered time and the response time, or "no response" when there is no value at that point.

#### Scenario: Hover over a data point
- **WHEN** the user hovers over a point on the response-time line
- **THEN** the tooltip shows that point's time and response time, with no console error

#### Scenario: Hover over an outage gap
- **WHEN** the user hovers over a time where the line is interrupted (an outage)
- **THEN** the tooltip renders showing the time and "no response", without throwing
