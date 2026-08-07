# Changelog

All notable changes to Site Checker are recorded here.

## Concurrency & robustness hardening — verification round 2 — 2026-08-06

**No code change.** Nothing under `src-tauri/src/` or `src/` differs from what
shipped in the entry below; `git diff 8feb989..HEAD` over both trees is empty.
What this entry records is evidence, and one thing that evidence closed.

Spec: [`specs/20260806-120353-concurrency-hardening/spec.md`](specs/20260806-120353-concurrency-hardening/spec.md) ·
Review: [`reviews/review-20260806-175302.md`](specs/20260806-120353-concurrency-hardening/reviews/review-20260806-175302.md) ·
QA: [`qa/qa-20260806-181800.md`](specs/20260806-120353-concurrency-hardening/qa/qa-20260806-181800.md)

### Verified

- **The shell layer was finally exercised against a running window.** `commands.rs`
  has no automated coverage by design (Constitution IV, research R7), and the manual
  click-through standing in for it had only ever been half-run — a *refused* add, and
  nothing else. This round drove the real window through the macOS accessibility API
  with synthetic mouse clicks and performed a successful add, an edit, and a delete,
  hashing and parsing `sites.json` after each. The edit kept its list index; the
  delete did not come back after a relaunch. That closes the verification gap review
  finding R002 was tracking.
- **FR-011 and the warning channel are now proven at runtime.** With the app running,
  the store directory was made unwritable and a valid site added: the banner rendered
  *"Could not write sites.json: Permission denied (os error 13)"*, the row appeared
  anyway, its check ran, and `sites.json` on disk stayed byte-identical. Because
  `lock.rs:103` is the crate's only `store-warning` emit and FR-004's warning goes
  through the same private `warn()` helper, this is also the first runtime proof of
  the mechanism FR-004 reuses.
- **Both verdicts were re-derived, not inherited.** All three merge gates were re-run
  from a clean checkout — 55/0/0 Rust, 30/0 frontend unmodified, clippy clean at
  `--all-targets -- -D warnings`. The lock inventory (nine sites, all recovering),
  FR-015/FR-016 by file list, and FR-018 by removal audit were each re-checked from
  source.

### Known gaps

- The poison → `warn()` trigger still has no runtime assertion (QA TC-005, PARTIAL).
  No sequence of clicks can poison a mutex; closing it needs the `tauri` `test`
  feature and a mock-app harness, declined at research time.
- Review findings R001 (the source-text lock guard under-covers), R003, and R004
  (`load` accepts duplicate ids) remain open and are still not in `docs/ROADMAP.md`.
  That miss is itself tracked as R005.

## Concurrency & robustness hardening — 2026-08-06

The three actionable items from roadmap section 1, all in the Rust core. The
headline is that a panic inside the app no longer bricks it: today one fault
while the site list is locked poisons that lock and cascades a panic into every
later command, so the window keeps ticking status updates while refusing to add,
edit, delete, or even list anything until relaunch. As with the feature before
it, no frontend file is touched, no dependency is added, and nothing about the
on-disk shape or the IPC contract changes.

Spec: [`specs/20260806-120353-concurrency-hardening/spec.md`](specs/20260806-120353-concurrency-hardening/spec.md) ·
Plan: [`specs/20260806-120353-concurrency-hardening/plan.md`](specs/20260806-120353-concurrency-hardening/plan.md) ·
Tasks: [`specs/20260806-120353-concurrency-hardening/tasks.md`](specs/20260806-120353-concurrency-hardening/tasks.md) ·
Research: [`research.md`](specs/20260806-120353-concurrency-hardening/research.md) ·
Source: section 1 of the roadmap. None of the three was reachable from the
shipped window — the point is that the core stops relying on the window to avoid
states it permits.

### Fixed

- **One internal panic no longer disables the whole app.** Ten
  `Mutex::lock().unwrap()` call sites meant a panic inside any critical section
  poisoned that lock, and every later command panicked on contact with it. The
  fix is not ten call-site edits: `SharedStore` now wraps `Arc<Mutex<Store>>`
  with no accessor for the raw mutex, so a store lock *cannot* be taken
  un-recovered. Recovery is `PoisonError::into_inner()` — which preserves
  whatever the interrupted operation left behind rather than resetting the list
  — plus `Mutex::clear_poison()`, which makes the poison one-shot so the user
  gets one banner per fault instead of one per subsequent action. The two
  task-registry locks and the startup-warning lock call the same `lock::recover`
  helper directly and discard its flag: recovering them is required, warning
  about them is not, because which checks are running is ephemeral by design and
  rebuilt every launch.
