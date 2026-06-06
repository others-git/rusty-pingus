## 1. Switcher control + persistence

- [x] 1.1 Add `assets/theme.js` (vanilla, self-initializing): on load apply `localStorage.theme` to `document.documentElement.dataset.theme`; inject a `position: fixed` edge handle at the top-right; toggle a dropdown of themes on click; on select set `data-theme` + save to `localStorage` + close; close on Escape/outside-click
- [x] 1.2 In `assets/index.html` and `assets/monitor.html`: add a tiny inline `<head>` script that sets `data-theme` from `localStorage` before paint (no flash); link `/themes.css`; load `/theme.js`
- [x] 1.3 In `assets/style.css`, add theme-independent base styles for the edge handle + dropdown (subtle by default, prominent on hover/focus; sized to click deliberately, narrow enough to avoid accidents; menu styling)

## 2. Theme stylesheets

- [x] 2.1 Create `assets/themes.css` scoped by `html[data-theme="…"]`, overriding the app's bounded palette (bg-slate-950/900/800, text-slate-100…700, text/bg-cyan-*, border-slate-700, status emerald/red/amber-400, body font) with `!important`/high specificity; "Default"/none = no overrides
- [x] 2.2 **Cyberpunk "NEON CITY"** — near-black, neon magenta + electric cyan + acid-yellow accents, monospace, glowing shadows, 0-radius edges, faint scanline overlay; status as neon green/red/amber
- [x] 2.3 **Steampunk "BRASS & STEAM"** — parchment/sepia, brass/copper borders, serif type, warm shadows; patinated-green/oxidized-red status
- [x] 2.4 **Solarpunk "SUNFLOWER"** — cream + leaf-green + marigold, big rounded corners, soft organic shadows, friendly rounded sans
- [x] 2.5 **Vaporwave/GeoCities "A E S T H E T I C"** — hot-pink + teal on a gradient/tiled background, Comic Sans/cursive headings, chunky drop shadows, letter-spaced titles; readable but maximal
- [x] 2.6 Apply the functional guardrails to every theme: legible contrast, up/down/pending still meaningful, interactive elements obvious, layout intact

## 3. Verification

- [x] 3.1 `cargo build` (assets re-embed) and `node --check assets/theme.js` pass; served pages link `themes.css` + `theme.js` and include the no-flash inline snippet
- [ ] 3.2 (browser-only — needs your verification) Browser check on both pages: handle is subtle but clickable, menu opens/closes (click, Escape, outside-click), each theme applies and fully reskins the chrome, choice persists across navigation + reload with no flash, and every theme stays legible with up/down/pending still distinguishable
