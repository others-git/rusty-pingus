## Why

The dashboard monitor tiles have three rough edges: the small pause/delete buttons sit at the top-right where they overlap the status badge ("UP"/"PAUSED") on hover; only an inner region of the tile is a link, so clicking much of the card does nothing; and short tiles leave visible dead space beneath the "Checked …" line (the grid stretches rows to equal height). These are small but everyday-visible UI papercuts.

## What Changes

- **Move the pause/delete controls to the bottom-right.** Group the two hover buttons in the lower-right corner of the tile so they no longer render over the status badge. They stay hover-revealed.
- **Make the entire tile a clickable link** to the monitor detail page (stretched-link pattern), instead of only an inner block. The action buttons remain independently clickable (they sit above the link and don't trigger navigation).
- **Reclaim the dead space** beneath "Checked …": with the controls anchored bottom-right, the previously empty lower area of stretched tiles is used, and the tile layout reads as intentional rather than padded.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- **web-ui** — the dashboard monitor tile becomes a single clickable link to its detail page, and its per-tile controls (pause/delete) move to the bottom-right so they no longer overlap the status badge.

## Impact

- **Frontend:** `assets/index.html` only — restructure the monitor card: unwrap the inner `<a>` into a stretched-link overlay covering the whole tile, and move the pause + delete buttons into a bottom-right hover group layered above the link.
- **No JS change** (`toggleMonitor`/`pendingDelete` and their `@click.stop` handlers are unchanged; the buttons are siblings of the link, not nested in it).
- **No backend, API, or data change.**
- **Specs:** `web-ui`.
