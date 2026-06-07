## Why

The traceroute detail page draws each hop's min–max latency as a separate horizontal band with the averages joined by a line. As discrete per-hop rectangles it reads as a stack of bars rather than a single picture of how latency spreads down the path. Connecting the bands into one continuous filled shape ("blob") makes the min/max envelope and where it widens read at a glance.

## What Changes

- **Render the per-hop latency as one continuous ribbon** instead of discrete per-hop bands: a single filled area whose left edge follows each hop's minimum and right edge follows each hop's maximum, connected vertically across hops, with the average still drawn as a line of dots through it.
- **Handle gaps sanely:** a non-responding hop (no min/avg/max) breaks the ribbon rather than collapsing it to zero, so it doesn't draw a misleading pinch to 0 ms.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- **web-ui** — the traceroute per-hop latency graph renders as a continuous min–max ribbon (filled band connected across hops) with the average overlaid, replacing the discrete per-hop bands.

## Impact

- **Frontend only:** `assets/monitor.js` — change `_renderHopChart`'s min–max band series from one rectangle per hop into a single connected polygon (down the min edge, back up the max edge) so it forms a continuous filled ribbon; keep the average line/dots and tooltips; break the ribbon across non-responding hops.
- **No backend, API, data, or other UI change.**
- **Specs:** `web-ui`.
