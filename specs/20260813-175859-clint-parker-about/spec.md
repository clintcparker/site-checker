# Feature Specification: Author Attribution in About

**Feature Branch**: `20260813-175859-clint-parker-about`

**Created**: 2026-08-13

**Status**: Draft

**Input**: User description: "add Clint Parker to the about window and add a link to clintparker.com"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See who made this (Priority: P1)

Someone running Site Checker opens the app's About surface and sees, alongside the app's
name and version, that the app was created by Clint Parker. Today the About surface names
the app but names nobody — the person who wrote it is invisible from inside the running
app.

**Why this priority**: This is the whole of the request's first half, and it stands alone.
Shipping only this already delivers the attribution; the link is an enhancement on top of a
surface that is already correct.

**Independent Test**: Launch the app, open the About surface, and read it. The author's name
is present and spelled "Clint Parker". No other part of the app needs to change for this to
be verifiable.

**Acceptance Scenarios**:

1. **Given** the app is running, **When** the user opens the About surface, **Then** the
   surface shows the app name, the app's version, and an attribution naming Clint Parker as
   the app's creator.
2. **Given** the About surface is open, **When** the user reads the attribution, **Then**
   the name appears exactly as "Clint Parker" — not an email address, not a handle, not a
   username.
3. **Given** the user closes the About surface, **When** they return to the site list,
   **Then** the list and its checks are exactly as they were — opening and closing About
   changes nothing about what is being checked or when.

---

### User Story 2 - Get to the author's site (Priority: P2)

From that same About surface, the user follows a link to clintparker.com and lands there in
their normal browser. Site Checker itself never renders the page.

**Why this priority**: It depends on the About surface carrying the attribution (P1) but
adds a distinct capability — leaving the app for the author's site. Deferring it still
leaves a shipped, correct About surface.

**Independent Test**: With the About surface open, activate the link and confirm the default
browser comes forward at clintparker.com while Site Checker stays running and unchanged.

**Acceptance Scenarios**:

1. **Given** the About surface is open, **When** the user activates the clintparker.com
   link, **Then** the address opens in the user's default browser and Site Checker remains
   open and continues checking sites on schedule.
2. **Given** the user activates the link twice in rapid succession, **When** the second
   activation lands within the app's existing repeat-suppression window, **Then** only one
   browser navigation is requested.
3. **Given** the operating system refuses to open the address, **When** the refusal comes
   back, **Then** the user sees a plain-language message explaining that the address could
   not be opened, and the app stays usable — no crash, no stalled checks, no lost sites.
4. **Given** the link is visible, **When** the user inspects it, **Then** its destination is
   unambiguously clintparker.com and it is presented as something activatable, not as bare
   unclickable text.

---

### Edge Cases

- **The About surface does not exist yet in the form the request assumes.** The app
  currently ships a single window and no hand-built About view; on macOS the About item is
  supplied by the system. Whatever surface ends up carrying this content must be reachable
  by an ordinary user without special knowledge — a menu item or an in-app control, not a
  keyboard secret. See Assumptions and Open Decisions.
- **The link is activated repeatedly.** The app already treats a second activation of the
  same address inside a one-second window as impatience rather than a second request; the
  About link behaves the same way, so drumming on it does not open a stack of tabs.
- **No browser is available, or the OS refuses the request.** The user gets the same kind of
  visible, plain-language message the app already uses for a refused open. The failure is
  never silent and never fatal.
- **The machine is offline.** The app does not test whether clintparker.com is reachable
  before handing the address over; the browser reports its own failure, as it would for any
  link. Site Checker does not check, monitor, or report on clintparker.com.
- **A development or unreleased build.** Released builds carry a real version stamped at
  build time; local builds carry a placeholder. The About surface must render legibly in
  both cases rather than showing a blank where the version belongs.
- **The window is at its minimum size.** The About content must remain readable and the link
  activatable at the app's smallest supported window size.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The app MUST provide an About surface that a user can open from the running
  app through an ordinary, discoverable control.
- **FR-002**: The About surface MUST display the app's name.
- **FR-003**: The About surface MUST display the app's version as reported by the running
  build.
- **FR-004**: The About surface MUST attribute the app to "Clint Parker", rendered with that
  exact spelling and capitalisation.
- **FR-005**: The About surface MUST present an activatable link whose destination is
  `https://clintparker.com`.
