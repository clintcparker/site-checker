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

### A failed write never costs the user their edit

When persisting a change fails, the change SHALL remain in effect for the session and the user SHALL be told, rather than the edit being rolled back or the failure being swallowed. The app keeps working; only durability was lost.

#### Scenario: the disk write fails while adding a site
- **WHEN** a site is added and the write fails
- **THEN** the site is still live and being checked this session
- **AND** a warning is surfaced to the user

#### Scenario: a learned method preference cannot be saved
- **WHEN** recording what was learned about a server fails
- **THEN** it still holds for the session and no warning is raised, because the next check rediscovers it

### Invalid input is rejected before anything is stored

A URL SHALL be validated before a site is created or updated, and SHALL be rejected with a message the user can act on rather than being stored and failing later. A missing scheme SHALL be filled in rather than treated as an error, and only web schemes SHALL be accepted. The text stored MUST be the user's own, so a URL does not come back subtly rewritten.

#### Scenario: a bare hostname is entered
- **WHEN** a URL is entered with no scheme
- **THEN** a secure scheme is added
- **AND** the rest of the text is preserved exactly as typed, with no added trailing path

#### Scenario: a non-web scheme is entered
- **WHEN** a URL names a scheme the app does not check
- **THEN** it is rejected and nothing is stored

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
