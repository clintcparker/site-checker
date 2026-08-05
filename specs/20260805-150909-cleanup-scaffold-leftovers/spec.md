# Feature Specification: v1 Cleanup — Scaffold Leftovers & Message Clarity

**Feature Branch**: `20260805-150909-cleanup-scaffold-leftovers`

**Created**: 2026-08-05

**Status**: Draft

**Input**: User description: "tackle section 1 of docs/ROADMAP.md"

## Overview

Section 1 of the roadmap ("Cleanup — safe, cosmetic, do anytime") collects five
items left over from the original project scaffold or flagged as wording
problems during the v1 review. None of them are bugs. Four are invisible to whoever runs
the app; one — a pair of near-identical error messages — can actively mislead
the person reading it.

This feature closes all five. The bar for "done" is that the app is
**observably identical** afterwards: same window, same product name, same
identifier, same behavior, same green test suite. Nothing here is allowed to
change what the app does.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Tell a broken file apart from a blocked file (Priority: P1)

The person running Site Checker launches it and sees the warning banner saying
their site list could not be loaded. Today the two reasons this can
happen — the file is scrambled, versus the app was not allowed to open it —
produce messages that read almost the same, so they cannot tell whether to
restore the file from a backup or fix a permissions problem. After this change
the banner names the cause plainly enough to pick the right next step.

**Why this priority**: This is the only item in the roadmap section that anyone
running the app can actually perceive. Everything else is invisible tidiness.

**Independent Test**: Put a scrambled site-list file in place, launch, read the
banner. Separately make the file unreadable, launch, read the banner. The two
messages must lead to different, correct actions without consulting the code.

**Acceptance Scenarios**:

1. **Given** the stored site list contains text that is not a valid list, **When** the app launches, **Then** the banner states that the file's contents could not be understood, and says the existing file has been left in place so it can be recovered.
2. **Given** the stored site list exists but the app is denied permission to open it, **When** the app launches, **Then** the banner states that the file could not be opened, and does not describe the contents as damaged.
3. **Given** either failure, **When** the banner appears, **Then** the app still starts with an empty list and remains fully usable.
4. **Given** the two messages side by side, **When** compared, **Then** they do not share an opening phrase and cannot be mistaken for each other.

---

### User Story 2 - Ship without an unused permission grant (Priority: P2)

The app currently declares that it is allowed to hand links and files off to
other applications — a capability inherited from the project scaffold that no
part of Site Checker ever exercises. Anyone auditing what the installed app is
permitted to do (including the owner, months from now) sees a grant that
overstates what it needs. Removing the grant and the unused packages behind it
makes the declared permissions match the actual behavior.

**Why this priority**: An unused permission grant is the one cleanup item with a
trust dimension rather than a purely aesthetic one, and removing the packages
also drops dead weight from the build.

**Independent Test**: Inspect the app's declared permissions and its dependency
lists — the link-opening capability appears in none of them — then launch the
app and exercise every function to confirm nothing depended on it.

**Acceptance Scenarios**:

1. **Given** the project's permission declaration, **When** it is read after the change, **Then** it grants only the core capability set and no link-opening capability.
2. **Given** the project's dependency declarations (both the frontend and backend ones), **When** searched for the link-opening package, **Then** there are zero matches, including in the resolved dependency lock records.
3. **Given** the rebuilt app, **When** a site is added, edited, checked, and deleted and the launch-at-login box is toggled, **Then** every action works exactly as before.

---

### User Story 3 - The project calls itself Site Checker (Priority: P3)

Every place the project records its own name, description, and author still
carries the scaffold's placeholders — an app called "tauri-app", described as
"A Tauri App", authored by "you". The shipped product name, window title, and
application identifier are already correct, so nobody running the app sees
this; it shows up in build output, dependency records, and crash-report
metadata. Correcting it makes the project's self-description honest.

**Why this priority**: Not user-visible, but it is the item most likely to
cause confusion later — placeholder names surface in tooling output where
they're easy to misread as a different project.

**Independent Test**: Read the project's identity metadata; it names Site
Checker and its real author. Then rebuild and confirm the produced application
bundle, its name, and its identifier are byte-for-byte the same product as
before.

**Acceptance Scenarios**:

1. **Given** the project identity metadata, **When** read after the change, **Then** the name identifies Site Checker, the description describes what Site Checker does, and the author field names the real author rather than a placeholder.
2. **Given** the rebuilt application bundle, **When** compared to the previous build, **Then** the bundle name, the window title, and the application identifier are unchanged.
3. **Given** the rename, **When** the project is built from a clean state, **Then** the build succeeds with no unresolved references to the old placeholder name.

---

### User Story 4 - No dead artwork in the source tree (Priority: P4)

Three logo images from the scaffold sit in the source tree and are referenced
by nothing. They are already excluded from the shipped app, so this costs
nothing at runtime — but they mislead anyone browsing the project into thinking
the app displays them.

**Why this priority**: Zero functional impact and zero size impact on the
shipped product. Pure tidiness, so it goes last among the removals.

**Independent Test**: Confirm no page, style, or code references the three
images, delete them, and confirm the app builds and looks identical.

**Acceptance Scenarios**:

1. **Given** the source tree after the change, **When** searched for the three scaffold logo files, **Then** none are present.
2. **Given** the rebuilt app, **When** opened, **Then** the interface renders identically to before, with no missing-image placeholders.

---

### User Story 5 - The URL-scheme note explains its own rule (Priority: P5)

