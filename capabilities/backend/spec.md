# Backend — Living Spec

> [DRAFT] Surface-first draft from existing code — every requirement is observed from the code surface unless tagged otherwise. Review before trusting.

## Purpose

Answers "is this thing up, and how long ago did we last confirm that?" for a personal list of sites, and keeps answering it unattended. It owns the site list and its durability, decides what counts as up, and schedules the checks. Without it the app has nothing to show and no memory between launches — and, done carelessly, it would hammer other people's servers on the user's behalf.

## Requirements

### Each site is checked on its own independent schedule

Every site SHALL be polled by its own recurring task, so that one site's cadence, failure, or edit has no effect on any other site's checks. Editing or removing a site MUST disturb only that site's schedule.

#### Scenario: one site's interval is changed
- **WHEN** a site's interval is edited
- **THEN** only that site's recurring check is restarted
- **AND** every other site keeps its existing cadence and offset

#### Scenario: a site is removed
- **WHEN** a site is deleted
- **THEN** its recurring check stops
- **AND** the remaining sites continue uninterrupted

#### Scenario: a site is restarted while already running
- **WHEN** a check is scheduled for a site that already has one running
- **THEN** the previous task is cancelled before the new one is tracked
- **AND** no untracked task is left running

### Checks are spread out so a shared interval does not stampede

Sites SHALL NOT all fire on the same instant merely because they share an interval. A bounded startup offset SHALL be applied once per site, and because each cycle waits a full interval after the check completes, that offset SHALL persist for the life of the task without further randomization.

#### Scenario: several sites share one interval
- **WHEN** the app starts with multiple sites on the same interval
- **THEN** their first checks are staggered rather than simultaneous
- **AND** the delay before the first result is still short

#### Scenario: the task keeps running
- **WHEN** a site has completed several cycles
- **THEN** its checks remain offset from other sites on the same interval

### The poll interval has a floor that anything scheduled must respect

A minimum interval SHALL be enforced as a guardrail against hammering an endpoint. Raising a too-small value to the floor is the only correction — a value at or above the floor is never altered. This is a property of *what gets scheduled*, not merely of what the UI submits: any interval that reaches the scheduler MUST already respect the floor, whatever its source.

> **Known deviation.** The floor is applied only when a site is added or updated through the command surface. Sites loaded from the saved file at startup are scheduled with whatever interval that file contains, so a hand-edited or corrupted `interval_secs: 0` polls with no delay between requests. The requirement above states the intent; the load path does not yet meet it.

#### Scenario: a saved site carries an interval below the floor
- **WHEN** the stored site list contains an interval under the floor
- **THEN** it is raised to the floor before that site is scheduled

#### Scenario: a too-frequent interval is requested
- **WHEN** an interval below the floor is submitted
- **THEN** it is raised to the floor before being stored or scheduled

#### Scenario: a reasonable interval is submitted
- **WHEN** an interval at or above the floor is submitted
- **THEN** it is stored and scheduled unchanged

### "Up" means the site answered, not that the host is reachable

A check SHALL judge the application's response, treating any successful or redirect-class HTTP status as up and everything else as down. A redirect that is still a redirect at the end of the followed chain SHALL count as up rather than being reported as a failure.

#### Scenario: the site serves content
- **WHEN** the response carries a success or redirect status
- **THEN** the site is reported up with no failure reason

#### Scenario: the site answers with an error
- **WHEN** the response carries a client- or server-error status
- **THEN** the site is reported down
- **AND** the status is given as the reason

#### Scenario: the redirect chain resolves
- **WHEN** a URL redirects to a page that answers successfully
- **THEN** the site is reported up

### Checking stays cheap, but adapts to servers that refuse the cheap method

A check SHALL prefer the body-less request method, and when a server rejects that method specifically, SHALL retry once with the full request against the same URL and remember the server's preference so later checks skip the futile probe. What was learned about a server MUST be discarded when the URL changes, since it was learned about a different endpoint.

#### Scenario: a server rejects the body-less method
- **WHEN** a check is refused because the method is unsupported
- **THEN** it is retried once with the full method
- **AND** the preference is recorded so the next check goes straight there

#### Scenario: a known full-method-only site is checked again
- **WHEN** a site already recorded as needing the full method is checked
- **THEN** no probe of the refused method is attempted

#### Scenario: the URL is edited
- **WHEN** a site's URL is changed
- **THEN** what was learned about the previous URL's method support is forgotten

#### Scenario: the retry also fails
- **WHEN** the retried request answers with an error status
- **THEN** the site is reported down with that status as the reason

### Every failure explains itself in one short phrase

A down result SHALL carry a brief, human-readable reason suitable for a hover tooltip. Transport-level failures SHALL be collapsed into a category rather than surfaced as the underlying library's full error chain, which is far too long to read in place.

#### Scenario: the request times out
- **WHEN** a check exceeds the request timeout
- **THEN** the reason names the timeout in a few words

#### Scenario: the host cannot be reached
- **WHEN** the connection is refused or the host does not resolve
- **THEN** the reason states that plainly rather than quoting a nested error

