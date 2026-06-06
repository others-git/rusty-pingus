## Why

The flat slate "Default" theme is the least interesting look in the app, while the cyberpunk theme is the intended signature aesthetic. The app should lead with that personality instead of a neutral default. Two parts of the cyberpunk look also undercut it: the cursor-follow neon spotlight is a distracting gimmick, and the plain scrolling grid background is bland.

## What Changes

- **Cyberpunk becomes the default/baseline theme** — applied when no theme is stored, and before first paint so there is no flash of another theme.
- **Remove the "Default" (plain slate) theme** — it is dropped from the theme switcher and is no longer the no-selection baseline. Users who had "Default" saved are treated as cyberpunk.
- **Remove the cursor-follow "mouse glow"** from the default/cyberpunk experience. (Other themes keep their cursor-reactive effects; the pointer tracking stays for them.)
- **Replace the cyberpunk background** with a more interesting animated aurora: a slow cyan→magenta→violet gradient wash behind a faint static grid and scanlines, instead of the current flat scrolling grid.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- **theme-switcher** — the default/baseline theme is now cyberpunk rather than the plain slate "Default" (which is removed); the default experience has no cursor-follow glow and uses an animated aurora background. The remaining whimsical themes are unchanged.

## Impact

- **Frontend:**
  - `assets/theme.js` — default theme becomes `cyber`; remove the `Default` entry from the theme list; map any saved `default` to `cyber`; keep pointer tracking but only for the other themes that use it.
  - `assets/themes.css` — remove the cyberpunk cursor-spotlight layer (`body::after`); replace the background with the animated aurora + faint grid + scanlines.
  - `assets/index.html`, `assets/monitor.html` — the inline `<head>` theme snippet defaults to `cyber` (no flash) and treats a saved `default` as `cyber`.
- **Backend / API / data:** none.
- **Specs:** `theme-switcher`.
