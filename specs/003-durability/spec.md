# Feature Specification: Durability & Data Integrity

**Feature Branch**: `20260806-102818-durability-and-data`

**Created**: 2026-08-06

**Status**: Draft

**Input**: Section 1 ("Durability & data integrity") of `docs/ROADMAP.md`

## Overview

`sites.json` is the only file this app owns, and the constitution calls it sacred. Today
every save rewrites it in place with a plain whole-file write. If the app dies partway
through — crash, force-quit, power loss — the file on disk is left half-written. The next
launch handles that gracefully (empty list plus a visible banner, bad file untouched so it
can be hand-repaired), so this is not silent data loss. But the user's last edit is gone
and their whole list is missing until they go rescue the file by hand.

This feature closes that window and sweeps up two smaller integrity items that live in the
same neighbourhood: a URL scheme that persists in whatever case the user typed it, and an
`add` path that would happily store two sites under one id if it were ever asked to.

Nothing here changes what the app does. It changes what survives when the app stops
unexpectedly.

## Clarifications

Resolved by direct inspection of the tree on 2026-08-06:

- **There is exactly one write path.** `Store::add`, `Store::update`, and `Store::delete`
  all mutate the in-memory `Vec<Site>` and then call the private `Store::save`. Making
  `save` crash-safe therefore covers all three mutations; no caller needs to change.
- **The graceful-corruption behaviour already exists and must be preserved.** `load()`
  treats a parse failure as an empty list plus a warning and deliberately leaves the bad
  file on disk (`corrupt_file_yields_an_empty_list_a_warning_and_is_left_on_disk` pins
  this). Atomic saves do not replace that safety net — they make it far less likely to be
  needed. Both behaviours must hold after this feature.
- **A failed save is already non-fatal to the session.** `commands.rs::warn_on_write_failure`
  emits a `store-warning` banner and the in-memory change stands. That contract is
  unchanged; atomicity only adds the guarantee that the *previous* file is still intact
  when a save fails.
- **`normalize_url` returns the user's own text, not the reserialized URL.** It builds
  `candidate` (trimmed, scheme-prefixed) and returns that, so `example.com` yields
  `https://example.com` rather than `https://example.com/`. The scheme check passes for
  `HTTPS://example.com` because `url::Url::parse` lowercases the scheme it reports, while
  `candidate` keeps the uppercase — that is the whole bug. The fix must rebuild the
  returned string with a lowercased scheme; switching to the parsed URL's own
  serialization would re-introduce the trailing slash the current code exists to avoid.
- **Duplicate ids are unreachable from the app today.** The only caller of `Store::add`
  is `commands.rs::add_site`, which mints a fresh v4 UUID for every site. The guard is
  belt-and-braces against a future non-UI caller, and its value is in what it refuses to
  do, not in a bug it fixes.
- **Existing files are not rewritten wholesale.** A `sites.json` already holding
  `HTTPS://example.com` keeps that value until the user next edits that site. Loading does
  not normalize, and this feature does not add a migration.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The list survives an interrupted save (Priority: P1)

A user adds, edits, or deletes a site, and the app is killed at the worst possible moment —
mid-write. On the next launch their list is intact: either exactly as it was before that
last edit, or including it. It is never a half-written file, and they are never met with an
empty table and a corrupt-file banner because of it.

**Why this priority**: This is the roadmap's own "highest-value item". It is the only item
here that protects real user data, and the other two are cosmetic or hypothetical beside
it. It is also independently shippable — nothing else in this feature depends on it.

**Independent Test**: Drive the store through a save that is interrupted after the new
contents are staged but before they replace the live file, then load from that same path.
The load returns the pre-save list with no warning. Repeat with the interruption after the
replacement: the load returns the post-save list. Existing store tests pass unmodified.

**Acceptance Scenarios**:

1. **Given** a `sites.json` holding two sites, **When** a third is added and the process is
   interrupted before the new contents replace the old file, **Then** the next load returns
   the original two sites and no warning banner.
2. **Given** the same starting state, **When** the save completes, **Then** the next load
   returns all three sites and the file contains no partial or duplicated content.
3. **Given** a save fails outright (for example the volume is full or read-only), **When**
   the failure is reported, **Then** the previous `sites.json` is still complete and
   loadable, the in-memory list still reflects the user's edit, and the existing
   write-failure banner is shown.
4. **Given** a `sites.json` that is already corrupt from before this feature, **When** the
   app loads it, **Then** behaviour is unchanged — empty list, warning banner, and the bad
   file left on disk for hand-recovery.
5. **Given** `sites.json`'s parent directory does not exist, **When** a save runs, **Then**
   the directory is created and the save succeeds, as it does today.

---

### User Story 2 - A typed scheme is stored in a consistent case (Priority: P2)

A user pastes `HTTPS://example.com` into the URL field. The site is stored — and shown back
to them — as `https://example.com`, matching every other entry in their list rather than
standing out in shouty capitals.

