## Why

Hovering the monitor detail chart throws `Uncaught TypeError: Cannot read properties of undefined (reading '0')` in the ECharts axis tooltip `formatter`. The formatter assumes `params[0].value` is the `[ts, y]` array, but with an axis-trigger tooltip ECharts can pass items that have no `value` (e.g. over a line gap/outage where the point is null, or non-data components), so `p.value[0]` dereferences `undefined`.

## What Changes

- Make the chart tooltip `formatter` defensive: derive the timestamp from `axisValue` (the hovered axis value) and read `y` from `value` only when it is an array; render "no response" when there is no value. Never index into a possibly-undefined `value`.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `web-ui`: The monitor detail chart tooltip renders without error at any hover position, including over outage gaps.

## Impact

- `assets/monitor.js` — rewrite the `tooltip.formatter` to guard against items without a `value`; use `axisValue` for the time.
- No backend changes.
