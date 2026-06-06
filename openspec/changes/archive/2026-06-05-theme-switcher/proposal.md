## Why

The dashboard looks sharp but it's all one (very tasteful) dark theme. Time for fun: let people reskin the whole app from a discreet control that's always within reach, with a set of deliberately over-the-top *-punk themes. Pure delight — no functional requirement, just personality (kept usable).

## What Changes

- **A hidden theme control on every page.** A slim, low-key handle pinned to the right edge of the header bar, extending inward — large enough to hit on purpose but unobtrusive enough not to trigger by accident (subtle until hover/focus). It's the same control on the dashboard and the monitor detail page.
- **Click → dropdown menu of themes.** The handle expands into a small menu listing the available themes, current one indicated.
- **A set of stylesheets.** "Default" is the current look (no overrides). Plus several wacky, unapologetically unprofessional *-punk themes that completely reskin the chrome (colors, fonts, borders, decorative flourishes) while staying **functional**: legible text, status colors that still read up/down, clickable things still obviously clickable, layout intact.
- **The choice persists** across pages and reloads (localStorage), applied early so there's no flash of the wrong theme.

## Capabilities

### New Capabilities

- `theme-switcher`: A persistent, page-agnostic control to switch the app's visual theme, plus the bundled theme stylesheets.

## Impact

- New `assets/themes.css` — `[data-theme="…"]` blocks overriding the app's (bounded) Tailwind palette per theme, plus decorative touches.
- New `assets/theme.js` — self-initializing: applies the saved theme, injects the edge handle + dropdown, persists selection. Loaded on both pages.
- `assets/index.html`, `assets/monitor.html` — link `themes.css` + `theme.js`; a tiny inline `<head>` snippet sets `data-theme` from localStorage before paint (no flash).
- `assets/style.css` — styles for the edge handle/menu (theme-independent base).
- No backend changes.
- Out of scope (v1): recoloring the ECharts canvas (its colors are set in JS); themes restyle the page chrome, the chart keeps its palette for now.
