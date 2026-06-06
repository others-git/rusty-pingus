## 1. Make cyberpunk the default (theme.js)

- [x] 1.1 Remove the `{ id: 'default', name: 'Default' }` entry from the `THEMES` list so it no longer appears in the switcher.
- [x] 1.2 Change the default theme from `default` to `cyber`: `current()` returns `cyber` when nothing is stored, and treat a stored legacy value of `default` as `cyber`. (Added `DEFAULT_THEME` + `normalize()`.)
- [x] 1.3 Update `apply()` so the default applies `data-theme="cyber"` (rather than removing the attribute); ensure the safety-net `apply(current())` shows cyberpunk on load.

## 2. Default-theme no-flash snippet (HTML)

- [x] 2.1 In `assets/index.html` and `assets/monitor.html`, update the inline `<head>` theme snippet to default to `cyber` and map a saved `default` → `cyber`, so a fresh load paints cyberpunk with no flash.

## 3. Remove the mouse glow + new background (themes.css)

- [x] 3.1 Remove the cyberpunk cursor-spotlight layer (`html[data-theme="cyber"] body::after` using `--mxpx/--mypx`). Leave the global pointer tracking in `theme.js` for the other themes that use it. (Repurposed `body::after` as the static grid/scanline/vignette layer; aurora moved to `body::before`.)
- [x] 3.2 Replace the cyberpunk background (`body::before`) with an animated aurora: a slow cyan→magenta→violet gradient wash as the base, a faint static grid for structure, and subtle scanlines, on fixed full-viewport pseudo-elements behind content.
- [x] 3.3 Gate the aurora animation behind `@media (prefers-reduced-motion: reduce)` to a static gradient.

## 4. Verification

- [~] 4.1 With empty localStorage, load the dashboard and a detail page: cyberpunk shows immediately with no flash of the plain slate look. (Verified the served `<head>` snippet sets `data-theme="cyber"` synchronously before paint; visual no-flash not screenshot-confirmed — no browser here.)
- [x] 4.2 Confirm the theme menu no longer lists "Default"; cyberpunk + the other themes are present and selectable. (`THEMES` no longer contains a Default entry; cyber + steam/solar/vapor/chaos remain.)
- [x] 4.3 Confirm no glow follows the cursor on the default theme, and that another theme (e.g. steam) still shows its cursor effect. (Cyber `body::after` cursor spotlight removed — 0 refs in served CSS; 5 `--mxpx` spotlight refs remain for other themes; pointer tracking left intact.)
- [~] 4.4 Confirm the aurora background animates, text stays legible over it, and reduced-motion users get a static background. (Aurora keyframe + `prefers-reduced-motion` fallback present in served CSS; cards keep translucent panels. Visual/animation not screenshot-confirmed — no browser here.)
- [x] 4.5 Confirm a previously saved `default` value now renders as cyberpunk. (`normalize()` in theme.js and the inline snippet both map `default`/empty → `cyber`.)
- [x] 4.6 Sync the `theme-switcher` main spec via this change's delta at archive time.
