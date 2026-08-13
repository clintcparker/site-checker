# Frontend — Living Spec


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

### A row's address is the control that visits the site

The address shown in a row SHALL be activatable, and activating it SHALL open that site in the user's default browser. Visiting a listed site is otherwise a copy, a switch, and a paste; this is the same intention in one action.

Activating it MUST NOT navigate the dashboard itself anywhere. That is unrecoverable — this window has no way back — so the control SHALL be one that carries nothing for the window to follow, under any modifier, any middle-click, and any path where the script did not run. The cost, accepted deliberately, is that assistive technology announces it as a button rather than as a link.

The address SHALL be identifiable as openable before it is activated, and the whole stored address SHALL be what opens regardless of how much of it the row displays.

A row's address control MUST be keyed on something the row's own edit and delete controls cannot match, so activating an address can never reach them. This is structural, not a convention to be remembered.

#### Scenario: a listed site's address is activated
- **WHEN** the user activates the address shown in a row
- **THEN** that exact address opens in the default browser
- **AND** the dashboard still shows the table, still updating

#### Scenario: the address is longer than the row shows
- **WHEN** the displayed address is wrapped or shortened
- **THEN** activating it still opens the whole stored address

#### Scenario: the pointer rests on the address
- **WHEN** the user hovers the address without activating it
- **THEN** it reads as something that can be opened

#### Scenario: a row's other controls are used
- **WHEN** the user activates a row's edit or delete control
- **THEN** nothing is opened

### A labelled site shows its label as text and its address as the control

Where a site has a label, the label SHALL be the row's primary line and the address the secondary one; where it has none, the address SHALL be the primary line. The label itself SHALL never be activatable — it names the site, it does not identify a destination.

Adding or removing a label moves the address between those two lines, changing which element holds it. That transition MAY rebuild the row's name cell, which is not an exception to the repaint rule: a label only changes on a save the user just made, never on a repaint.

#### Scenario: a site has a label
- **WHEN** a labelled row is displayed
- **THEN** the label is the primary line and is inert
- **AND** the address beneath it is the control

#### Scenario: the label is clicked
- **WHEN** the user clicks a row's label
- **THEN** nothing opens

#### Scenario: a label is added to a site
- **WHEN** a site gains or loses its label
- **THEN** the address moves between the primary and secondary line and remains activatable

### An address the app will not open is shown, but never offered

An address that is not a web address SHALL be displayed as ordinary text: not underlined, not reachable by keyboard, and not announced as something that can be acted on. It MUST NOT be hidden, flagged as invalid, or repaired — it is shown as it is stored, simply without an affordance the app cannot honour.

The scheme alone is NOT sufficient grounds to offer an address. An address the backend would refuse SHALL be rendered as that same ordinary text even when it begins with a web scheme — so an address that does not parse as a URL, or that carries no host, gets no affordance. The judgement stays synchronous, because it is made on every repaint.

Because the parse is delegated to the host web engine, and engines disagree about what is a forbidden character in a host — some refusing it, some percent-encoding it into the host and reporting success — the decision MUST NOT rest on the engine's verdict alone where they differ. A host containing a percent sign or whitespace SHALL be treated as not openable, which closes that gap identically in every engine, including the one the app actually ships in. This applies to the *host* only: a space elsewhere in the address, such as in its path, is accepted here exactly as the backend accepts it.

> **Known duplication.** Which addresses may be opened is decided in two places: here, to choose how a row renders, and in the backend, which is authoritative and refuses anything else. It is not fetched from the backend because it is a rendering decision taken on every repaint, once a second per row. Should the two ever disagree, the UI would offer something the backend then refuses — which surfaces as a visible message rather than as silence.

#### Scenario: a saved address uses a non-web scheme
- **WHEN** a row's stored address is not a web address
- **THEN** it appears as plain text with no affordance
- **AND** it is skipped when tabbing through the row

#### Scenario: such an address is clicked
- **WHEN** the user clicks it anyway
- **THEN** nothing happens

#### Scenario: a web-scheme address the backend would refuse
- **WHEN** a row's stored address begins with a web scheme but does not parse or names no host
- **THEN** it appears as plain text with no affordance, exactly as a non-web address does

#### Scenario: the host itself is malformed
- **WHEN** a stored address's host contains a space or a percent sign
- **THEN** it is not offered as openable, whichever web engine is rendering the app

#### Scenario: a space appears later in the address
- **WHEN** a stored address has a valid host and a space in its path
- **THEN** it is still offered as openable, matching what the backend will open

### Everything reachable by pointer in a row is reachable by keyboard

The address control SHALL take focus in the row's own reading order, ahead of that row's action controls, and SHALL activate from the keyboard exactly as it does from a click. Keyboard focus SHALL be visibly indicated when it arrives that way, and only that way — a mouse click MUST NOT leave an indicator behind.

Because the table repaints every second, focus on the address control MUST survive a repaint, under the same reconciliation rule the rest of the table follows.

#### Scenario: the user tabs through a row
- **WHEN** focus moves through a row
- **THEN** the address takes focus before that row's edit and delete controls, with a visible indication

#### Scenario: the focused address is activated
- **WHEN** the user activates the focused address from the keyboard
- **THEN** the same site opens as a click would have opened

#### Scenario: a repaint occurs while the address has focus
- **WHEN** a status arrives, or the age counters tick, while the address holds focus
- **THEN** focus stays where it was

### Repeated rapid activations of one address open it once

Activating the same address again within a short window SHALL be treated as impatience rather than as a request for a second browser window, and suppressed. The window is per address: two different sites activated in quick succession are two intentional visits. A *suppressed* activation MUST NOT extend the window, or someone drumming on the control would push it ahead of themselves indefinitely and never open the site at all.

#### Scenario: the address is double-clicked
- **WHEN** the user activates one address twice in quick succession
- **THEN** the site is opened once

#### Scenario: two different sites are activated in a row
- **WHEN** the user activates one address and then another immediately
- **THEN** both are opened

#### Scenario: the same site is deliberately revisited
- **WHEN** the user activates the same address again after the window has passed
- **THEN** it opens again

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

Warnings that do not stop the app — a site list that could not be read, a save that did not reach disk, a site that could not be opened — SHALL be surfaced in a persistent notice rather than a modal, both when reported at startup and when they occur later. The app stays usable throughout.

A failure to open a site belongs in that notice and NOT beside the form: it is not about anything the user typed, and there is nothing in the form to correct.

#### Scenario: the saved list could not be read at startup
- **WHEN** the app starts and reports a load warning
- **THEN** the notice is displayed and the rest of the UI works normally

#### Scenario: a save fails during use
- **WHEN** a write failure is reported while the app is running
- **THEN** the notice appears without interrupting the current interaction

#### Scenario: a site cannot be opened
- **WHEN** opening a site is refused
- **THEN** the reason appears in the notice rather than beside the form
- **AND** the table keeps updating and every other action still works

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
