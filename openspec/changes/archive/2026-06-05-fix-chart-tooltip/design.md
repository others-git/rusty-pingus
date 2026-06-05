## Context

The ECharts response-time chart uses an axis-trigger tooltip whose `formatter(params)` does `new Date(params[0].value[0])`. With `trigger: 'axis'`, ECharts passes an array of items for the hovered x; an item's `value` is only the `[tsMs, y]` data array when there is a data point there. Over a line gap (null y at an outage) or for non-series contributions, `value` can be `undefined`, so `value[0]` throws. The hovered time is always available as `params[0].axisValue` regardless.

## Goals / Non-Goals

**Goals:** tooltip never throws; shows the hovered time and the response (or "no response" at gaps).
**Non-Goals:** changing tooltip content/styling beyond the guard; any backend change.

## Decisions

### Defensive formatter
Use `axisValue` for the timestamp and treat `value` as optional:
```js
formatter: (params) => {
  const arr = Array.isArray(params) ? params : [params];
  if (!arr.length) return '';
  const p = arr[0];
  const tsMs = p.axisValue != null ? p.axisValue
             : (Array.isArray(p.value) ? p.value[0] : null);
  const y = Array.isArray(p.value) ? p.value[1] : null;
  const time = tsMs != null ? new Date(tsMs).toLocaleString() : '';
  const body = y == null ? 'no response' : `${Math.round(y)} ms`;
  return time ? `${time}<br/>${body}` : body;
}
```
This removes the unconditional `value[0]` dereference, the root of the crash, and correctly labels outage points as "no response".

## Risks / Trade-offs

- Browser-only verification (hover behavior can't be tested headlessly); confirm by hovering across data and an outage gap.

## Open Questions

_None._
