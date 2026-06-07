## Context

`_renderHopChart` in `assets/monitor.js` builds the traceroute latency graph with ECharts: a `custom` series whose `renderItem` draws one rounded rectangle per hop spanning that hop's `min_ms`→`max_ms` (at the hop's category-axis position), plus a `line` series of the per-hop averages with circle symbols. Hops are categories on the (inverse) Y axis; latency is the value X axis.

## Goals / Non-Goals

**Goals:**
- One continuous filled min–max shape across hops, not separate per-hop rectangles.
- Keep the average line/dots and the hover tooltip.
- A non-responding hop breaks the ribbon (no false 0 ms pinch).

**Non-Goals:**
- Changing the data, the brush, the table, or any other monitor type.
- Switching charting libraries.

## Decisions

### Decision: One polygon for the ribbon instead of per-hop rectangles
Replace the per-hop rectangle `renderItem` with a single custom series that draws one polygon per contiguous responding run. At each hop, the left edge is at `x = -min_ms` (mirrored onto the negative side of a centered axis) and the right edge is at `x = +max_ms`. Walk top→bottom along the left (min) edge, then bottom→top along the right (max) edge, close — yielding a continuous filled band. Use `api.coord` per hop to convert data coordinates to pixels; fill with the existing translucent cyan.
- *Why:* A single closed path reads as one shape. Adjacent hops with different spreads naturally produce diagonal edges, giving the "continuous blob that widens/narrows" look.
- *Trade-off:* The `renderItem` emits one group of polygons built from the hops closure; per-hop items in `bandData` are kept for axis-trigger tooltip purposes.

### Decision: Centered X axis (min mirrored left, max right)
The X axis is centered at 0: `min: -maxX, max: +maxX`. Each hop's ribbon spans `[-min_ms, +max_ms]`. The average line series is still plotted at `+avg_ms` (positive side, inside the ribbon). A hairline `markLine` at `xAxis: 0` provides the visual centre reference.
- *Why:* Centering makes the chart symmetric and intuitive — the left lobe shows "how low latency can go" and the right lobe shows "how high". A narrow left lobe with a wide right lobe instantly reads as high jitter.
- *Trade-off:* The average dot no longer sits at the geometric center of the ribbon (it's always on the positive side). This is intentional: avg_ms is always ≥ 0, and the chart correctly places it at its true latency value.

### Decision: Break the ribbon at non-responding hops
Where a hop has null min/max, split the polygon into separate filled segments on each side of the gap (don't bridge across the missing hop, and don't plot it as 0). The average line already uses `connectNulls: false`, so it breaks consistently.
- *Why:* Bridging or zero-pinching at a dead hop would misrepresent latency. Segmenting keeps the ribbon honest and visually matches the broken average line.
- *Trade-off:* Slightly more path bookkeeping (emit a polygon per contiguous run of responding hops).

## Risks / Trade-offs

- **Coordinate/extent correctness** → The polygon must use the same axis mapping as the average line so they overlay exactly. Mitigation: build both from the same per-hop `api.coord`/value mapping; verify the average dots sit within the ribbon.
- **Single-hop or all-equal min=max** → A zero-width ribbon segment should still render a thin line, not vanish. Mitigation: enforce a minimum visual width as the current band does.
- **Visual verification only** → This is a look change; correctness is judged by eye. Mitigation: note it needs a browser check.

## Migration Plan

Frontend-only; ships with embedded assets (release rebuild to re-embed; debug serves from disk). Trivially revertable.

## Open Questions

- None.