- **FR-006**: Activating that link MUST hand the address to the user's default browser and
  MUST NOT render the page inside Site Checker.
- **FR-007**: Activating the link MUST leave the app running and its scheduled checks
  undisturbed — no site is added, removed, re-scheduled, or re-checked as a side effect.
- **FR-008**: Repeat activations of the link within the app's existing repeat-suppression
  window MUST result in a single browser navigation.
- **FR-009**: A refusal from the operating system when opening the address MUST surface to
  the user as a visible, plain-language message and MUST NOT crash the app or stop checks.
- **FR-010**: This feature MUST NOT read, write, or migrate the user's saved site list, and
  MUST NOT introduce any new file the app owns.
- **FR-011**: This feature MUST NOT issue any network request of its own — the app does not
  fetch, ping, or check clintparker.com, it only hands the address to the browser.
- **FR-012**: The attribution and the link MUST be readable and the link activatable at the
  app's minimum supported window size.

### Key Entities

None. This feature displays fixed text and one fixed address; it introduces no new data, no
new persisted field, and no new user-editable value.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user who has never seen the app before can find out who made it in under 15
  seconds from launch, using only the app's visible controls.
- **SC-002**: 100% of launches show the same attribution — there is no state, no setting,
  and no timing in which the About surface appears without the author's name.
- **SC-003**: Activating the link brings the browser forward at clintparker.com on the first
  attempt in every trial, with no intermediate dialog or confirmation step.
- **SC-004**: Ten rapid activations of the link within one second produce exactly one
  browser navigation.
- **SC-005**: With the address made impossible to open, the user sees an explanatory message
  within one second and the app continues checking every configured site on schedule — zero
  missed checks attributable to the failure.
- **SC-006**: The user's saved site list is byte-for-byte unchanged across a session in
  which the About surface is opened, the link is activated, and the app is quit.

## Assumptions

- **The About surface must be established, not merely edited.** The request says "the about
  window", but the app as it stands has no About view of its own — one window, no custom
  menu, and on macOS an About item supplied by the system that names the app and nothing
  else. This spec therefore requires an About surface that carries the content, and leaves
  *which* surface — enriching the system-supplied panel versus adding a small in-app About
  view — to the planning step, because both satisfy every requirement above and the choice
  turns on implementation constraints rather than on user need.
- **The attribution reads as authorship.** "Created by Clint Parker" is the intended sense.
  No role, title, company, or contact detail is invented; the email already recorded in the
  project's package metadata is deliberately not surfaced.
- **The address is `https://clintparker.com`** — secure scheme, apex domain, no path. The
  description gave the bare domain; https is the only reasonable default in 2026.
- **Showing the version is in scope.** The description asked only for a name and a link, but
  an About surface that omits the version is not one users recognise, and the release
  pipeline already stamps a real version into every released build. Included as FR-003 and
  flagged below as an addition beyond the literal request.
- **Link behaviour reuses what the app already does.** The site list's URL cells already
  open addresses in the default browser and already suppress a repeat activation inside a
  one-second window; the About link is specified to behave identically rather than to invent
  a second, differing rule.
- **This does not widen the product's scope.** The project constitution reserves Site Checker
  to one question for one person on one Mac. Attribution is chrome around that question, not
  a new capability: no alerting, no history, no network of its own, no new stored data. It is
  called out here because the constitution requires scope changes to be stated explicitly —
  and this is asserted to be a non-change.
- **No localisation.** The app ships in English only; the attribution and link text are
  English strings.

## Open Decisions

Recorded for the pull request to surface. This run was unattended; each was decided here
rather than asked.

1. **Which surface carries the About content** — the system-supplied About panel, enriched,
   versus a small in-app About view. Deferred to planning; the spec is satisfied either way.
   The in-app route is the safer bet if the system panel turns out not to support an
   activatable link, and the system route is less code if it does.
2. **Including the version (FR-003)** — an addition beyond the literal request, on the
   grounds that a version-less About surface is not one users recognise. Drop FR-003 if the
   addition is unwanted; nothing else in the spec depends on it.
3. **Exact attribution wording** — "Created by Clint Parker" is the assumed phrasing.
   Alternatives ("By Clint Parker", "© Clint Parker") satisfy FR-004 equally as long as the
   name is spelled exactly.