### Checks reflect the live site, not a cache

The HTTP client SHALL be a polite, ordinary-looking browser client with a bounded timeout and a bounded redirect chain, and SHALL NOT cache responses. A cached response would make the check meaningless — it would report the last answer rather than the current one.

#### Scenario: the same URL is checked repeatedly
- **WHEN** consecutive checks run against one URL
- **THEN** each performs a real request rather than reusing a stored response

#### Scenario: a site sits behind a filtering proxy
- **WHEN** a check is made
- **THEN** the client presents itself as a common browser rather than an unknown tool [inferred] — the intent is to avoid being rate-limited or blocked, which the surface states but cannot demonstrate

### A damaged site list never prevents the app from starting

Loading the site list SHALL NOT fail. A missing file means there is nothing saved yet; an unreadable or unparseable one means an empty session plus a warning the user can see. A file that could not be parsed MUST be left exactly as it is on disk, so the user still has the chance to recover it by hand.

#### Scenario: no site list exists yet
- **WHEN** the app starts with no saved file
- **THEN** it starts with an empty list and says nothing

#### Scenario: the site list is corrupt
- **WHEN** the saved file cannot be parsed
- **THEN** the app starts with an empty list and reports a warning
- **AND** the unparseable file is left untouched on disk

### A save is published whole or not at all

Saving the site list SHALL NOT expose a partially written file at the path the app reads. The new contents SHALL be written and flushed to a staging copy first, and only a single publishing step SHALL make them visible, so a reader always sees either the complete previous list or the complete new one. A save that never reaches publication MUST leave the previous file exactly as it was.

> **Clarification.** The staging copy is a sibling of the real file, not a temp-directory file, because publication is only atomic within one filesystem. Its name is fixed rather than unique, which bounds the debris from repeated interrupted saves at one file rather than one per attempt.

> **Known limit.** This defends against the *process* dying — a panic, a kill, a dev-server restart — because the operating system completes the publication whether or not the app survives it. It is not a power-loss guarantee: the platform flush used here does not force the drive's own write cache, and the containing directory is deliberately not flushed.

#### Scenario: the app dies mid-save
- **WHEN** a save writes its staging copy and the process dies before publishing
- **THEN** the previously saved list is still loadable and complete
- **AND** the interrupted save does not make that list look corrupt

#### Scenario: a save succeeds
- **WHEN** a save completes
- **THEN** no staging copy is left behind

#### Scenario: saves are interrupted repeatedly
- **WHEN** several saves in a row are interrupted before publishing
- **THEN** at most one leftover staging copy exists, not one per attempt

#### Scenario: a leftover staging copy is present at startup
- **WHEN** the app loads with debris from a crashed run beside the site list
- **THEN** the leftover is ignored rather than read or reported as corruption
- **AND** the next save reclaims it

### A failed write never costs the user their edit

When persisting a change fails, the change SHALL remain in effect for the session and the user SHALL be told, rather than the edit being rolled back or the failure being swallowed. The app keeps working; only durability was lost.

This covers changes that *were* applied in memory. A change refused outright — see the next requirement — is not a failed write and MUST NOT be reported as one, because there is no edit left standing to warn about.

#### Scenario: the disk write fails while adding a site
- **WHEN** a site is added and the write fails
- **THEN** the site is still live and being checked this session
- **AND** a warning is surfaced to the user

#### Scenario: a learned method preference cannot be saved
- **WHEN** recording what was learned about a server fails
- **THEN** it still holds for the session and no warning is raised, because the next check rediscovers it

### An addition that clashes with an existing entry changes nothing at all

An attempt to add a site whose identity is already in the list SHALL be refused *before* anything is mutated, so the in-memory list and the saved file are left agreeing and byte-identical. A refusal MUST be distinguishable from a failed write, since the two owe the user opposite answers: a refusal means no row appears, no checks start, and no "could not be saved" banner is raised — the refusal message itself is the whole story. The refused id MUST be carried internally for a caller that can use it, but the user MUST NOT be shown an internal identity.

#### Scenario: the identity is already taken
- **WHEN** an add names an identity already in the list
- **THEN** it is refused with a message saying nothing was changed
- **AND** the existing entry is left exactly as it was
- **AND** no row appears and no checks begin for the refused site
- **AND** no warning banner is raised alongside the refusal

#### Scenario: nothing is written on refusal
- **WHEN** an add is refused
- **THEN** the saved file is byte-identical to before
- **AND** no save is attempted, so no staging copy appears

### An edit is decided from the list as it stands when it is applied

Applying an edit SHALL read the current entry, decide what carries forward from it, write the result, and save as one indivisible step. There MUST be no window between the read and the write in which another edit can land, so an edit can never resurrect a value from a picture of the list that a concurrent edit has already replaced. An edit naming a site that is no longer in the list SHALL be reported as such and MUST write nothing.

#### Scenario: two edits to one site overlap
- **WHEN** an edit completes while another edit of the same site is in flight
- **THEN** the later edit decides what carries forward from the earlier edit's result, not from a stale reading
- **AND** the later edit's own values win