- **A refused add no longer leaves a ghost row.** `Store::add` gained a
  duplicate-id refusal in the previous feature, but `add_site` funnelled every
  `Store::add` error into `warn_on_write_failure` and returned `Ok` regardless.
  A refusal therefore surfaced as "could not be saved" — a message that promises
  the change is still there, just un-persisted — while the window added the row
  and a timer started checking a site that was in no list, until it vanished at
  the next launch. A two-variant `AddError` lets the shell tell the two apart:
  a refusal returns `Err` with no row, no timer, and no banner, while a genuine
  write failure keeps today's behaviour exactly.
- **Two overlapping edits to one site can no longer discard each other.**
  `update_site` took the store lock twice — once to read `method_override`, once
  to write — so a second edit that began before the first finished decided from
  the same stale snapshot and silently overwrote it. The concrete loss was the
  app's memory of which request method a site needs, learned at the cost of an
  extra failed request against the user's site. The read-decide-write now lives
  inside `Store::replace`, where a single `&mut self` borrow makes the
  interleaving impossible by construction rather than by call-site discipline.

### Technical Notes

- One module added, `src-tauri/src/lock.rs`, holding `recover` as a Tauri-free
  generic function so poison recovery is unit-testable without a `State`.
- The lock-site count goes 10 → 9: collapsing `update_site`'s two acquisitions
  into one is what removes the tenth.
- No dependency added, runtime or dev. Poison recovery is `std::sync` only;
  `tauri`'s `test` feature was considered and declined.
- `AddError`, `Replaced`, and `SharedStore` are internal Rust types that never
  cross the IPC boundary — the commands map them back to the existing
  `Result<Site, String>` and `Result<(), String>` shapes, so the frontend
  contract is byte-identical and `src/` is untouched.
- Only two user-visible changes are permitted by the spec and only two were
  made: the poison-recovery warning, which reuses the existing banner rather
  than a new mechanism, and the reworded refusal message.
- Gate at merge: 55 Rust tests passing (up from 42), 30 frontend tests
  unchanged, `cargo clippy -- -D warnings` clean.

## Durability & data integrity — 2026-08-06

The three items from roadmap section 1, all in the Rust core. The headline is
that saving `sites.json` is now all-or-nothing: a crash mid-save can no longer
truncate the file and cost the user their list. No frontend file is touched, no
dependency is added, and nothing about the on-disk shape or the IPC contract
changes.

Spec: [`specs/003-durability/spec.md`](specs/003-durability/spec.md) ·
Plan: [`specs/003-durability/plan.md`](specs/003-durability/plan.md) ·
Tasks: [`specs/003-durability/tasks.md`](specs/003-durability/tasks.md) ·
Source: section 1 of the roadmap. Unlike the two features before it, this one
ran the full specify → plan → tasks cycle, so the mechanism decisions are
recorded in [`research.md`](specs/003-durability/research.md) rather than left
to implementation.

### Fixed

- **A crash mid-save can no longer truncate `sites.json`.** `Store::save` called
  `std::fs::write` directly, which empties the file and then refills it — a
  window in which a panic, a kill, or a dev-server restart left a truncated
  file. The next launch parsed that as corrupt, showed the banner, and started
  empty: graceful, but the last edit was gone. The write is now staged to a
  sibling `sites.json.tmp`, flushed with `sync_all`, and published with
  `std::fs::rename`, which is atomic within a filesystem — a reader sees either
  the complete old file or the complete new one, never a mixture. Because `add`,
  `update`, and `delete` all funnel through the one private `save`, this covers
  every mutation and no caller moved (US1).
  - The staging name is *fixed* rather than randomized, so repeated interrupted
    saves reuse the one artifact instead of leaving an orphan per crash, and the
    next successful save reclaims it. `load` opens only the path it was handed,
    so an orphan is never mistaken for the site list.
  - The honest limit, recorded in the code: this defends against the *process*
    dying, because the kernel completes the rename whether or not we survive it.
    It is not a power-loss guarantee — macOS `fsync` does not force the drive's
    own write cache the way `F_FULLFSYNC` does, and the parent directory is
    deliberately not synced.
- **`Store::add` refuses an id it already holds.** It pushed unconditionally, so
  two sites under one id would have made `get`/`update` hit the first while
  `delete` removed both. The check runs *before* the push and before the save, so
  a refusal leaves the in-memory list and the file agreeing. Unreachable from the
  shipped app — `add_site` mints a fresh v4 UUID per site — and added so the
  invariant lives at the layer that owns it rather than being a property of one
  caller's id generator (US3).

### Changed