**Why this priority**: Cosmetic. The URL works either way and checks succeed either way.
It is second because it is the only item a user can actually see, and it is independent of
both other stories.

**Independent Test**: Call the URL normalizer with mixed-case schemes and assert the
returned string's scheme is lowercase while the rest of the input is untouched. No
persistence or app launch required.

**Acceptance Scenarios**:

1. **Given** input `HTTPS://example.com`, **When** it is normalized, **Then** the result is
   `https://example.com`.
2. **Given** input `HtTp://example.com/health`, **When** it is normalized, **Then** the
   result is `http://example.com/health` — path and case after the scheme preserved.
3. **Given** input `example.com` with no scheme, **When** it is normalized, **Then** the
   result is still `https://example.com` with no trailing slash added.
4. **Given** input `example.com?next=HTTP://x.dev`, **When** it is normalized, **Then** only
   the leading scheme is affected — the query string is returned verbatim.
5. **Given** input `FTP://example.com`, **When** it is normalized, **Then** it is still
   rejected as an unsupported scheme.
6. **Given** a stored site whose URL has an uppercase scheme from before this change,
   **When** the app loads, **Then** it is left as-is and continues to be checked normally;
   it is normalized only if the user edits that site.

---

### User Story 3 - The store refuses to hold two sites under one id (Priority: P3)

A caller that tries to add a site whose id already exists is refused. The stored list keeps
the site it already had, and nothing is written.

**Why this priority**: Last, because it cannot be triggered by the shipped app — ids are v4
UUIDs minted per add. Its value is that the store stops depending on its only caller
behaving, which matters the moment a second caller exists.

**Independent Test**: Add a site, then add another with the same id. The second call is
refused, the list still holds one site, and the file on disk is unchanged. No app launch
required.

**Acceptance Scenarios**:

1. **Given** a store holding a site with id `abc`, **When** a different site with id `abc`
   is added, **Then** the call reports a failure, the list still holds exactly one site,
   and its fields are the original ones.
2. **Given** that refused add, **When** the store is reloaded from disk, **Then** the file
   matches the pre-add state — the refusal wrote nothing.
3. **Given** a store holding a site with id `abc`, **When** a site with id `xyz` is added,
   **Then** it succeeds exactly as it does today.

---

### Edge Cases

- **Interruption between staging and replacement.** The staged copy is orphaned and the
  live file is untouched. The next load must return the previous list with no warning, and
  must not see the orphan as part of the user's data.
- **Orphaned staging artifacts accumulating.** Repeated crashes must not litter the app
  support directory. At most one orphan may exist at a time, and a later successful save
  must consume or replace it rather than adding another.
- **Staging on a different filesystem.** The staged copy must sit alongside `sites.json` in
  the same directory, or the replacement stops being atomic (and may fail outright) across
  a volume boundary.
- **Volume full or read-only.** The save fails, the previous file survives complete, and
  the user gets the existing write-failure banner. The user's edit is not silently reverted
  in the UI.
- **A directory where `sites.json` should be.** The save fails and reports it; nothing is
  destroyed. Unchanged from today's behaviour.
- **A symlink where `sites.json` should be.** Behaviour *changes* here, and the change is
  accepted rather than worked around. A plain write followed the symlink and wrote through to
  its target; the replacement step instead **replaces the symlink itself** with a regular
  file. Nothing is destroyed — the old target keeps every byte it held — but the indirection
  is gone. This is inherent to an atomic replace and cannot be avoided without reopening the
  truncation window this feature exists to close. The app never creates such a symlink; only
  a user who hand-linked the file would meet it, and their data survives either way.
  (Verified by experiment, not recalled — research R5.)
- **`load()` meeting a truncated file written before this feature.** Unchanged behaviour —
  empty list, warning, file preserved.
- **Scheme-only case difference on edit.** Editing a site from `HTTPS://example.com` to
  `https://example.com` changes the stored URL. The existing rule that a changed URL clears
  `method_override` applies as written; re-learning HEAD support costs one extra request.
- **Uppercase host.** `https://EXAMPLE.com` keeps its host case. Only the scheme is
  normalized in this feature.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Persisting the site list MUST be atomic from the reader's point of view: an
  interruption at any moment leaves `sites.json` holding either its complete previous
  contents or the complete new contents, never a partial or interleaved mixture.
- **FR-002**: All three mutations — add, update, delete — MUST go through that single
  crash-safe write path.
- **FR-003**: Any intermediate artifact used to stage a write MUST live in the same
  directory as `sites.json`, MUST NOT be readable as the site list, and MUST NOT accumulate
  across repeated interrupted saves.
- **FR-004**: A failed save MUST leave the previous `sites.json` complete and loadable, keep
  the user's change in memory, and surface the existing write-failure warning to the UI.
- **FR-005**: Existing load behaviour MUST be preserved exactly: a missing file is an empty
  list with no warning, an unreadable or unparseable file is an empty list plus a warning,
  and a bad file is left untouched on disk.
