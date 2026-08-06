# Feature Specification: Concurrency & Robustness Hardening

**Feature Branch**: `20260806-120325-section-1-docs`

**Spec Directory**: `specs/20260806-120353-concurrency-hardening`

**Created**: 2026-08-06

**Status**: Draft

**Input**: User description: "tackle section 1 of docs/ROADMAP.md"

## Context

`docs/ROADMAP.md` §1 ("Concurrency & robustness hardening") lists four items. Three
are latent defects with an expected fix; the fourth is a behavioural note recorded
deliberately with "no action expected". This feature closes the three.

None of the three is a live bug in the app as shipped. Each is a correctness hole in
the layer beneath the UI — reachable by a future caller, an importer, or a restore
path, but not by a user clicking around today. The value is that the shell stops
depending on the UI to avoid a state the core permits.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The app stays usable after an internal fault (Priority: P1)

Something inside the app fails unexpectedly while it is partway through reading or
writing the user's site list. Today that single fault permanently disables every
later action: adding a site, editing one, deleting one, and even listing what is
already there all stop working, and the only cure is quitting and relaunching. The
window keeps ticking status updates for the sites it already knows about, so the app
looks alive while being unable to accept any change.

After this story, one internal fault costs at most the operation it happened in. The
app recovers, the next action succeeds, and if the user's list may have been left
mid-change the user is told rather than left guessing.

**Why this priority**: Widest blast radius of the three. It applies to every shared-
state access in the app (ten today, spread across the site list, the startup warning,
and the timer registry), and it is the only one of the three whose failure mode is
"the whole app stops accepting input". The other two are wrong answers in narrow
paths; this one is a total loss of function.

**Independent Test**: Provoke a fault while the app is partway through a change to
its shared state, then perform each ordinary action — list, add, edit, delete — and
confirm every one still completes.
Delivers value on its own: a fault that used to require a relaunch no longer does.

**Acceptance Scenarios**:

1. **Given** a fault has previously occurred while the site list was being read or
   written, **When** the user adds a site, **Then** the site is added, saved, and
   begins being checked — exactly as if no fault had occurred.
2. **Given** the same prior fault, **When** the user edits or deletes a site,
   **Then** the change is applied and saved.
3. **Given** the same prior fault, **When** the window lists the user's sites,
   **Then** the list is returned rather than the app becoming unresponsive to it.
4. **Given** a fault has occurred while the app's list of running checks was being
   modified, **When** the user adds, edits, or deletes any site, **Then** that
   site's checks start, restart, or stop as normal.
5. **Given** a fault has left the site list possibly mid-change, **When** the app
   recovers from it, **Then** the user sees a warning telling them their saved list
   may not reflect their most recent change, using the same banner the app already
   uses for load and write problems.

---

### User Story 2 - A refused add never leaves a ghost row (Priority: P2)

The user adds a site and the app cannot store it — not because the disk write failed,
but because the list already holds an entry with that identity and the core refuses
it outright, changing nothing.

Today the two failures are indistinguishable to the part of the app that talks to the
window. A refusal is reported as though the disk write failed, which carries the
promise "your change is still here, it just isn't saved yet". That promise is false
in this case: nothing was applied anywhere. The window adds the row anyway, the app
starts checking a site that is in no list, and the row disappears at the next launch.

After this story, a refusal is reported as a refusal: no row appears, no checks start,
and the user is told the add did not happen. A genuine write failure keeps its
existing behaviour — the row stays, checks run, and the banner says it could not be
saved.

**Why this priority**: The user is shown an outcome that did not occur, and the app
starts work for a site it did not store. Ranked below P1 because it costs one add
rather than the whole session, and above P3 because it produces a visibly wrong state
rather than a silently lost field.

**Independent Test**: Ask the core to store a site whose identity is already present,
and confirm the window shows no new row, no checks begin for it, and the message the
user sees says the site was not added. Deliverable on its own without either other
story.

**Acceptance Scenarios**:

1. **Given** the list already holds an entry with the identity being added, **When**
   the add is attempted, **Then** no row is added to the window, no checking begins
   for it, and the user is told the site was not added.
2. **Given** that same refusal, **When** the app is relaunched, **Then** the saved
   list is unchanged — the refusal neither added nor removed anything.
3. **Given** a valid, non-duplicate site whose save to disk fails, **When** the add
   is attempted, **Then** today's behaviour is preserved: the row appears, checking
   begins, and the banner reports that the list could not be saved.
4. **Given** a valid, non-duplicate site that saves successfully, **When** the add
   is attempted, **Then** the row appears, checking begins, and no warning is shown.