- **A typed-in scheme is now stored lowercase.** `HTTPS://example.com` persisted
  verbatim, because `normalize_url` returns the user's own text rather than
  `url::Url`'s serialization — deliberately, since that is what keeps
  `example.com` yielding `https://example.com` and not `https://example.com/`.
  `has_leading_scheme` became `leading_scheme_end`, returning the scheme's byte
  index instead of a bool, so exactly the scheme slice is lowercased and the rest
  of the input is passed through untouched: hosts, paths, and query values keep
  their case, and a `HTTP://` inside a query string is not a leading scheme and is
  left alone (US2).
  - There is no migration. `load` does not call `normalize_url`, so a site already
    stored as `HTTPS://…` keeps that value until the user next edits it. On that
    edit it counts as a URL change under the existing rule, so the row drops to
    Pending and `method_override` is cleared — one extra request to re-learn HEAD
    support, once, for that site.
- **A symlink at the `sites.json` path is now replaced rather than followed.** A
  plain write followed the link and wrote through to its target; an atomic replace
  cannot. Nothing is destroyed — the old target keeps every byte it held — but the
  indirection is gone. Inherent to the fix, and the app never creates such a link.

### Added

- Seven `store.rs` tests covering the write path end to end: an interrupted save
  leaving the previous list loadable, the staged copy holding the new contents
  beside the live file, a successful save leaving nothing behind, repeated staging
  never accumulating more than one artifact, a failed staging preserving the
  previous file, a failed publication leaving exactly one orphan, and a stale
  artifact from a crashed run being inert to both `load` and the next save. The
  staging step is split out from `save` specifically so a test can stop a save at
  the instant the guarantee is about, rather than racing a killed subprocess.
- Two `store.rs` tests for the duplicate-id refusal, both asserting on a *reload*
  rather than the in-memory list, which is what proves the refusal preceded any
  write.
- Four `model.rs` tests for the scheme table, including the guard that stops the
  fix from being a lazy whole-string lowercase.
- Rust tests 29 → 42. Frontend stays at 30 — this feature touches no frontend file.

## Robustness — 2026-08-05

Five small correctness wins from the v1 review's Minor findings. One was a real
(if rare) bug; the rest close windows that were reachable but harmless, or
latent. No Rust source is touched, no dependency is added, and nothing about
`sites.json` or the IPC contract changes.

Tasks: [`specs/002-robustness/tasks.md`](specs/002-robustness/tasks.md) ·
Source: section 1 of the roadmap. There is no `spec.md` or `plan.md` — the
roadmap section named the file, the function, and the symptom for each item, so
it served as the spec directly.

### Fixed

- **A status event arriving during startup is no longer dropped.** `src/main.ts`
  registered `onSiteStatus` / `onStoreWarning` only after `await mountAutostart()`
  and `await getWarning()`. Tauri events have no replay, so anything emitted in
  that window was gone. Both registrations now run before every other `await` in
  `main()`. This is the one item the v1 review called a real bug (US1).
- **A row returns to Pending the moment its URL changes.** `upsertSite` updated
  `sites` but never `statuses`, so editing a good URL to a bad one kept showing
  a green dot until the next check landed — the UI claiming a confirmation it
  did not have. The stale status is now dropped on a URL change, and only on a
  URL change: label-only and interval-only edits keep the dot, because that
  result is still about that URL (US2).
- **Double-click can no longer save or delete twice.** The submit handler and
  the per-row Delete in `src/form.ts` had no in-flight guard. Both now disable
  their button around the awaited call and return early if it is already
  disabled. A failed save re-enables in a `finally`, so a rejection stays
  retryable (US3).

### Changed

- **The interval field is bounded at both ends.** `index.html`'s `#site-interval`
  gains `max="86400"` and `src/form.ts`'s clamp gains a matching ceiling, so a
  pasted 21-digit number is clamped at the source instead of failing `u64`
  deserialization at the IPC boundary. The floor behaviour is unchanged. 86400
  is a product guardrail chosen here, not a protocol limit — the backend still
  enforces only `MIN_INTERVAL_SECS` (US4).
- **A missing `#autostart` element degrades to a banner instead of a dead page.**
  `mountAutostart`'s `querySelector(...)!` meant that if the element ever went
  missing, the `catch` block's own `checkbox.disabled = true` threw a second
  time and aborted the rest of `main()`. It now early-returns with a banner,
  leaving `catch` operating on a checkbox known to exist. Latent only — the
  element is static in `index.html` (US5).

### Added

- `src/form.test.ts` — a DOM unit test over `form.ts`, following
  `render.test.ts`'s local-fixture style with `./api` stubbed by deferred
  promises so the in-flight window is inspectable. Covers the submit and delete
  guards, the retryable failure path, Add-vs-Edit dispatch, and the clamp table
  (floor, in-range, ceiling, empty, non-numeric).
