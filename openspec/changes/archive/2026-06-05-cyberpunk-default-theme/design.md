## Context

Theming is driven by `assets/themes.css` keyed on `<html data-theme="…">`, with `assets/theme.js` providing the switcher, persistence (localStorage `theme`), and a pointer tracker that exposes `--mx/--my/--mxpx/--mypx` for cursor-reactive backgrounds. An inline `<head>` snippet in each HTML page applies the saved theme before first paint to avoid a flash.

Today the "Default" appearance is the no-attribute baseline (plain Tailwind slate); `cyber` is one selectable theme. The cyberpunk background is `body::before` (a scrolling neon grid + scanlines) and `body::after` (a cursor-follow neon spotlight using `--mxpx/--mypx`).

## Goals / Non-Goals

**Goals:**
- Cyberpunk is what users see by default, with no flash of another theme on load.
- The plain "Default" theme is gone from the switcher and as a baseline.
- The default/cyberpunk view has no cursor-follow glow.
- A richer, animated aurora background for the default/cyberpunk view.

**Non-Goals:**
- Changing the other themes (steam/solar/vapor/chaos) or their cursor effects.
- Restructuring the theming mechanism (still `data-theme`-keyed CSS).
- Any backend/API/data change.

## Decisions

### Decision: Keep `data-theme="cyber"` as the default selection (don't re-root to `:root`)
The default is implemented by *selecting* `cyber` when nothing is stored, not by moving cyber's rules onto the no-attribute baseline. `theme.js`'s default becomes `cyber`; the inline `<head>` snippet sets `data-theme="cyber"` when no theme (or a legacy `default`) is stored.
- *Why:* Minimal, low-risk — the entire existing `[data-theme="cyber"]` ruleset is reused as-is; no large CSS move. The plain-slate baseline simply stops being reachable.
- *Trade-off:* The no-attribute look still technically exists in Tailwind classes but is never shown. Acceptable and reversible.
- *Alternative:* Re-root cyber styles to `:root` so "no theme" *is* cyber — more invasive, larger diff, no user-visible benefit.

### Decision: Migrate saved `default` → `cyber`
Both `theme.js` and the inline snippet treat a stored value of `default` (or missing) as `cyber`, so existing users who never picked a theme — or explicitly saved "Default" — get the new baseline rather than a now-unlisted plain look.
- *Why:* "Default" is removed; a stale saved value must resolve to the new default.

### Decision: Remove only the cyberpunk cursor spotlight; keep global pointer tracking
Delete the `html[data-theme="cyber"] body::after` cursor-spotlight layer. Leave `theme.js`'s pointer tracker in place because steam/solar/vapor still use `--mxpx/--mypx` spotlights.
- *Why:* Matches the requested scope (default/cyberpunk only) without regressing other themes.

### Decision: Aurora background via animated gradients, no JS
Replace the cyberpunk background layers with a CSS-only animated aurora: a slow-shifting cyan→magenta→violet gradient wash (animated `background-position`/hue over tens of seconds) as the base, a faint **static** grid for structure, and subtle scanlines, all on fixed full-viewport pseudo-elements behind content (`z-index:-1`, `pointer-events:none`).
- *Why:* CSS-only keeps it cheap and matches the existing pattern (`body::before/::after`). No cursor input, so it reads calm rather than gimmicky. Cards keep their solid translucent backgrounds, so text stays legible over the moving backdrop.
- *Trade-off:* A slow gradient animation is GPU-light; the grid stays static to avoid visual noise competing with the aurora.

## Risks / Trade-offs

- **Legibility over a moving background** → Cards already use translucent solid panels (`rgba(12,10,28,.85)` etc.); the aurora sits behind them at low opacity, so body text never sits directly on the animated wash. Mitigation: keep card opacity, keep aurora subtle.
- **FOUC if the inline snippet and JS disagree on the default** → Both must default to `cyber` and both must map `default→cyber`. Mitigation: change them together; verify a fresh load (empty localStorage) shows cyberpunk immediately.
- **Reduced-motion users** → The aurora animates continuously. Mitigation: gate the animation behind `@media (prefers-reduced-motion: reduce)` to a static gradient.

## Migration Plan

Frontend-only; ships with the embedded assets. A release build must be rebuilt to re-embed; debug serves from disk. No data migration. Rollback is reverting the asset changes. Existing users' saved `default` resolves to `cyber` automatically.

## Open Questions

- Should "Default"/plain-slate remain reachable as a hidden fallback (e.g. for screenshots), or be fully removed? Proposed: fully removed from the menu; the underlying classes remain but unused.
