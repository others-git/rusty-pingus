## Why

The initial `web-ui` spec called for a "minimal" frontend with vanilla JS and hand-rolled CSS — functional but plain. Since the binary embeds assets at compile time anyway, using CDN-hosted, well-known libraries adds zero runtime overhead to the server while dramatically improving the visual quality and developer ergonomics of the frontend.

## What Changes

- Replace hand-rolled CSS with **Tailwind CSS** (CDN) for utility-first styling
- Replace vanilla JS DOM manipulation with **Alpine.js** (CDN) for lightweight reactive UI without a build step
- Replace Chart.js with **Chart.js** (CDN, already planned) — kept but properly themed with a dark/modern palette
- Dashboard redesign: dark-mode by default, card-based layout, status badges, animated pulse indicators for live monitoring feel
- Monitor detail redesign: prominent uptime gauges, gradient response-time charts, tabbed history view
- Add **Font Awesome** (CDN) for iconography (protocol icons, status icons)
- Add **Inter** font (Google Fonts CDN) for a clean, modern typographic base

## Capabilities

### New Capabilities

- `ui-design-system`: Defines the visual design language — color palette, typography, spacing, component styles — applied consistently across the dashboard and detail pages

### Modified Capabilities

- `web-ui`: Requirements for what the UI displays are unchanged, but HOW it looks and which libraries it uses changes significantly. Delta spec clarifies CDN library usage, dark theme, and enhanced visual components (animated indicators, gradient charts, uptime gauge rings).

## Impact

- `assets/index.html`, `assets/monitor.html` — fully rewritten with Tailwind classes and Alpine.js directives
- `assets/app.js`, `assets/monitor.js` — simplified significantly; Alpine.js handles reactivity, Chart.js config updated for dark theme
- `assets/style.css` — reduced to minimal overrides; Tailwind handles 95% of styling
- No changes to Rust backend, API, or database
- CDN links added to HTML `<head>`; no new Rust dependencies
- Binary size of embedded assets unchanged or smaller (less hand-written CSS)
