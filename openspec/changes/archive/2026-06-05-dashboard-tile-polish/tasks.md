## 1. Restructure the monitor tile (assets/index.html)

- [x] 1.1 Unwrap the inner content `<a>`: keep the header/endpoint/detail/metrics/"Checked …" blocks as plain elements, and add a stretched-link overlay `<a class="absolute inset-0 z-0 rounded-xl" :href="'/monitors/' + encodeURIComponent(m.name)" :aria-label="'View ' + m.name">` covering the whole tile.
- [x] 1.2 Move the pause and delete buttons into one bottom-right hover group: `<div class="absolute bottom-3 right-3 flex items-center gap-1 z-10 opacity-0 group-hover:opacity-100 transition-opacity">` containing both buttons (keep their `@click.stop`, titles, and icons). Remove the old `top-3 right-3/right-9` positions.
- [x] 1.2a Ensure the buttons sit above the overlay link (`z-10` on the group) so they stay clickable while the rest of the tile navigates.
- [x] 1.3 Confirm the status badge remains at the header's top-right and is no longer overlapped; tighten any now-unneeded bottom spacing so the tile reads as intentional.

## 2. Verification

- [~] 2.1 Clicking anywhere on a tile (not a button) navigates to the monitor detail page; the cursor/affordance covers the whole tile.
- [~] 2.2 Pause/resume and delete still work and do NOT navigate; the controls appear bottom-right on hover and never cover the status badge.
- [~] 2.3 Paused and down tiles still render their distinct states correctly; short tiles no longer show obvious dead space below "Checked …".
- [x] 2.4 Sync the `web-ui` main spec via this change's delta at archive time.