---

### User Story 3 - Two edits to one site cannot discard each other (Priority: P3)

Two changes to the same site are applied at nearly the same moment. Today each one
separately reads the site's current state, decides what the updated entry should look
like from that reading, and then writes its result. If the second reads before the
first has written, both decide from the same stale picture and the later write
silently discards what the earlier one established.

The concrete loss is the app's memory of which request method a site needs. That is
learned once, at a cost of an extra failed request, and re-learning it means another
wasted round trip against the user's site. Two overlapping edits can throw it away.

After this story, an edit reads the current state and writes its result as one
indivisible step, so the second edit always decides from the first edit's result.

**Why this priority**: Lowest of the three, and the roadmap already downgraded it.
The only route to it from the shipped window — a fast double-submit on one row — was
closed by the in-flight submit guard added in `002-robustness`. It remains real one
layer down, so any future caller that is not the current window reopens it, but no
user can reach it today.

**Independent Test**: Apply two edits to the same site so that the second begins
before the first completes, and confirm the final saved entry reflects the second
edit applied on top of the first's result — not on top of the state before either.

**Acceptance Scenarios**:

1. **Given** two edits to the same site that overlap in time, **When** both complete,
   **Then** the saved entry reflects the later edit applied to the earlier edit's
   result, and no decision in it was made from a picture the earlier edit had already
   replaced.
2. **Given** an edit to a site whose address is unchanged, **When** it is applied,
   **Then** the app's learned request method for that site is preserved — unchanged
   from today.
3. **Given** an edit that changes a site's address, **When** it is applied, **Then**
   the learned request method is discarded so it is re-learned against the new
   address — unchanged from today.
4. **Given** an edit naming a site that is no longer in the list, **When** it is
   applied, **Then** the user is told the site no longer exists and nothing is
   written — unchanged from today.

---

### Edge Cases

- **A fault interrupts a change midway.** The user's in-memory list may hold a
  half-applied change when recovery resumes. Recovery continues from whatever state
  is there rather than discarding it, and the user is warned that the saved list may
  not match what they last asked for. This is the honest position for a single-user
  tool: the app cannot reconstruct the intended state, and throwing the list away
  would be worse than keeping it.
- **A fault occurs in the timer registry rather than the site list.** Recovery is the
  same, but no warning is shown: which checks are running is ephemeral by design
  (Constitution II), it is rebuilt on the next launch, and every scheduling action
  replaces rather than accumulates.
- **A site is refused as a duplicate *and* the list could not have been saved
  anyway.** The refusal is decided before anything is written, so the user is told
  the site was not added. There is no second, contradictory message.
- **A site is deleted between an edit being started and applied.** Unchanged: the
  user is told the site no longer exists, and nothing is written.
- **A fault occurs during startup, before the window is showing.** Recovery applies
  there too — the list still loads and checks still start.
- **Recovery happens more than once in a session.** Each recovery is independent; the
  app does not accumulate a permanent degraded mode, and repeated warnings do not
  stack into an unreadable banner.

## Requirements *(mandatory)*

### Functional Requirements

**Fault recovery (User Story 1)**

- **FR-001**: Every access to the app's shared site list MUST continue to function
  after a prior fault occurred while that list was being read or written.
- **FR-002**: Every access to the app's registry of running checks MUST continue to
  function after a prior fault occurred while that registry was being modified.
- **FR-003**: Every access to the app's startup warning MUST continue to function
  after a prior fault occurred while it was being read.
- **FR-004**: When the app recovers from a fault that occurred while the site list
  was being read or written, it MUST warn the user that their saved list may not
  reflect their most recent change, using the existing warning banner rather than a
  new mechanism.
- **FR-005**: Recovery from a fault in the registry of running checks MUST NOT warn
  the user.
- **FR-006**: Recovery MUST preserve whatever state the interrupted operation left
  behind; it MUST NOT reset the site list, discard entries, or restart checks as a
  side effect of recovering.
- **FR-007**: In the absence of any fault, every operation MUST behave exactly as it
  does today — no new warnings, no changed outcomes, no changed ordering.

**Refused add (User Story 2)**

- **FR-008**: The app MUST be able to distinguish a refused add from an add whose
  save to disk failed.
- **FR-009**: When an add is refused, the app MUST NOT add a row to the window and
  MUST NOT begin checking that site.
- **FR-010**: When an add is refused, the user MUST be told the site was not added,
  in terms that do not imply the change is being held un-saved.
