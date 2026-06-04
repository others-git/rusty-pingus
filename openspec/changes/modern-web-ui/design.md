## Context

The `uptime-monitor` change established a web UI spec with "minimal" vanilla HTML/CSS/JS. This change upgrades the frontend visual quality using CDN-hosted libraries — no build pipeline, no npm, no bundler. Assets are still compiled into the Rust binary via `rust-embed`. The Rust backend and API are untouched.

Target aesthetic: dark-mode ops dashboard. Think Vercel status page meets Grafana. Clean, data-dense, confident.

## Goals / Non-Goals

**Goals:**
- Dark-mode-first design with a refined color palette
- Tailwind CSS (CDN) for consistent, maintainable utility-first styling
- Alpine.js (CDN) for reactive UI without a build step or virtual DOM overhead
- Chart.js (CDN) with a dark-themed, gradient configuration for response time charts
- Font Awesome (CDN) for crisp protocol and status icons
- Inter font (Google Fonts CDN) for modern typography
- Animated pulse ring on live "up" status indicators
- Uptime percentage displayed as circular gauge rings on the detail page
- Responsive layout: works on desktop and tablet

**Non-Goals:**
- Any server-side rendering changes
- Authentication or access control
- WebSocket/SSE (real-time push) — polling remains
- Mobile-first breakpoints (desktop-first is fine for an ops tool)
- npm, webpack, Vite, or any build step for frontend assets
- Supporting Internet Explorer or legacy browsers

## Decisions

### 1. Tailwind CSS via CDN (Play CDN)
Tailwind's Play CDN (`https://cdn.tailwindcss.com`) generates styles on-the-fly in the browser from utility classes in the HTML. No build step required. The trade-off is a slightly larger initial JS payload (~100KB) and no purging — acceptable since the HTML is small. Alternative: hand-rolled CSS — rejected because it's slow to iterate and hard to keep consistent.

### 2. Alpine.js for reactivity
Alpine.js (CDN, ~15KB gzipped) provides `x-data`, `x-bind`, `x-for`, `x-show`, `x-transition` and reactive state — enough to drive the entire UI without React/Vue overhead or a build step. The polling loop, status updates, and chart rendering all live in small `x-data` objects. Alternative: vanilla JS — works but becomes messy fast with dynamic DOM updates; Vue/React — overkill and require a build step.

### 3. Chart.js for response time charts
Already planned in the `uptime-monitor` design. We configure it with a dark background (`#0f172a`), a cyan/teal gradient fill under the line, and hidden gridlines for a clean look. Tooltip shows exact timestamp and response time. Alternative: D3.js — too complex for a simple time series; ApexCharts — heavier, less customizable.

### 4. Color palette: Slate dark + Cyan accent
- Background: `slate-950` (`#020617`) / `slate-900` (`#0f172a`)
- Card surface: `slate-800` (`#1e293b`)
- Border: `slate-700` (`#334155`)
- Up/green: `emerald-400` (`#34d399`)
- Down/red: `red-400` (`#f87171`)
- Pending: `amber-400` (`#fbbf24`)
- Accent/links: `cyan-400` (`#22d3ee`)
- Text primary: `slate-100`, secondary: `slate-400`

### 5. Uptime gauge: SVG stroke-dasharray ring
A simple SVG circle with `stroke-dasharray` driven by the uptime percentage gives a clean circular gauge without a chart library dependency. The ring color transitions from red → amber → emerald based on the uptime value.

### 6. CDN library versions (pinned)
- Tailwind CSS Play CDN: `3.x` (latest 3)
- Alpine.js: `3.x` (latest 3)
- Chart.js: `4.x` (latest 4)
- Font Awesome Free: `6.x` (latest 6)
- Inter font: via Google Fonts

All pinned to major version to avoid breaking changes while receiving patches.

## Risks / Trade-offs

- **CDN availability** → If CDN is unreachable, the UI degrades (no styles/interactivity). Mitigation: these are industry-standard CDNs (jsDelivr, cdnjs, Google) with 99.9%+ uptime; document offline fallback as a future enhancement (self-host assets).
- **Tailwind Play CDN in production** → Tailwind recommends the Play CDN for development/prototyping, not production (due to lack of purging). Mitigation: for a self-hosted ops tool with tiny HTML files, the unpurged size is acceptable (~30KB of applied styles); purging can be added later with a simple build step.
- **Alpine.js complexity ceiling** → For very complex interactions, Alpine becomes unwieldy. Mitigation: the UI scope is bounded (two pages, polling loop, one chart); Alpine is appropriate at this scale.

## Open Questions

- Should the dashboard show a global summary banner (e.g., "X of Y monitors up") at the top? → Yes, add it as a sticky header stat.
- Should the response time chart on the detail page allow zooming/panning? → No for v1; Chart.js zoom plugin can be added later.
