# Frontend — Living Spec

> [DRAFT] Surface-first draft from existing code — every requirement is observed from the code surface unless tagged otherwise. Review before trusting.

## Purpose

Presents the site list as a glanceable table that stays current on its own and lets the user add, edit, and remove entries. It exists so the answer to "is this up?" is readable in a second without interaction — which means the display has to keep changing under the user's hands without disrupting what they are doing.

## Requirements

### The display stays current without asking the backend for it

The UI SHALL hold its own view of the sites and their latest results, updating it from pushed status events rather than by polling. The age of the last result SHALL advance on a local timer, so elapsed time stays visibly accurate without any additional backend traffic.

#### Scenario: a check completes in the background
- **WHEN** a status event arrives for a listed site
- **THEN** that row's state and reason update in place with no user action

#### Scenario: no new result arrives for a while
- **WHEN** time passes without a new check completing
- **THEN** the displayed age of the last result keeps counting up
- **AND** no request is made to the backend to achieve that

### Repainting must not disturb what the user is doing

Because the table repaints continuously, rows SHALL be reconciled against their existing elements — matched by site, updated in place, and left alone when nothing changed — rather than rebuilt. Element identity MUST survive a repaint: a native tooltip only appears under sustained hover on the same element, and keyboard focus on a row control is lost the moment its element is replaced. Row position MUST likewise be adjusted only when it is genuinely wrong.

#### Scenario: a status changes while the user hovers it
- **WHEN** a repaint occurs while the pointer rests on a row's status indicator
- **THEN** the tooltip is not interrupted

#### Scenario: a repaint occurs while a row control has focus
- **WHEN** the user has tabbed to a row's action button
- **THEN** the repaint leaves that focus intact

#### Scenario: a site is added or removed
- **WHEN** the site list changes
- **THEN** only the affected rows are created or removed
- **AND** untouched rows keep their existing elements

### A site with no result yet reads as its own state

A site that has not yet completed a check this session SHALL be presented as distinct from both up and down, and its last-checked time SHALL show as unknown rather than as a misleading value. This state belongs to the UI alone — the backend never reports it.

#### Scenario: the app has just launched
- **WHEN** saved sites are listed before any check completes
- **THEN** each reads as awaiting a result, with no last-checked time

#### Scenario: the first result lands
- **WHEN** a site's first check completes
- **THEN** it leaves the awaiting state and shows its result and age

### A failure explains itself on demand, and stops explaining when it is fixed

The reason a site is down SHALL be reachable without adding a column or crowding the row — surfaced on the status indicator itself. When there is no longer anything to explain, the explanation MUST be removed rather than left showing a stale reason.

#### Scenario: a site goes down
- **WHEN** a status arrives carrying a failure reason
- **THEN** the reason is available by hovering the row's status indicator

#### Scenario: the site recovers
- **WHEN** a later status carries no reason
- **THEN** the previous explanation is removed entirely

### One form serves both adding and editing

Adding and editing SHALL use a single form that switches modes, so there is one place to type and one set of rules. Entering edit mode SHALL pre-fill the site's current values and offer a way out; completing or cancelling SHALL return the form to its add state.

#### Scenario: the user edits an existing site
- **WHEN** edit is chosen for a row
- **THEN** the form fills with that site's values and offers to cancel

#### Scenario: the edit is saved
- **WHEN** the save succeeds
- **THEN** the form returns to its add state, ready for the next entry

#### Scenario: the site being edited is deleted
- **WHEN** the row currently loaded in the form is removed
- **THEN** the form returns to its add state rather than holding a site that no longer exists

### A rejected change is explained without discarding what was typed

When the backend rejects a change, the message SHALL be shown to the user near the form and the entered values SHALL be preserved so the entry can be corrected rather than retyped. A rejection MUST never fail silently.

#### Scenario: the backend rejects a URL
- **WHEN** a save is refused
- **THEN** the reason is shown with the form
- **AND** what the user typed is still there to correct

