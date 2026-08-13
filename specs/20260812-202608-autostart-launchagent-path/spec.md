# Feature Specification: Launch-at-login survives upgrades

**Feature Branch**: `20260812-202608-autostart-launchagent-path`

**Created**: 2026-08-12

**Status**: Draft

**Input**: User description: "fix this issue: https://github.com/clintcparker/site-checker/issues/25"

**Source issue**: [#25 — Autostart LaunchAgent pins the versioned Cellar path, so it breaks on the first brew upgrade](https://github.com/clintcparker/site-checker/issues/25)

## Problem

When Site Checker turns on "Launch at login", the login item macOS records points at the
*exact copy* of the app that happened to be running at the time. For a Homebrew install that
copy lives in a version-numbered directory (`.../site-checker/1.0.0/...`), which the next
upgrade deletes and replaces with `.../1.0.1/...`.

The result is a login item pointing at something that no longer exists. Site Checker silently
stops opening at login, while its own "Launch at login" checkbox still says it is on — so the
user has no way to tell what went wrong, and no way to fix it short of unticking and reticking
the box.

The same pin causes a second problem: removing Site Checker leaves the login item behind,
pointing at a deleted application, and nothing in the documented removal steps tells the user
to clear it.

Nothing is broken today, because there has been no upgrade yet. It breaks on the first one.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Launch at login keeps working after an upgrade (Priority: P1)

A user installs Site Checker, leaves "Launch at login" on, and later upgrades to a newer
version through their package manager. At the next login, Site Checker opens by itself, exactly
as it did before the upgrade. The user never learns that anything could have gone wrong.

**Why this priority**: This is the defect. Every user who installed the app and upgrades once
loses launch-at-login, and the failure is silent — the app keeps claiming the feature is on.
Fixing the registration is the whole point of the change; everything else in this spec is
follow-through.

**Independent Test**: Enable launch at login on an installed copy, upgrade the installation to a
different version so the previous install directory is removed, and confirm the recorded login
item still resolves to a working copy of Site Checker (and that the app opens at the next login).

**Acceptance Scenarios**:

1. **Given** Site Checker is installed through the package manager and "Launch at login" is on,
   **When** the user upgrades to a newer version, **Then** the login item still refers to an
   application that exists and the app opens automatically at the next login.
2. **Given** Site Checker is installed through the package manager, **When** the user turns
   "Launch at login" on, **Then** the recorded login item refers to the install location that is
   stable across versions, not to the version-numbered one.
3. **Given** Site Checker is running from somewhere other than a package-manager install (a copy
   the user built and placed themselves, or a development build), **When** the user turns
   "Launch at login" on, **Then** the login item refers to that copy exactly as it does today —
   behaviour is unchanged.

---

### User Story 2 - An already-broken login item repairs itself (Priority: P2)

A user who installed the version that shipped this defect upgrades once, so their login item is
already pointing at a deleted directory. The next time they open Site Checker by hand, the app
quietly corrects the login item. From then on, launching at login works again — with no
uninstall, no reinstall, and no checkbox fiddling.

**Why this priority**: Without it, the fix only helps installs that are created after it ships;
anyone already upgraded stays broken forever, and cannot discover the problem because the
checkbox lies. It is a second, smaller behaviour on top of US1, and US1 delivers value without
it, so it is P2 rather than P1.

**Independent Test**: Point an existing login item at a stale, version-numbered location, launch
Site Checker, then re-read the login item and confirm it now names the stable location and
resolves to a real application.

**Acceptance Scenarios**:

1. **Given** a login item for Site Checker exists but names a location that is no longer the
   stable one for the running copy, **When** Site Checker starts, **Then** the login item is
   rewritten to name the stable location, and remains enabled.
2. **Given** no login item for Site Checker exists (the user deliberately turned it off),
   **When** Site Checker starts, **Then** no login item is created — the repair never re-enables
   a feature the user turned off.
3. **Given** a login item exists and already names the correct stable location, **When** Site
   Checker starts, **Then** it is left untouched.
4. **Given** the login item cannot be read or understood (it is damaged, or was written by
   something else), **When** Site Checker starts, **Then** it is left untouched, the app starts
   normally, and nothing is reported to the user.

---

### User Story 3 - Removing Site Checker leaves nothing behind (Priority: P3)

A user decides to remove Site Checker. The removal instructions they follow — in the project
README and in the package manager's own post-install notes — tell them to clear the login item
along with the app and the optional shortcut. Following them leaves no reference to Site Checker
on the machine except the site list, which is documented as deliberately kept.

**Why this priority**: A stale login item after removal is inert — it points at nothing and does
nothing but occupy a file. It is a tidiness and trust problem rather than a broken feature, and
it is fixed by documentation alone.

**Independent Test**: Read the README removal section and the package manager's post-install
notes and confirm both name the login item and how to remove it; then follow them end to end on
a real install and confirm no Site Checker login item remains.

**Acceptance Scenarios**:

1. **Given** a user reads the project README's removal instructions, **When** they follow every
   step, **Then** the login item is removed along with the application.
2. **Given** a user reads the notes the package manager prints after install, **When** they
   follow the removal steps in them, **Then** the login item is removed along with the
   application.

---

### Edge Cases

- **The package manager is installed somewhere non-standard.** The stable location must be
  derived from where the running copy actually is, not from a hard-coded prefix — Apple-silicon
  and Intel machines use different default prefixes, and users may relocate either.
- **The stable location does not resolve.** If the version-independent location is missing or
  broken, registering it would trade one dead login item for another; in that case the app
  registers the location it can prove exists (the running copy) rather than a stable-looking
  path that resolves to nothing.
- **The user launches through the optional `/Applications` shortcut.** The shortcut points into
  the install and the running copy resolves back to the version-numbered location, so this path
  must be treated identically to launching the install directly.
- **The user turned launch-at-login off on purpose.** Neither the corrected registration nor
  the repair may turn it back on. The existing "first run enables it once, and only once"
  behaviour is unchanged.
- **A development build is running.** No package-manager install is involved; registration
  behaves exactly as it does today, and the repair has nothing stable to point at, so it
  leaves the login item alone unless the running copy's own location differs from what is
  recorded.
- **Two copies exist** (a hand-built one and a package-managed one). Whichever copy the user
  runs is the one that registers itself; the app does not attempt to arbitrate between them.
- **Downgrade.** A move to an *older* version is the same event as an upgrade — the previous
  version-numbered directory is removed either way — and must be covered by the same behaviour.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: When Site Checker registers itself to launch at login, it MUST register a location
  that is independent of the installed version whenever one is available for the running copy.
- **FR-002**: The version-independent location MUST be derived from the running copy's own
  location, so that non-default and relocated package-manager installations are handled without
  configuration.
- **FR-003**: If no version-independent location can be determined, or the one determined does
  not resolve to an existing application, Site Checker MUST register the running copy's own
  location — the behaviour in effect today.
- **FR-004**: For copies not installed by the package manager (user-placed and development
  builds), the registered location MUST be unchanged from today's behaviour.
- **FR-005**: On every start, if a launch-at-login registration for Site Checker already exists
  and names a location other than the one FR-001–FR-003 would register, Site Checker MUST
  rewrite it to that location.
- **FR-006**: The repair in FR-005 MUST NOT create a registration where none exists, and MUST
  leave the enabled/disabled state as it found it.
- **FR-007**: If an existing registration cannot be read or interpreted, Site Checker MUST leave
  it untouched and continue starting normally, without reporting anything to the user.
- **FR-008**: A failure at any point in registering or repairing MUST NOT prevent Site Checker
  from starting, and MUST NOT lose or alter the user's site list.
- **FR-009**: The "Launch at login" control in the app MUST continue to report the same
  enabled/disabled state before and after a repair — repairing a registration is not a change
  the user made.
- **FR-010**: The project README's removal instructions MUST include removing the launch-at-login
  registration, alongside the existing steps for the application, the optional shortcut, and the
  site list.
- **FR-011**: The notes the package manager prints after installing MUST include the same
  removal step, so a user who never reads the README still sees it.
- **FR-012**: Site Checker MUST NOT delete the user's launch-at-login registration on its own —
  removal stays a documented, user-performed step.

### Key Entities

- **Launch-at-login registration**: The record that tells macOS to open Site Checker when the
  user logs in. It holds the location of the application to open, and its presence or absence is
  what the app's "Launch at login" checkbox reflects.
- **Install location**: Where the running copy of Site Checker lives. A package-managed install
  has two names for it — one that includes the version and changes with every upgrade, and one
  that does not — and only the second is safe to record anywhere durable.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user with launch-at-login on who moves between versions (upgrade or downgrade)
  still has Site Checker open automatically at the next login, in 100% of cases, with zero
  actions taken by the user.
- **SC-002**: A user whose registration is already stale has it corrected on the first launch of
  Site Checker after the fix reaches them — no reinstall, and no interaction with the "Launch at
  login" control.
- **SC-003**: Following the documented removal steps end to end leaves zero files on the machine
  that reference Site Checker, other than the site list that is documented as intentionally kept.
- **SC-004**: For copies not installed by the package manager, the recorded launch-at-login
  location is byte-for-byte what it is today — zero behaviour change.
- **SC-005**: The state shown by the "Launch at login" control matches whether Site Checker
  actually opens at login, in every scenario listed above.
- **SC-006**: No failure path introduced by this change can stop Site Checker from starting or
  alter the user's site list — demonstrated for a missing, unreadable, and unexpected
  registration.

## Assumptions

- **Scope is the launch-at-login defect and its documentation, nothing else.** The issue's three
  suggestions — register the stable location, document removal, repair stale registrations — are
  all in scope. Nothing about how the app is packaged, signed, or distributed is reopened here.
- **Repair-on-launch is included even though the issue lists it as "consider".** *Judgment call,
  unattended run.* Without it, the fix helps only installs created after it ships; every user
  who already upgraded stays silently broken with no way to notice, since the checkbox reports
  the feature as on. The behaviour is small and bounded (rewrite an existing record; never
  create one), so the cost is low relative to leaving known-broken installs in the field.
- **Removal of the registration stays a manual, documented step (FR-012).** *Judgment call,
  unattended run.* The app is not running when it is uninstalled and the package manager cannot
  run code that touches the user's home directory, so there is no honest way to automate it.
  Documenting it in both places the user might look is what is actually achievable. This mirrors
  how the site list is already handled: kept, and documented as kept.
- **"Version-independent location" is treated as a property the running copy can work out for
  itself**, rather than something configured at build or install time. This keeps the fix
  correct on both processor architectures and on relocated installations, with no new
  configuration surface.
- **The existing first-run behaviour is unchanged**: launch-at-login is on by default the first
  time the app runs, is registered exactly once, and a deliberate untick sticks.
- **No user-visible UI change.** The checkbox, its label, and its behaviour stay as they are;
  everything in this feature happens beneath it.
- **Verification of the upgrade path requires two real builds** installed one after the other,
  which is a manual step — no automated test can observe a package-manager upgrade. The
  underlying location-derivation and repair decisions are expected to be verifiable without one.

## Open Decisions for Review

Recorded here because this run was unattended; the pull request should surface them.

1. **Repair-on-launch is in scope** (see Assumptions). If it is unwanted, US2 and FR-005–FR-007
   can be dropped without affecting US1 or US3.
2. **The stale registration is repaired but never removed** — including when it points at an
   application that no longer exists at all. Removing it would mean the app deleting a login item
   it cannot prove is unwanted, which FR-012 rules out.
3. **No warning is shown for an unreadable registration** (FR-007). The alternative — surfacing
   it in the existing startup warning banner — was rejected as noise the user cannot act on.
