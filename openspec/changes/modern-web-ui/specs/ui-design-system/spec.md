## ADDED Requirements

### Requirement: CDN library inclusion
The system SHALL load Tailwind CSS Play CDN, Alpine.js, Chart.js, Font Awesome, and Inter font from well-known public CDNs in the HTML `<head>` of every page, pinned to a specific major version.

#### Scenario: Libraries load on page open
- **WHEN** a user opens the dashboard or detail page in a browser with internet access
- **THEN** all CDN resources load successfully and the page renders with correct styles and interactivity

#### Scenario: Graceful degradation without CDN
- **WHEN** CDN resources fail to load (network error)
- **THEN** the page content (monitor names, statuses, timestamps) remains readable as plain HTML, even if unstyled and non-interactive

### Requirement: Dark-mode color palette
The system SHALL use a dark-mode-first color palette consistently across all pages: `slate-950`/`slate-900` backgrounds, `slate-800` card surfaces, `slate-700` borders, `emerald-400` for "up" status, `red-400` for "down" status, `amber-400` for "pending", `cyan-400` for accents and links, and `slate-100`/`slate-400` for primary/secondary text.

#### Scenario: Up status is green
- **WHEN** a monitor has status `up`
- **THEN** its status indicator, badge, and any associated text use the emerald-400 color

#### Scenario: Down status is red
- **WHEN** a monitor has status `down`
- **THEN** its status indicator, badge, and any associated text use the red-400 color

#### Scenario: Pending status is amber
- **WHEN** a monitor has not yet run a probe
- **THEN** its status indicator and badge use the amber-400 color

### Requirement: Typography
The system SHALL use the Inter typeface (loaded from Google Fonts) as the base font family across all pages, with Tailwind's default type scale for sizing.

#### Scenario: Inter font applied globally
- **WHEN** any page is rendered
- **THEN** all text elements use the Inter font family if the Google Fonts CDN resource loaded successfully

### Requirement: Animated live status indicator
The system SHALL display an animated pulsing ring around the status dot for monitors with `up` status, conveying a live/active monitoring feel.

#### Scenario: Up monitor shows pulse animation
- **WHEN** a monitor card displays `up` status
- **THEN** a CSS pulse ring animation plays continuously around the status dot using Tailwind's `animate-ping` utility

#### Scenario: Non-up monitors have no animation
- **WHEN** a monitor is `down` or `pending`
- **THEN** no pulse animation is shown — the status dot is static

### Requirement: Uptime gauge ring
The system SHALL display each uptime percentage value as an SVG circular gauge ring on the monitor detail page, where the filled arc length represents the uptime fraction. The ring color SHALL use the status palette (emerald above 99%, amber 90–99%, red below 90%).

#### Scenario: 100% uptime shows full green ring
- **WHEN** a monitor has 100% uptime over the queried window
- **THEN** the SVG ring is fully filled in emerald-400

#### Scenario: 95% uptime shows partial amber ring
- **WHEN** a monitor has 95% uptime
- **THEN** the SVG ring shows 95% filled arc in amber-400

#### Scenario: Below 90% uptime shows red ring
- **WHEN** a monitor has less than 90% uptime
- **THEN** the SVG ring arc is red-400

### Requirement: Responsive layout
The system SHALL use a responsive grid layout on the dashboard that adapts from a single column on narrow viewports to multi-column on wider viewports, using Tailwind responsive prefixes (`sm:`, `md:`, `lg:`).

#### Scenario: Wide viewport shows multi-column grid
- **WHEN** the dashboard is viewed at 1280px or wider
- **THEN** monitor cards are displayed in a 3-column grid

#### Scenario: Narrow viewport shows single column
- **WHEN** the dashboard is viewed at less than 640px wide
- **THEN** monitor cards stack in a single column