- **FR-006**: A save MUST still create `sites.json`'s parent directory when it is absent.
- **FR-007**: URL normalization MUST return a lowercase scheme regardless of the case the
  user typed.
- **FR-008**: URL normalization MUST otherwise return the user's input unchanged — no
  trailing slash added, host and path case preserved, query string verbatim — and MUST keep
  rejecting empty input, unparseable input, non-`http`/`https` schemes, and missing hosts.
- **FR-009**: Adding a site whose id is already present MUST be refused, leaving both the
  in-memory list and the file on disk unchanged.
- **FR-010**: `sites.json` MUST keep its documented shape — a bare JSON array of sites with
  snake_case keys — and no persisted or event field name may change.
- **FR-011**: No change in this feature may alter check scheduling, request behaviour, or
  the app's user interface.

### Key Entities

- **Site**: unchanged in shape and meaning. Its `id` gains an explicit uniqueness guarantee
  within the store, and its `url` gains a lowercase-scheme guarantee at the point it is
  created or edited.
- **Site list file**: the single JSON array the app owns, at
  `~/Library/Application Support/com.clintparker.site-checker/sites.json`. Its location,
  format, and load semantics are unchanged; only the manner in which it is replaced changes.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Interrupting a save at any point and relaunching yields either the complete
  pre-edit list or the complete post-edit list — in 100% of trials, never an empty list
  with a corrupt-file banner attributable to that interruption.
- **SC-002**: After a save that fails for any reason, the previously saved list is still
  fully recoverable without hand-editing any file.
- **SC-003**: A user who types a URL with an uppercase scheme sees it listed in the same
  lowercase form as every other entry, with the rest of what they typed unchanged.
- **SC-004**: Every URL the app accepted before this change is still accepted and produces
  the same stored value, except for the case of the scheme.
- **SC-005**: Repeated interrupted saves leave at most one leftover artifact in the app's
  data directory — the count does not grow with the number of interruptions.
- **SC-006**: The store never holds two entries with the same id, and an attempt to create
  one changes neither the list nor the file.
- **SC-007**: All existing automated tests pass unmodified, except where a test's own
  assertion encodes the old behaviour being fixed; the project's three quality gates
  (`cargo test`, `pnpm test`, `cargo clippy -- -D warnings`) are green after each story, not
  just at the end.
- **SC-008**: `docs/ROADMAP.md` section 1 is emptied, with each of its three items marked
  done or explicitly re-deferred with a stated reason.

## Assumptions

- **A refused duplicate add reports a failure rather than replacing or ignoring.** Refusing
  preserves existing data, which is the point of this feature; the roadmap does not specify
  a behaviour. Since the shipped app cannot trigger it, the choice costs nothing today.
- **Only the scheme is case-normalized.** Hosts are case-insensitive too, but lowercasing
  them would rewrite the user's text more aggressively than the roadmap asked and than the
  current "return what they typed" design intends.
- **No migration of already-stored URLs.** Rewriting existing entries at load time would
  mean writing to the store on startup, which the app does not do today and which would
  itself be a durability risk. Uppercase schemes already on disk are normalized only when
  the user next edits that site.
- **Durability is scoped to process death, not media failure.** The guarantee is that an
  interrupted or crashed *process* cannot leave a partial file. Guaranteeing survival of a
  sudden power loss at the hardware level is a stronger claim and is not assumed here.
- **The single-user desktop assumption holds.** One process owns the file; this feature does
  not add cross-process locking.
- **The existing write-failure banner is the reporting channel.** No new UI, message, or
  event is introduced.

## Constitution Alignment

- **I. One Mac, One Person** — no scope widening. Nothing here adds a feature; it hardens
  what exists.
- **II. Results Are Ephemeral, Config Is Sacred** — this is that principle's most direct
  expression. The file's location, shape, and load semantics are untouched; the graceful
  corrupt-file path is explicitly preserved rather than replaced.
- **III. Be a Polite Client** — request behaviour is untouched. The one indirect effect is
  that editing a URL's scheme case clears `method_override`, costing at most one extra
  request for that site, under the existing rule.
- **IV. Testable Core, Thin Shell** — all three changes land in the pure, already-tested
  layer (`store.rs` against a temp dir, `model.rs` as plain functions). No logic moves into
  the Tauri shell.
- **V. The Rust/TS Contract Is snake_case, As-Is** — no serialized field name changes and no
  change to the on-disk array shape.
- **Quality Gates** — enforced per story, and the roadmap is drained explicitly rather than
  silently.

## Out of Scope

Sections 2–5 of `docs/ROADMAP.md`: mutex-poison recovery and the `update_site`
read-modify-write race (section 2), bundle size and the DMG build step (section 3), the
remaining test-coverage gaps in `check.rs` / `render.ts` / `main.ts` / `form.ts` and the
duplicated 86400 ceiling (section 4), and every deferred v2 feature (section 5).

Also out of scope: backups or version history for `sites.json`, an automatic repair or
migration pass over an existing store, cross-process file locking, and any change to how
check results are held or displayed.