#### Scenario: a delete fails
- **WHEN** removing a site is refused
- **THEN** the reason is shown and the row remains listed

### Non-fatal backend problems are visible without blocking use

Warnings that do not stop the app — a site list that could not be read, a save that did not reach disk — SHALL be surfaced in a persistent notice rather than a modal, both when reported at startup and when they occur later. The app stays usable throughout.

#### Scenario: the saved list could not be read at startup
- **WHEN** the app starts and reports a load warning
- **THEN** the notice is displayed and the rest of the UI works normally

#### Scenario: a save fails during use
- **WHEN** a write failure is reported while the app is running
- **THEN** the notice appears without interrupting the current interaction

### The login-item control reflects the system, not the click

The launch-at-login control SHALL display the state the operating system actually reports, both when first shown and after every change. If a change is refused the control MUST correct itself rather than showing a setting that is not in effect, and if the current setting cannot be read at all the control MUST be disabled rather than displayed as a guess.

#### Scenario: the setting is toggled successfully
- **WHEN** the user changes the control
- **THEN** it settles on the state the system reports afterwards

#### Scenario: the system refuses the change
- **WHEN** the change fails
- **THEN** the control reverts and the reason is surfaced

#### Scenario: the setting cannot be read
- **WHEN** reading the current setting fails when the UI mounts
- **THEN** the control is disabled and the reason is surfaced

### All backend access goes through one typed boundary

Every call into the backend and every subscription to its events SHALL go through a single module that owns the transport. No other part of the UI may reach the backend directly. That module is where the wire contract lives, including the asymmetry that command *arguments* are case-converted across the boundary while serialized payload *fields* are not — a rule that is invisible everywhere else and easy to get wrong.

#### Scenario: a new backend call is needed
- **WHEN** the UI needs a capability from the backend
- **THEN** a typed function is added to the boundary module and used from there

#### Scenario: the payload shape changes
- **WHEN** the backend changes a payload's fields
- **THEN** the boundary module's types are the single place the UI must be updated

### Events for sites that are no longer shown are ignored

A status may still arrive for a site the user has just removed, because a check already in flight cannot be recalled. The UI SHALL NOT render such an event or resurrect the row. What the UI does with the event internally is deliberately unspecified — the guarantee is about what is displayed, and rendering is driven by the site list rather than by the arriving events.

#### Scenario: a site is deleted mid-check
- **WHEN** a status arrives for a site that is no longer listed
- **THEN** no row appears for it and nothing else in the table changes

### The interval floor is stated once and honored everywhere it is shown

The minimum poll interval is the backend's guardrail. Where the UI enforces or displays it, it SHALL agree with the backend, so the field never accepts or suggests a value that will be silently corrected on save.

> **Known duplication.** The floor is presently written as an independent literal in three places — the backend's constant, the form's constant, and the interval input's minimum attribute in the page markup — with no mechanism keeping them in step. Changing the backend's floor therefore requires changing the other two by hand, and this requirement is what should catch them diverging.

#### Scenario: the backend's floor changes
- **WHEN** the minimum interval is changed on the backend
- **THEN** the form's enforcement and the input's stated minimum change with it

#### Scenario: a too-small interval is typed
- **WHEN** the user enters an interval below the floor
- **THEN** the value sent is already raised to the floor, so the field reflects what was actually saved

#### Scenario: the interval is left blank or non-numeric
- **WHEN** no usable interval is entered
- **THEN** a sensible default is used rather than the entry being rejected

### The page markup is the UI's mount contract

The script SHALL attach to elements the page already defines, and every element it requires MUST be present. Removing or renaming one of those anchors breaks the mount outright rather than degrading, so the markup is part of this capability's contract and not merely presentation.

#### Scenario: the UI starts up
- **WHEN** the script mounts
- **THEN** it finds the table body, form controls, notice area, and login-item control the page defines

#### Scenario: an anchor is renamed
- **WHEN** an element the script depends on is renamed in the markup
- **THEN** the UI fails to mount, so markup and script must change together

## Uncovered

_None — every file in the area was read._
