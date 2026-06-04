## 1. CDN & Base HTML Setup

- [x] 1.1 Update `assets/index.html` `<head>`: add Tailwind Play CDN script, Alpine.js CDN script (defer), Chart.js CDN script, Font Awesome CDN link, Inter font Google Fonts link
- [x] 1.2 Update `assets/monitor.html` `<head>` with the same CDN links
- [x] 1.3 Set `<html class="dark bg-slate-950 text-slate-100">` and `<body class="min-h-screen bg-slate-950 font-sans">` on both pages
- [x] 1.4 Trim `assets/style.css` to only custom overrides not expressible in Tailwind (e.g., Chart.js canvas sizing, SVG gauge keyframe if needed)

## 2. Dashboard Page — Structure & Layout

- [x] 2.1 Build sticky header bar in `index.html`: app name/logo, global summary stats ("X / Y Up", "Z Down") populated by Alpine.js reactive data, "Last updated: N seconds ago" counter
- [x] 2.2 Build the monitors grid container: `<div x-data="dashboard()" class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 p-6">` iterating over monitors with `x-for`
- [x] 2.3 Build monitor card markup: `slate-800` background, `slate-700` border, rounded-xl, shadow-lg; top row with protocol Font Awesome icon + monitor name; status badge row; endpoint URL; response time + 24h uptime row; last checked timestamp (relative)

## 3. Dashboard Page — Status Indicators & Badges

- [x] 3.1 Implement animated pulse indicator: outer `animate-ping` ring div + inner solid dot, both colored by status using Alpine `:class` binding (`emerald-400` / `red-400` / `amber-400`)
- [x] 3.2 Implement status badge component: pill-shaped span with uppercase text and background tint (`emerald-400/10 text-emerald-400` / `red-400/10 text-red-400` / `amber-400/10 text-amber-400`)
- [x] 3.3 Add Font Awesome protocol icons: `fa-globe` for HTTP, `fa-network-wired` for TCP, `fa-satellite-dish` for ICMP; apply `cyan-400` color

## 4. Dashboard Page — Alpine.js Data & Polling

- [x] 4.1 Write `dashboard()` Alpine component in `assets/app.js`: `monitors` array, `lastUpdated` timestamp, `loading` bool, `fetchMonitors()` async method calling `/api/monitors`
- [x] 4.2 Implement 30-second polling loop in `dashboard()` using `setInterval` started in Alpine's `init()` hook
- [x] 4.3 Implement relative time formatting helper (`formatRelative(isoString)`) returning "just now", "2 min ago", etc.
- [x] 4.4 Add `x-transition` fade for cards on initial load to prevent layout flash

## 5. Monitor Detail Page — Structure & Layout

- [x] 5.1 Build detail page header in `monitor.html`: back arrow link to `/`, monitor name (from URL), current status badge with pulse indicator, endpoint URL
- [x] 5.2 Build uptime gauges row: four `<div>` containers, one per window (1h, 24h, 7d, 30d), each holding an SVG ring + percentage label + window label
- [x] 5.3 Build response time chart container: `slate-900` card with Chart.js `<canvas>` element, title "Response Time (last 24h)"
- [x] 5.4 Build probe history table: `slate-800` card, table with columns Timestamp, Status (badge), Response Time, Failure Reason; alternating `slate-800`/`slate-900` row classes

## 6. Monitor Detail Page — SVG Gauge Rings

- [x] 6.1 Implement SVG gauge ring as an inline SVG template: circle with `stroke-dasharray` and `stroke-dashoffset` computed from uptime value; Alpine `:stroke-dashoffset` and `:stroke` bindings
- [x] 6.2 Implement color threshold logic for gauge stroke: `emerald-400` (≥99%), `amber-400` (90–98.9%), `red-400` (<90%)
- [x] 6.3 Center the percentage text label inside the SVG ring using `<text>` element with `dominant-baseline="middle"` and `text-anchor="middle"`

## 7. Monitor Detail Page — Chart.js Response Time Chart

- [x] 7.1 In `assets/monitor.js`, fetch `/api/monitors/:name/history` and extract `checked_at` and `response_time_ms` arrays for Chart.js
- [x] 7.2 Configure Chart.js: `type: 'line'`, dark background plugin (`slate-950`), cyan gradient fill using `ctx.createLinearGradient`, no gridlines (`grid: { display: false }`), cyan `#22d3ee` line color
- [x] 7.3 Configure Chart.js tooltip: dark background (`slate-800`), show exact timestamp and response time in ms
- [x] 7.4 Configure Chart.js x-axis: time scale with `moment` or built-in adapter, tick color `slate-400`, no border

## 8. Monitor Detail Page — Alpine.js Data

- [x] 8.1 Write `monitorDetail()` Alpine component in `assets/monitor.js`: extract monitor name from URL path, `history`, `uptime` (keyed by window), `currentStatus`, `loading` state
- [x] 8.2 Implement `fetchUptime()` calling `/api/monitors/:name/uptime` for 1h/24h/7d/30d windows in parallel (`Promise.all`)
- [x] 8.3 Implement `fetchHistory()` calling `/api/monitors/:name/history?limit=100` and populating history table rows
- [x] 8.4 Wire Alpine `init()` to call both fetch methods and then initialize the Chart.js chart with the fetched data

## 9. Polish & Verification

- [x] 9.1 Verify all pages render correctly in Chrome and Firefox with CDN resources loaded
- [x] 9.2 Verify dashboard auto-refreshes every 30 seconds and "Last updated" counter updates
- [x] 9.3 Verify pulse animation plays for up monitors and is absent for down/pending
- [x] 9.4 Verify gauge rings show correct arc length and color for sample uptime values (100%, 95%, 85%)
- [x] 9.5 Verify response time chart renders with dark theme, gradient fill, and correct data points
- [x] 9.6 Verify the dashboard grid is 3-column at 1280px+ and single-column below 640px
- [x] 9.7 Verify the `rust-embed` compilation still includes the updated asset files and the binary serves them correctly
