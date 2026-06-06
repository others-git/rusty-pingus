# Theme Switcher Spec

## Purpose

Defines a persistent, low-emphasis theme control present on every page that lets users switch between the default appearance and several deliberately whimsical themes, with the choice persisting across navigation and reloads without a flash of the wrong theme.

## Requirements

### Requirement: Persistent theme control
The system SHALL provide a theme control present on every page, rendered as a discreet handle pinned to the right edge of the header bar and extending inward. The handle SHALL be unobtrusive (low-emphasis until hovered or focused) yet a large enough target to click deliberately, and small enough to avoid accidental activation. Activating it SHALL open a dropdown menu listing the available themes with the active theme indicated; selecting a theme SHALL apply it immediately.

#### Scenario: Control present on every page
- **WHEN** a user is on the dashboard or a monitor detail page
- **THEN** the theme handle is present at the right edge of the header

#### Scenario: Handle is discreet but clickable
- **WHEN** the handle is not hovered or focused
- **THEN** it is visually subtle; on hover/focus it becomes prominent and is an easy click target

#### Scenario: Open the theme menu
- **WHEN** the user activates the handle
- **THEN** a dropdown lists the available themes with the current theme indicated

#### Scenario: Select a theme
- **WHEN** the user picks a theme from the menu
- **THEN** the app's appearance changes to that theme immediately and the menu closes

#### Scenario: Dismiss the menu
- **WHEN** the user clicks outside the menu or presses Escape
- **THEN** the menu closes without changing the theme

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

### Requirement: Default theme has an animated, non-cursor-reactive background
The default (cyberpunk) experience SHALL render an animated background behind the page content and SHALL NOT include a cursor-follow ("mouse glow") spotlight effect. Content panels SHALL remain legible over the animated background. This requirement applies to the default theme; other themes MAY retain cursor-reactive effects.

#### Scenario: Background animates subtly
- **WHEN** the default theme is active
- **THEN** the background animates on its own behind the cards without obscuring text or requiring mouse input

#### Scenario: No cursor-follow glow on the default
- **WHEN** the user moves the mouse over the default theme
- **THEN** no spotlight/glow follows the cursor