- **FR-011**: When an add's save to disk fails, today's behaviour MUST be preserved:
  the row appears, checking begins, and the warning banner reports the save failure.
- **FR-012**: A refused add MUST leave both the in-memory list and the saved file
  exactly as they were.

**Atomic edit (User Story 3)**

- **FR-013**: Applying an edit MUST read the site's current state and write the
  resulting entry as one indivisible step, so no other change can be applied between
  the read and the write.
- **FR-014**: The rules that decide an edited entry's contents MUST be unchanged: an
  unchanged address preserves the learned request method, a changed address discards
  it, and an unknown site is reported as no longer existing.

**Scope and compatibility (all stories)**

- **FR-015**: This feature MUST NOT change the shape, field names, or location of the
  user's saved list.
- **FR-016**: This feature MUST NOT add any user-facing capability. The only
  user-visible changes permitted are the warning in FR-004 and the refusal message in
  FR-010.
- **FR-017**: Each of the three behaviours MUST be pinned by at least one automated
  test that fails against the current code and passes after the change.
- **FR-018**: The fourth §1 item — a link at the saved list's path being replaced
  rather than followed — MUST NOT be changed. It is recorded as expected behaviour
  and is inherent to the safe-save guarantee shipped in `003-durability`; undoing it
  would reopen the window that feature exists to close.

### Key Entities

- **Site**: One entry in the user's list — its identity, address, optional label,
  check interval, and the learned request method. Crosses between the core and the
  window unchanged by this feature.
- **Site list**: The user's whole list, held in memory and mirrored to disk. Shared
  by the window's actions and the background checks; the subject of User Stories 1
  and 3.
- **Check registry**: Which sites currently have a running recurring check. Ephemeral
  by design, rebuilt at every launch; the subject of FR-002 and FR-005.
- **Warning banner**: The existing one-line, non-fatal message the app already shows
  for a list that could not be loaded or saved. Reused, not replaced, by FR-004 and
  FR-010.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: After a single internal fault anywhere in the app's shared state, 100%
  of ordinary user actions — list, add, edit, delete, and starting or stopping checks
  — still complete successfully. Zero require a relaunch.
- **SC-002**: All ten of the app's shared-state accesses recover from a prior fault
  rather than failing; none is left able to disable the app.
- **SC-003**: Every row the window shows corresponds to an entry actually held in the
  user's list. A refused add produces zero rows and zero running checks.
- **SC-004**: When two edits to one site overlap, the saved entry reflects the later
  edit applied to the earlier one's result in 100% of cases; the learned request
  method is discarded only when the address actually changed.
- **SC-005**: Every message shown to the user states what actually happened —
  specifically, a refused add is never described as an unsaved change.
- **SC-006**: The three behaviours are covered by new automated tests, each verified
  to fail against the code as it stands today before being kept.
- **SC-007**: The project's existing merge bar is met with nothing disabled or
  skipped: the backend and frontend suites both green, the linter clean at its
  strictest setting, and no Critical or Important review finding open.
- **SC-008**: With no fault present, behaviour is indistinguishable from today —
  every existing automated test passes unmodified.

## Assumptions

- **Recovery means continue, not reset.** For a single-user desktop tool
  (Constitution I), carrying on from a possibly-inconsistent list and telling the user
  is better than discarding their list or refusing to run. Reconstructing the intended
  state is not possible and is not attempted.
- **The warning reuses the existing banner.** The app already surfaces non-fatal
  problems this way for load and write failures, and Constitution II establishes
  "warn, don't fail" as the house pattern. No new notification surface is introduced.
- **All three items are in scope.** "Section 1" is read as the whole section. Two of
  the three are unreachable from the shipped window today (User Story 2's branch,
  because every add mints a fresh identity; User Story 3's race, because the submit
  guard closed it). They are fixed anyway, because the point is that the core stops
  relying on the window to avoid states it permits.
- **The fourth item is out of scope by its own instruction.** The roadmap marks it
  "no action expected"; FR-018 pins that.
- **No new dependency, no new stored data.** The work is confined to how existing
  operations coordinate and report; nothing new is written to the user's file
  (Constitution II, V).
- **`002-robustness`'s and `003-durability`'s guarantees are preserved.** The submit
  and delete guards, the interval ceiling, and the stage-then-publish save all stay as
  they are; this feature must not weaken any of them.
- **Fault injection is achievable in tests.** Pinning User Story 1 requires a test
  that can leave shared state in the faulted condition. If that proves impractical for
  a given access, the fallback is a test asserting the recovering behaviour directly
  rather than dropping the coverage.