#### Scenario: the site was deleted first
- **WHEN** an edit names a site that is no longer in the list
- **THEN** it is reported as gone
- **AND** nothing is written and no staging copy appears

#### Scenario: the save behind an edit fails
- **WHEN** an edit is applied and the save fails
- **THEN** the edit still stands in memory so the row is kept and only a warning is raised

### An internal fault leaves the app usable, and is reported once

A fault inside the app while it holds shared state MUST NOT make that state permanently unusable for the rest of the session. Later operations SHALL continue from the state the faulting operation left behind — including a half-applied change — rather than resetting or discarding a list the app cannot reconstruct. The fault SHALL be reported exactly once, through the same one-line banner the app already uses for non-fatal problems, and only for the site list: internal bookkeeping that is rebuilt at every launch names no consequence the user could act on and SHALL recover silently. With no fault, behaviour MUST be indistinguishable from an app that never had recovery at all.

#### Scenario: a fault interrupts a change to the site list
- **WHEN** the app faults partway through applying a change
- **THEN** the next operation still works, starting from what had already been applied
- **AND** the user is told once that their saved list may not reflect their most recent change

#### Scenario: work continues after a reported fault
- **WHEN** further operations run after a fault has been reported
- **THEN** no further warning is raised for that same long-past fault

#### Scenario: a fault touches ephemeral state only
- **WHEN** the fault involves which checks are running, or the startup warning slot
- **THEN** recovery is silent, because that state is rebuilt every launch and replaced on every scheduling call

#### Scenario: nothing has faulted
- **WHEN** the app runs normally
- **THEN** no recovery is reported and behaviour is unchanged

### Invalid input is rejected before anything is stored

A URL SHALL be validated before a site is created or updated, and SHALL be rejected with a message the user can act on rather than being stored and failing later. A missing scheme SHALL be filled in rather than treated as an error, and only web schemes SHALL be accepted, whatever case they were typed in. A scheme the user typed SHALL be stored lowercase, and that SHALL be the *only* rewriting done: the rest of the text stored MUST be the user's own, so a URL does not come back subtly rewritten. Hosts are case-insensitive too, but rewriting them is more of the user's text than was asked for, and paths and query values are case-*sensitive*.

> **Clarification.** There is no migration for URLs already saved. A site stored with an upper-case scheme keeps that text until the user next edits it, and on that edit it counts as a URL change — so the row returns to "no result yet" and what was learned about the server's method support is re-probed at the cost of one request. Surprising exactly once per affected site.

#### Scenario: a bare hostname is entered
- **WHEN** a URL is entered with no scheme
- **THEN** a secure scheme is added
- **AND** the rest of the text is preserved exactly as typed, with no added trailing path

#### Scenario: an upper-case scheme is entered
- **WHEN** a URL is entered with its scheme in any mixed or upper case
- **THEN** the scheme is stored lowercase
- **AND** the host, path, and query keep the exact case they were typed in

#### Scenario: another URL appears inside the query
- **WHEN** the first `://` in the text belongs to a query value rather than a leading scheme
- **THEN** a scheme is still added at the front
- **AND** the embedded URL's own case is left alone

#### Scenario: a non-web scheme is entered
- **WHEN** a URL names a scheme the app does not check
- **THEN** it is rejected and nothing is stored
- **AND** the rejection does not depend on how that scheme was capitalized

#### Scenario: the entry is empty or unparseable
- **WHEN** the URL is blank or cannot be parsed
- **THEN** it is rejected with a message naming the problem

### Launch-at-login is on by default, but turning it off sticks

Launching at login SHALL be enabled once, on first run only, and a later deliberate opt-out MUST survive restarts. First run MUST be distinguished from "the user turned this off", and that distinction MUST be recorded even if enabling failed — otherwise a failure would re-enable the setting on every subsequent launch.

#### Scenario: the very first launch
- **WHEN** the app runs for the first time
- **THEN** launch-at-login is enabled without the user asking

#### Scenario: the user opts out
- **WHEN** the user turns launch-at-login off and later restarts
- **THEN** it remains off

#### Scenario: the system refuses to enable it
- **WHEN** enabling fails on first run
- **THEN** the user is told, and the app does not retry on every future launch

### Closing the window ends the app

Closing the window SHALL terminate the process. A windowless process lingering in the background is explicitly not wanted for this app.

#### Scenario: the user closes the window
- **WHEN** the window is destroyed
- **THEN** the process exits rather than staying resident

### Reported state carries only what the UI can act on

The status pushed to the UI SHALL identify the site, its state, when the check completed, and a reason when there is one. Only states the backend can actually determine SHALL exist here — "no result yet" is the UI's own concern and MUST NOT be representable as a reported state.

#### Scenario: a check completes
- **WHEN** any check finishes
- **THEN** a status carrying the site, its state, and the completion time is pushed to the UI

#### Scenario: a site has never been checked this session
- **WHEN** no check has completed for a site
- **THEN** no status is pushed, and the UI decides how to present the absence

## Uncovered

_None — every file in the area was read._
