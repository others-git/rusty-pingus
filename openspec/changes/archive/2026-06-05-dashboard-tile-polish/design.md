## Context

The dashboard card (`assets/index.html`, the `x-for="m in monitors"` template) is a `relative` `<div>` containing: two `absolute top-3 right-3 / right-9` hover buttons (pause, delete), then an inner `<a>` that wraps the header (with the status badge at the header's top-right), endpoint, optional detail, the response/uptime row, and the "Checked …" line. The grid uses `align-items: stretch`, so tiles in a row grow to the tallest tile's height — short tiles show empty space below their last line.

The two absolute buttons are pinned top-right, directly over the status badge, so on hover they cover "UP"/"PAUSED". Only the inner `<a>` is clickable, not the whole tile.

## Goals / Non-Goals

**Goals:**
- The whole tile is one click target to the monitor detail page.
- Pause/delete controls don't overlap the status badge; they live bottom-right.
- The lower dead space on stretched tiles reads as intentional.
- Pure markup/CSS; no JS or behavior change to toggling/deleting.

**Non-Goals:**
- Changing what the tile displays, the status semantics, or the paused styling.
- Reworking the grid or making tiles equal-height by other means.
- Touching the detail page.

## Decisions

### Decision: Stretched-link pattern for whole-tile clickability
Replace the content-wrapping `<a>` with a stretched-link overlay: keep the content as plain blocks, and add a single `<a class="absolute inset-0" :href=… aria-label=…>` that covers the tile. Interactive controls (the buttons) are layered above it with `relative z-10`, so they remain clickable while every other part of the tile navigates to the detail page.
- *Why:* This is the standard, accessible way to make a card a link without nesting interactive elements inside an anchor (buttons inside `<a>` is invalid). The buttons are siblings of the link, so a button click never triggers navigation — the existing `@click.stop` stays as defensive belt-and-suspenders.
- *Trade-off:* The overlay link sits above the text, so card text isn't selectable and the detail tile's `:title` hover tooltip won't fire. Acceptable — the detail is already rendered in full via `formatDetail`, and a clickable card is the priority. The `aria-label` keeps the link meaningful for screen readers.
- *Alternative:* Wrap the whole card in `<a>` and `preventDefault` on the buttons — rejected (invalid nesting, fragile).

### Decision: Bottom-right control group, hover-revealed
Put both buttons in one `absolute bottom-3 right-3 flex gap-1 z-10` container that is `opacity-0 group-hover:opacity-100` (unchanged reveal behavior). The status badge stays where it is at the header's top-right.
- *Why:* Frees the top-right for the status badge (the reported overlap) and uses the lower-right space that was dead on short, stretched tiles. Grouping the two buttons keeps spacing consistent and simplifies positioning.
- *Trade-off:* On a tile with a lot of `detail` text, the bottom-right controls overlay the bottom of the content on hover; the `z-10` + small icon footprint keeps this unobtrusive, and the controls only appear on hover.

## Risks / Trade-offs

- **Overlay link swallows the buttons' clicks** → Mitigation: buttons get `relative z-10` (above the `absolute inset-0` link); verify both pause and delete still work and that clicking elsewhere on the tile navigates.
- **Lost text selection / detail tooltip** → Accepted per the decision above; the detail content is still fully visible.
- **Controls overlapping detail text on hover** → Small, hover-only, high z-index; visually checked during verification.

## Migration Plan

Frontend-only; ships with the embedded assets (release rebuild to re-embed; debug serves from disk). No data/API/config change; trivially revertable.

## Open Questions

- None.
