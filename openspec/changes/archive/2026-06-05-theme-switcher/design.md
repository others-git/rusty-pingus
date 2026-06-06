## Context

Two pages (`index.html`, `monitor.html`) share a sticky header and load Tailwind (CDN) + `/style.css`. Styling is Tailwind utility classes baked into the HTML, over a bounded palette: backgrounds `bg-slate-950/900/800`, text `text-slate-100…700`, accents `text-cyan-300/400/500` + `bg-cyan-500/600`, status `text-emerald/red/amber-400`, borders `border-slate-700`, `font-sans`. Both run Alpine. There's no build step — assets are embedded and served as-is.

## Goals / Non-Goals

**Goals:** a discreet always-present control to pick a theme; several maximalist *-punk themes that fully reskin the chrome but stay usable; selection persists across pages/reloads with no flash.
**Non-Goals:** recoloring the ECharts canvas (JS-set colors) in v1; a server-side preference; per-monitor theming; a Tailwind rebuild.

## Decisions

### 1. Theme = `data-theme` on `<html>` + scoped overrides
Themes are applied by setting `document.documentElement.dataset.theme`. A single `themes.css` contains `html[data-theme="cyber"] .bg-slate-900 { … !important }` style blocks. Because the palette is a small, known set of utility classes, each theme overrides that finite set (backgrounds, text tiers, accents, borders, `body` font) plus a few decorative extras. `html[data-theme=…]` selectors out-specify bare utility classes; `!important` covers the rest. "Default" sets no `data-theme` (or `data-theme="default"`) and has zero overrides → current look untouched. Alternative (CSS variables) rejected: would require rewriting all the inline utilities.

### 2. No-flash application
A tiny inline script in each page `<head>` (before stylesheets render content) reads `localStorage.theme` and sets `data-theme` immediately, so the themed look is present on first paint. `theme.js` (deferred) then builds the control and handles changes.

### 3. The edge handle → dropdown (vanilla, page-agnostic)
`theme.js` self-initializes (no coupling to the page's Alpine component) and injects a `position: fixed` element at the top-right corner overlapping the header's right edge:
- A slim vertical handle (e.g. ~10–14px wide, ~40–52px tall) flush to the right edge, **extending inward**, low-contrast/translucent so it's "hidden"; on hover/focus it brightens and shows a 🎨 hint. Big enough to click deliberately, narrow enough to avoid accidental hits.
- Click toggles a small dropdown listing themes; the active one is checked. Selecting sets `data-theme`, saves to `localStorage`, closes the menu. Escape / outside-click closes it.
Fixed positioning guarantees it appears identically on every page without editing each header's layout. Base styles for the handle/menu live in `style.css` and are theme-independent (so they work even before a theme loads); themes may restyle them too.

### 4. Functional guardrails for every theme
Each theme MUST keep: text/background contrast legible; up = greenish/positive, down = reddish/negative, pending = amber-ish (status semantics preserved even if hues shift); buttons/links visibly interactive; layout/spacing intact (themes change color/font/border/shadow/decoration, not structural layout). Whimsy lives in palette, typography, borders, background patterns, and small flourishes.

### 5. The theme roster (creative latitude)
- **Default** — the current cyan-on-slate dark UI. No overrides.
- **Cyberpunk — "NEON CITY"** — near-black, neon magenta + electric cyan + acid-yellow accents; monospace; glowing text/box shadows; hard 0-radius edges; faint scanline overlay; status as neon green/red.
- **Steampunk — "BRASS & STEAM"** — parchment/sepia, brass & copper borders (double/riveted feel), serif type, warm shadows; status as patinated green / oxidized red.
- **Solarpunk — "SUNFLOWER"** — cream + leaf-green + marigold, big rounded corners, soft organic shadows, friendly rounded sans, optimistic and bright; status greens/ambers fit naturally.
- **Vaporwave/GeoCities — "A E S T H E T I C"** — the gleefully unprofessional one: hot-pink + teal on a gradient/tiled background, Comic Sans / cursive headings, chunky drop shadows, letter-spaced titles, maybe a subtle star/grid backdrop. Still readable, just… a lot.

Themes are easy to add/remove (one CSS block + one menu entry).

## Risks / Trade-offs

- **`!important` override soup** — restyling utility classes needs `!important`/high specificity; kept tractable by the small palette. If a stray element isn't covered, it just keeps default colors (harmless).
- **Chart stays default-colored** — the ECharts canvas won't match a theme in v1; acceptable, and a clean follow-up (pass theme tokens into the chart option) if wanted.
- **Comic Sans availability** — the joke theme leans on web-safe/system fonts (Comic Sans MS, cursive fallback); no font CDN dependency required.
- **Contrast slips** — the maximalist themes risk legibility; the functional guardrail (and a quick check per theme) keeps them usable.
- **Browser-only verification** — the look/interaction can't be tested headlessly; verify in a browser per theme on both pages.

## Open Questions

- Exact handle dimensions/affordance (icon vs. bare strip) — tune visually during implementation.
- Whether to also theme the ECharts canvas later (out of scope now).