The explanatory note on the helper that decides whether a typed URL already
begins with a scheme says why the check exists but not what it actually
accepts. A maintainer reading it cannot tell which characters count as part of
a scheme without reading the code beneath it.

**Why this priority**: Documentation-only, affects one reader (the maintainer)
at one moment (the next time that function is touched).

**Independent Test**: Read the note in isolation and predict, correctly,
whether a given example string would be treated as already having a scheme.

**Acceptance Scenarios**:

1. **Given** the note on the scheme-detection helper, **When** read without looking at the code, **Then** it states which characters are accepted before the separator and that a separator at the very start does not count.
2. **Given** the described rule, **When** compared against the code's actual behavior, **Then** they agree.

---

### Edge Cases

- **A removal turns out not to be dead.** If deleting the permission grant, the packages, or the images breaks any behavior, the change is wrong and must be reverted rather than patched around — nothing in this feature may alter what the app does.
- **The rename leaks into the shipped product.** Renaming the project internally must not change the bundle's name, its identifier, the executable it launches, or the window title. If a rename would change any of those, the shipped values must be pinned explicitly so they stay put.
- **Stale build state hides a broken rename.** The rename must be verified from a clean build, not an incremental one, so cached artifacts under the old name cannot mask an unresolved reference.
- **Stale installed packages hide a removed dependency.** Dependency removal must be verified from a fresh install, not against an existing installed-packages directory that still contains the removed package.
- **An existing test asserts the old message text.** Rewording the warning messages must leave the automated suite green; any test that pins the old wording is updated to pin the new distinction, not deleted.
- **The corrupt file is still recoverable.** Reworded messages must preserve the existing promise that a damaged site-list file is left untouched on disk until the next save.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The application MUST NOT declare the link/file-opening capability in its permission set.
- **FR-002**: The project MUST NOT declare a dependency on the link/file-opening package in either its frontend or its backend dependency declarations, and the resolved dependency records MUST be regenerated so neither still lists it.
- **FR-003**: The project's self-describing identity metadata MUST name Site Checker, describe its actual purpose, and credit its real author, with no scaffold placeholder values remaining.
- **FR-004**: The shipped product's name, application identifier, window title, and launch behavior MUST be unchanged by FR-003.
- **FR-005**: The source tree MUST NOT contain the three unreferenced scaffold logo images.
- **FR-006**: The warning shown when the stored site list cannot be *opened* MUST be distinguishable, on its own, from the warning shown when the stored site list cannot be *understood* — the two MUST NOT share an opening phrase, and each MUST indicate its own cause.
- **FR-007**: The "contents could not be understood" warning MUST continue to state that the existing file has been left in place.
- **FR-008**: Both warning paths MUST continue to start the app with an empty, fully usable site list rather than failing to launch.
- **FR-009**: The explanatory note on the scheme-detection helper MUST state the character rule it applies (letters, digits, `+`, `-`, `.` before the separator) and that a separator at position zero does not count as a scheme.
- **FR-010**: No change in this feature may alter observable application behavior beyond the wording of the two warning messages in FR-006.
- **FR-011**: The full automated test suite and the lint gate MUST pass after every item, with no test deleted to accommodate a change.

### Out of Scope

Roadmap sections 2 through 7 are explicitly excluded from this feature —
robustness fixes, durability work (including atomic saves), concurrency
hardening, bundle-size and packaging work, test-coverage gaps, and all v2
features. Any of those encountered while working here is appended to the
roadmap, not fixed in place.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Zero occurrences of the link/file-opening capability or package remain anywhere in the project's declarations, source, or resolved dependency records.
- **SC-002**: Zero scaffold placeholder identity values ("tauri-app", "A Tauri App", "you") remain in the project's metadata.
- **SC-003**: Zero unreferenced scaffold image files remain in the source tree.
- **SC-004**: Shown only the two warning messages, a reader who has never seen the code identifies which one means "the file is damaged" and which means "the file could not be opened" — correctly, both times.
- **SC-005**: A full clean build succeeds, and the resulting application's bundle name, identifier, and window title are identical to the pre-change build.
- **SC-006**: The complete automated suite passes and the lint gate reports zero warnings, with the same number of tests as before or more.
- **SC-007**: After a fresh install and launch, a person can add a site, edit it, watch its status change, delete it, and toggle launch-at-login — all with the same results as before the change.

## Assumptions

- **The roadmap's list of opener references was incomplete; the intent governs.** The roadmap names two locations for the unused opener plugin (the permission declaration and the frontend dependency list). The backend dependency declaration also carries it. All three are treated as in scope, since the stated goal is removing the unused plugin, not editing two specific files.
- **The internal library name is renamed alongside the package name.** The roadmap lists three identity fields; the project's internal library name carries the same placeholder and is referenced from exactly one place. Renaming it too is treated as part of "fix scaffold identity metadata" — leaving it behind would be a half-finished rename. If renaming it proves to carry any risk to the produced bundle, it is dropped and the other three fields still ship.
- **The author is the repository's owner** (Clint Parker, me@clintparker.com), taken from the existing commit history rather than invented.
- **The description is written from the product's actual behavior** as documented in the README and constitution, not from the scaffold text.
- **Items are independently shippable and may land in any order**, since none of them depend on another. Priority reflects value, not sequencing.
- **"Behavior unchanged" is verified by the existing automated suite plus a manual launch**, not by a new end-to-end test harness — consistent with the project's existing exclusion of UI end-to-end tests.
- **The two reworded warning messages are user-visible text, not persisted data**, so changing them is not a breaking change to the user's stored file.
