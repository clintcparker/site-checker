# Feature Specification: Clickable URLs Open in the Default Browser

**Feature Branch**: `20260813-105424-urls-clickable-and`

**Created**: 2026-08-13

**Status**: Draft

**Input**: User description: "urls should be clickable, and should open in the default browser"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Visit a site from its row (Priority: P1)

A user watching the dashboard sees a site reported as down. They want to look at it themselves — to confirm the outage, read the error page, or check whether it has come back. Today the URL is inert text, so the only way through is to select it, copy it, switch to a browser, and paste. The user wants to click the URL in the row and land on that page in the browser they already use.

**Why this priority**: This is the whole of the request and the entire user-facing value. Without it there is no feature. It is also self-contained: it delivers value on its own, with no dependency on the other stories.

**Independent Test**: Add a site, click its URL in the table, and confirm the operating system's default browser comes forward showing that address. Fully exercisable with a single site and no failure conditions.

**Acceptance Scenarios**:

1. **Given** a listed site with no label, **When** the user clicks the URL shown in its row, **Then** the system's default browser opens that exact address.
2. **Given** a listed site that has a label, **When** the user clicks the URL shown beneath the label, **Then** the default browser opens that address.
3. **Given** the user has clicked a URL, **When** the browser opens, **Then** the dashboard itself still shows the site table, unchanged and still updating.
4. **Given** a URL is displayed, **When** the user moves the pointer over it, **Then** it is visibly identifiable as something that can be opened, before any click.

---

### User Story 2 - Open a site without a mouse (Priority: P2)

A user driving the app from the keyboard wants to reach a site's page the same way a clicking user does — tab to the URL and press a key — rather than being forced to the pointer for this one action.

**Why this priority**: The dashboard's existing row controls are already keyboard-reachable, and the living spec commits to focus surviving repaints. A new interactive element that only responds to a pointer would be a step backwards from the standard the app already holds. It ranks below P1 because pointer users get the value first.

**Independent Test**: Tab through a row until the URL takes focus, press Enter, and confirm the browser opens the same address a click would have. Testable with one site and no failure conditions.

**Acceptance Scenarios**:

1. **Given** the table has rows, **When** the user tabs through a row, **Then** the URL receives keyboard focus and is visibly indicated as focused.
2. **Given** a URL has keyboard focus, **When** the user presses Enter, **Then** the default browser opens that address.
3. **Given** a URL has keyboard focus, **When** a repaint occurs (a status arrives, or the age counter ticks), **Then** the focus stays on that URL.

---

### User Story 3 - A refusal explains itself (Priority: P3)

A user clicks a URL and, for reasons outside the app, nothing opens. Rather than clicking repeatedly at an app that appears to have ignored them, they want to be told that the attempt failed and why.

**Why this priority**: An uncommon path, but the app already commits to never failing silently, and a click that produces no visible result and no message is indistinguishable from a frozen UI. Ranks last because it only matters when something else has already gone wrong.

**Independent Test**: Force the open attempt to fail and confirm a message appears and the dashboard stays usable. Testable in isolation from the success path.

**Acceptance Scenarios**:

1. **Given** the browser cannot be launched, **When** the user clicks a URL, **Then** the reason is surfaced in the app's existing notice area.
2. **Given** an open attempt has just failed, **When** the user interacts with the rest of the app, **Then** everything else — adding, editing, deleting, checking — continues to work.

---

### Edge Cases

