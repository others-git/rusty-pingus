## MODIFIED Requirements

### Requirement: Theme set with functional guarantees
The system SHALL ship the cyberpunk theme as the default (applied when no theme is selected) plus several additional, deliberately whimsical themes. The system SHALL NOT offer a separate plain "Default" theme. Every theme SHALL remain functional: text legible against its background, monitor status colors still reading as up (positive), down (negative), and pending; interactive controls visibly interactive; and the page layout intact. Themes MAY otherwise differ freely in palette, typography, borders, backgrounds, and decoration.

#### Scenario: Cyberpunk is the default look
- **WHEN** no theme is selected
- **THEN** the app appears in the cyberpunk theme

#### Scenario: No plain "Default" theme listed
- **WHEN** the user opens the theme menu
- **THEN** no plain "Default"/unstyled theme is offered; the listed themes are cyberpunk plus the other whimsical themes

#### Scenario: Whimsical themes stay usable
- **WHEN** a non-default theme is active
- **THEN** text remains legible, up/down/pending statuses remain distinguishable with their expected meaning, controls remain clearly interactive, and the layout is not broken

#### Scenario: Multiple themes available
- **WHEN** the user opens the theme menu
- **THEN** the cyberpunk default plus two or more additional themes are listed

### Requirement: Theme persistence without flash
The selected theme SHALL persist across page navigations and reloads, and SHALL be applied before first paint so the page does not briefly flash a different theme. When no theme is stored — or a legacy "Default" value is stored — the default cyberpunk theme SHALL be applied before first paint.

#### Scenario: Choice persists across navigation and reload
- **WHEN** a user selects a theme, then navigates to another page or reloads
- **THEN** the previously selected theme is still applied

#### Scenario: No flash on load
- **WHEN** a page loads with the cyberpunk default or a saved theme
- **THEN** that theme is applied on first paint with no visible flash of a different theme

#### Scenario: Legacy "Default" resolves to cyberpunk
- **WHEN** a page loads and the saved theme is the removed "Default" value (or none)
- **THEN** the cyberpunk default is applied

## ADDED Requirements

### Requirement: Default theme has an animated, non-cursor-reactive background
The default (cyberpunk) experience SHALL render an animated background behind the page content and SHALL NOT include a cursor-follow ("mouse glow") spotlight effect. Content panels SHALL remain legible over the animated background. This requirement applies to the default theme; other themes MAY retain cursor-reactive effects.

#### Scenario: Background animates subtly
- **WHEN** the default theme is active
- **THEN** the background animates on its own behind the cards without obscuring text or requiring mouse input

#### Scenario: No cursor-follow glow on the default
- **WHEN** the user moves the mouse over the default theme
- **THEN** no spotlight/glow follows the cursor