- `src/main.test.ts` — coverage for the three `main.ts` stories, which had none.
  Importing `main.ts` *is* running startup (it calls `main()` at module load),
  so the test stubs `./api` and mounts a fixture DOM first. US1 is pinned by
  mock `invocationCallOrder` — that both listeners register before any startup
  IPC call, which is the ordering property, not merely that they register. US2
  is pinned in all four directions: URL change drops the status; label-only,
  interval-only, and first-add do not. US5 is pinned by omitting `#autostart`
  from the fixture, so every other assertion in the file doubles as proof that
  a missing control no longer aborts `main()`.
- Frontend tests: 12 → 30.

### Technical Notes

- The guards check `disabled` explicitly rather than relying on the browser
  refusing to click a disabled button: this handler also runs for a programmatic
  submit, which never consults `disabled`.
- The delete guard deliberately re-enables only on the failure path. On success
  the row — and its button — is removed by `onDeleted`.
- `upsertSite` compares two backend-normalized URLs. `addSite`/`updateSite`
  resolve to the saved `Site` after `normalize_url`, so a cosmetic difference
  the backend already collapsed cannot false-positive into a spurious reset.
- 86400 now lives in three places that must stay in sync: `form.ts`'s constant,
  `index.html`'s `max`, and the ceiling case in `form.test.ts`.
- Every assertion added here was confirmed to fail against the pre-fix code
  before being kept — the five guard tests, the clamp ceiling, the URL-change
  reset, the missing-`#autostart` banner, and the listener ordering.
- Quality gates: `cargo test` (29 passing), `pnpm test` (30 passing),
  `cargo clippy -- -D warnings` (clean), `pnpm build` (clean).

## Scaffold Cleanup — 2026-08-05

Removes the residue `create-tauri-app` left behind and sharpens two imprecise
strings. No new code, no new dependencies, and no behavior change beyond the
wording of one warning banner.

Spec: [`specs/001-scaffold-cleanup/spec.md`](specs/001-scaffold-cleanup/spec.md) ·
Plan: [`specs/001-scaffold-cleanup/plan.md`](specs/001-scaffold-cleanup/plan.md) ·
Tasks: [`specs/001-scaffold-cleanup/tasks.md`](specs/001-scaffold-cleanup/tasks.md)

### Removed

- The unregistered `opener` plugin, in all three places it was declared:
  `"opener:default"` from `src-tauri/capabilities/default.json`,
  `@tauri-apps/plugin-opener` from `package.json`, and `tauri-plugin-opener`
  from `src-tauri/Cargo.toml`. `src-tauri/src/lib.rs` only ever initialized
  `tauri_plugin_autostart`, so the plugin was granted permission surface and
  compiled into the shipped binary without being used. Both lockfiles were
  regenerated (US1).
- The three orphaned scaffold SVGs — `src/assets/tauri.svg`,
  `src/assets/typescript.svg`, and `src/assets/vite.svg`. No source file or
  `index.html` referenced them, and Vite already excluded them from the bundle,
  so `dist/` is byte-for-byte unaffected. `src/assets/` is now empty and gone (US2).

### Changed

- The package and crate now identify themselves as Site Checker instead of the
  scaffold: `package.json` `name` is `site-checker`; `src-tauri/Cargo.toml` carries
  `name = "site-checker"`, a real one-line `description`, and
  `authors = ["Clint Parker <me@clintparker.com>"]` in place of `authors = ["you"]` (US3).
- `[lib] name` renamed `tauri_app_lib` → `site_checker_lib`, with the matching
  `src-tauri/src/main.rs` call site updated in the same change — the one
  build-breaking ripple in this feature (US3).
- The corrupt-file warning in `src-tauri/src/store.rs::load` now names its actual
  cause (the file is not valid JSON) instead of reading like the neighbouring
  I/O-error message. It still reassures the user the file was left on disk.
  This is the only user-visible change in the feature (US4).
- The `has_leading_scheme` doc comment in `src-tauri/src/model.rs` now states the
  character-class rule its body applies — the text before `://` must be entirely
  ASCII alphanumeric or one of `+`, `-`, `.` (US4).

### Technical Notes

- The bundle is unchanged where it counts: `src-tauri/tauri.conf.json` pins
  `productName: "Site Checker"` and `identifier: com.clintparker.site-checker`
  and never referenced the crate name, so `pnpm tauri build --bundles app` still
  emits `Site Checker.app` at the same 15 MB.
- No persisted or IPC field was renamed. `sites.json` keeps its shape, its path,
  and its load semantics — only the warning text changed.
- Sequencing was constrained by one cross-story conflict: US1 and US3 both edit
  `package.json` and `src-tauri/Cargo.toml`, so they were serialized. US2 and US4
  touch disjoint files.
- Quality gates after every story, not just at the end: `cargo test` (29 passing),
  `pnpm test` (12 passing), and `cargo clippy -- -D warnings` (clean).