- **A stored URL is not http or https.** URLs are validated as http/https when saved, but the saved-sites file is a plain file on disk that the user (or anything else) can edit by hand, and the load path does not re-validate the scheme. A hand-edited entry can therefore reach the table carrying any scheme at all. Such an entry must not be handed to the operating system to open.
- **A very long URL.** The site column is narrow. A URL long enough to wrap or be visually truncated must still open the whole address, not the visible fragment.
- **A site is deleted between the click and the open.** The address was valid when clicked; the removal does not need to cancel the open, but the app must not error or resurrect the row.
- **Rapid repeated clicks.** Clicking the same URL several times in quick succession is a user expressing impatience, not a request for many browser tabs.
- **A click that lands during a repaint.** The table reconciles continuously; a click must not be swallowed or misrouted because a repaint happened to run at that moment.
- **A click on the label rather than the URL.** A labelled row shows a name and an address; only one of them is an address.
- **No default browser is configured.** The app cannot resolve a handler and must report that rather than appear to do nothing.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Each site's URL, as displayed in the table, MUST be activatable by the user to open that address.
- **FR-002**: Activating a URL MUST open it in the operating system's default browser — an application outside this app, chosen by the user's system settings.
- **FR-003**: Activating a URL MUST NOT navigate the dashboard's own window away from the site table. The dashboard offers no way back, so navigating it away would strand the user in an unrecoverable state.
- **FR-004**: A URL MUST be visually distinguishable as activatable before it is activated, so its behaviour is discoverable without a trial click.
- **FR-005**: A URL MUST be reachable and activatable by keyboard, not by pointer alone, and MUST show a visible focus indication when focused.
- **FR-006**: The address opened MUST be the site's full stored URL, exactly as saved, regardless of how much of it is visible in the row.
- **FR-007**: The system MUST NOT open an address whose scheme is anything other than http or https, even when such an address is present in the site list. A URL that is not opened MUST NOT be presented as activatable.
- **FR-008**: When a labelled site shows both a label and a URL, the URL MUST be the activatable element. The label is a user-chosen name, not an address.
- **FR-009**: When an address cannot be opened, the reason MUST be surfaced through the app's existing non-blocking notice mechanism, and the app MUST remain fully usable.
- **FR-010**: Activating a URL MUST NOT trigger that row's other actions, and MUST NOT alter the site's stored data, its check schedule, or its current status.
- **FR-011**: The URL element MUST survive the table's in-place reconciliation the way existing row elements do — updated rather than recreated on repaint — so that focus and hover on it are not interrupted.
- **FR-012**: Repeated rapid activations of the same URL SHOULD result in a single browser navigation rather than one per click.

### Key Entities

- **Site**: An existing entity. This feature reads its URL and its label and adds no field to it. Nothing about this feature is persisted.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can go from seeing a site in the dashboard to viewing that site in their browser in one action, down from the current five (select, copy, switch apps, paste, submit).
- **SC-002**: 100% of listed sites with an http or https address can be opened this way; 0% of listed entries with any other scheme can.
- **SC-003**: The browser comes forward within two seconds of activation on a normally loaded machine.
- **SC-004**: Every task achievable with a pointer in this feature is also achievable with the keyboard alone.
- **SC-005**: Opening a site never costs the user their place: after activation, the dashboard still shows the same table, still updating, with no state lost.
- **SC-006**: An activation that cannot be completed produces a visible explanation 100% of the time, and never leaves the user without feedback.

## Assumptions

- **The user has a default browser and wants it.** "The default browser" is read as the handler the operating system already resolves for http/https. The app does not offer a browser preference of its own; adding one is out of scope.
- **Only the URL is activatable, not the whole row or the label.** The request names URLs specifically. Making the entire site cell clickable would put a large invisible target next to the Edit and Delete buttons and would make a labelled row's name behave like an address. Recorded as an open decision below.
- **Nothing new is stored.** The feature is a display and dispatch behaviour only; no new persisted field, no click history, no "last visited" state.
- **The scheme guard is the app's own responsibility.** URLs are validated as http/https on save today, so in normal use every stored URL already qualifies. FR-007 exists because the load path does not re-validate, meaning the guarantee does not hold for a hand-edited file. The guard is therefore treated as belonging to this feature rather than assumed from elsewhere.
- **The status indicator's existing tooltip is untouched.** The failure reason stays where it is; this feature adds no competing hover behaviour to the same element.
- **Scope is the site table only.** URLs appearing anywhere else in the app — form fields, notice text — are out of scope.

## Open Decisions for Review

These were decided without the user present and should be surfaced for confirmation when this work is proposed:

1. **Only the URL text is activatable** — not the whole site cell, and not the label on a labelled row. The alternative (whole-cell target) is easier to hit but blurs what is being opened and crowds the row actions. If a larger hit target is wanted, this is the decision to revisit.
2. **No new visual column or icon.** The URL is styled to read as activatable in place, rather than gaining an external-link glyph or a dedicated "Open" button beside Edit and Delete. This keeps the row's glanceability, which the frontend living spec treats as the point of the display.
3. **Non-http/https entries are shown but inert.** Such an entry is rendered as plain text rather than being hidden, flagged as invalid, or repaired on load. Re-validating URLs at load time would be a broader change to the backend's load contract and is deliberately not proposed here.
