---
description: "Task list for Durability & data integrity (ROADMAP section 1)"
---

# Tasks: Durability & data integrity

**Input**: [spec.md](./spec.md) — derived from section 1 ("Durability & data integrity") of
`docs/ROADMAP.md`

**Prerequisites**: The full design cycle ran for this feature and its artifacts are the
authority here — [plan.md](./plan.md), [research.md](./research.md) (R1–R9),
[data-model.md](./data-model.md), [contracts/store-write-path.md](./contracts/store-write-path.md),
[contracts/normalize-url.md](./contracts/normalize-url.md), [quickstart.md](./quickstart.md),
and `checklists/requirements.md` (16/16 complete). `.specify/memory/constitution.md` v1.0.0
governs; each story below cites the principle it answers to.

> **Note on this file's history.** An earlier draft of `tasks.md` was generated in the main
> checkout *before* the plan cycle ran in this worktree, and it stated that no `spec.md` or
> `plan.md` existed. It did, and the draft diverged from the design in two places that
> matter. Both are corrected here:
>
> 1. **`save` is split, not rewritten in place.** The draft folded staging + rename into one
>    function. [contracts/store-write-path.md](./contracts/store-write-path.md) and research
>    R4 require a private staging step that returns the staged path, so a unit test can stop
>    a save at exactly the moment the atomicity guarantee is about. The seam *is* the design.
> 2. **A failed save leaves the staging artifact, it does not delete it.** The draft called
>    for best-effort cleanup on the failure paths. The contract says the opposite: the fixed
>    name (R2) bounds the artifact at one no matter how many saves are interrupted, so
>    cleanup buys nothing — and deleting on rename failure would break the
>    "orphans do not accumulate" test in [quickstart.md](./quickstart.md), which stages
>    repeatedly and counts.

**Tests**: Included and **not optional**. Every line this feature changes lives in `model.rs`
and `store.rs` — the two files Constitution Principle IV names as the testable core, both
already covered by plain `cargo test` against pure functions and a temp dir. The gate is
`cargo test` + `cargo clippy -- -D warnings` from `src-tauri/` plus `pnpm test` / `pnpm build`,
run after **each** story (SC-007), not only at the end.

> **Not in scope**: ROADMAP §2 (mutex-poison recovery, the `update_site` read-modify-write
> race) — adjacent to `store.rs` and tempting to fold in, but a separate feature with a
> separate risk profile. §4's `store.rs` no-op-update coverage gap is likewise left alone.
> `commands.rs` and `load()` are **not modified** (research R7, R8).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1–US3)
- Exact file paths are included in every task

## Path Conventions

Desktop app: vanilla-TS frontend in `src/`, markup at `index.html`, Rust backend in
`src-tauri/`. Paths below are repo-relative. **Every code change in this feature is Rust** —
no frontend source is touched, though `pnpm test` / `pnpm build` still gate.

### File contention

- `src-tauri/src/store.rs` — US1 (`save`) and US3 (`add`), two different functions but a
  shared `#[cfg(test)] mod tests`. Sequential.
- `src-tauri/src/model.rs` — US2 only. Genuinely `[P]` against everything else.
- `src-tauri/src/commands.rs` — read-only for this feature (T024 is a contract check).

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Isolated workspace and a known-green baseline, so any later red is attributable
to this feature.

- [X] T001 Worktree and branch for `003-durability`. Worktree at `site-checker--20260806-102818-durability-and-data`, branch `20260806-102818-durability-and-data`, fast-forwarded onto `main` (`00fd10c`, which carries the before-screenshots commit). `docs/` is gitignored but already present in the worktree. `.specify/feature.json` points at `specs/003-durability`.
- [X] T002 Baseline recorded: `cargo test` **29 passed / 0 failed**, `cargo clippy -- -D warnings` **clean**, `pnpm test` **30 passed**. Green tree confirmed before any edit.
- [X] T003 [P] Re-verified all three ROADMAP §1 claims still describe the tree: `Store::save` (`store.rs:81-90`) ends in a bare `std::fs::write(&self.path, json)`; `normalize_url` (`model.rs:63-85`) returns `candidate` built from the caller's `trimmed` text, so an uppercase scheme survives verbatim; `Store::add` (`store.rs:62-65`) is `self.sites.push(site); self.save()` with no id check. All three hold.
- [X] T004 Live data file protected before any Rust work: `~/Library/Application Support/com.clintparker.site-checker/sites.json` copied to `/tmp/sites.json.safety` **and** `~/sites.json.003-durability.safety` (two copies — `/tmp` is not durable). Both verified byte-identical by `shasum` (`7150bef8…`). No leaked dev server was running.

**Checkpoint**: Isolated workspace, green baseline, all three claims confirmed, live `sites.json` backed up out of tree twice.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Pin the write-path mechanism before any code is written.

**⚠️ CRITICAL**: T005 blocks **US1 only**. US2 and US3 can start without it.

- [X] T005 Confirm and record the staging mechanism settled in research R1–R3, checking each against the tree rather than restating it: **(a)** no new dependency — `tempfile` stays in `[dev-dependencies]` (R1; also keeps ROADMAP §3's 15 MB bundle from growing); **(b)** the staging file is a **fixed** sibling name, `sites.json.tmp`, derived from `self.path`'s own directory — fixed because a randomized name would let orphans accumulate one per crash, violating FR-003/SC-005, and a sibling because `rename` is only atomic within one filesystem (R2); **(c)** `sync_all()` on the staged file before the rename, and **no** parent-directory `fsync` — durability is scoped to process death, not power loss (R3). Record (b) and (c) as the comment on `save` in `src-tauri/src/store.rs` (T014).

**Checkpoint**: Mechanism confirmed against the tree; the same-directory and fixed-name constraints are written down.

---

## Phase 3: User Story 1 - A crash mid-write cannot lose the last edit (Priority: P1) 🎯 MVP

**Goal**: `Store::save` becomes crash-safe. Today it calls `std::fs::write(&self.path, json)`
directly, which truncates `sites.json` to zero and then refills it. A crash, a kill, or a
power loss inside that window leaves a truncated file; the next launch parses it as corrupt,
shows the banner, and starts empty — graceful, but the user's last edit is gone. Stage the
bytes in a sibling temp file, `sync_all` it, then `rename` it over the target: `rename` is
atomic, so a reader sees either the whole old file or the whole new one, never a partial.

**Why P1**: The roadmap calls it the *highest-value item on this roadmap*, and it is the only
item in §1 that can lose user data. Constitution Principle II ("Results Are Ephemeral,
**Config Is Sacred**") makes `sites.json` the one file this app owns and is responsible for.

**Independent Test**: Automated, via the staging seam (R4) — stage a save without renaming,
then `load()` the live path and assert it still returns the *previous* list with no warning.
Against the current code that test cannot be written at all, because the truncation *is* the
write.

### Tests for User Story 1

- [X] T006 [US1] In `src-tauri/src/store.rs`'s `mod tests`, add `an_interrupted_save_leaves_the_previous_list_loadable`: build a store in a `tempfile::tempdir()`, `add` two sites (a real save), then push a third onto the in-memory list and call the **staging step directly** without renaming. Assert `load()` on the live path still returns **two** sites and `warning.is_none()` — the staged third site is invisible. This is spec scenario 1 and the core of the whole feature.
- [X] T007 [US1] In the same module, add `a_staged_save_holds_the_new_contents_beside_the_live_file`: after the same staging step, assert `sites.json.tmp` exists in the same directory and parses as the **three**-site list, while `sites.json` still holds two. Proves the staging artifact is real, complete, and simply not published yet (FR-003).
- [X] T008 [US1] In the same module, add `a_successful_save_leaves_no_staging_file_behind`: after two `add` calls, read the directory entries and assert the only file present is `sites.json`. This is what catches a `rename` that silently became a `copy`, or an early return that skips publication.
- [X] T009 [US1] In the same module, add `repeated_staging_never_accumulates_more_than_one_artifact`: call the staging step three times without renaming, then count directory entries and assert exactly `sites.json` + one `sites.json.tmp`. This is the test that pins the *fixed* name from R2 — a randomized name would produce three artifacts and pass every other test in this story (FR-003, SC-005).
- [X] T010 [US1] In the same module, add `a_failed_save_leaves_the_previous_file_intact`: create a valid `sites.json` via `add` and capture its bytes, then place a **directory** at the *staging* path (`sites.json.tmp`) so `File::create` fails with `EISDIR` and the rename never runs. Assert the next `add` returns `Err` and that `sites.json` is still byte-identical to the capture. Failing the staging step rather than the rename is what makes this test possible at all — a directory at the `sites.json` path itself would mean there is no previous file to preserve, so it could not assert the thing FR-004 is about. Spec scenario 3.
- [X] T011 [P] [US1] In the same module, add `a_stale_staging_file_does_not_affect_load_or_the_next_save`: write garbage to `sites.json.tmp` alongside a valid `sites.json` (simulating a crash between staging and rename), assert `load` returns the real sites with **no warning**, then `add` a site and assert the reload sees all of them. Pins that a leftover artifact from a crashed run is inert and that the next save reclaims it (R8 — `load` is not modified, so this must hold by construction).

### Implementation for User Story 1

- [X] T012 [US1] In `src-tauri/src/store.rs`, extract a private staging step `fn stage(&self) -> Result<PathBuf, String>` per [contracts/store-write-path.md](./contracts/store-write-path.md): keep the existing `create_dir_all(parent)` and `to_string_pretty` steps **and their error strings unchanged**; derive the staging path as a fixed sibling of `self.path`; write the bytes with `File::create` + `write_all` (not `fs::write`, which hands back no handle to sync); `sync_all()`; return the staging path. Private, so `mod tests` (a child module) reaches it while nothing outside `store.rs` can — no `#[cfg(test)]`-only method and no new public surface.
- [X] T013 [US1] In `src-tauri/src/store.rs`, reduce `save` to the staging step followed by `std::fs::rename(&staged, &self.path)`. Do **not** clean up the staging file on either failure path: per the contract, on staging failure the rename never runs and `sites.json` is untouched, and on rename failure the staged file remains as the single permitted orphan — bounded at one by the fixed name and reclaimed by the next save (T009, T011 pin both halves). Give each new failure point its own message but keep the existing shape `Could not …: {e}`, one line, naming `sites.json`, so `warn_on_write_failure` in `src-tauri/src/commands.rs` keeps producing banner text the user already knows (R9).
- [X] T014 [US1] Add a comment above the write path in `src-tauri/src/store.rs` recording *why*, at the density of the surrounding comments: `rename` is atomic within a filesystem, so a reader sees the whole old file or the whole new one and never a truncated one; the staging file is a sibling because a cross-filesystem rename is not atomic; the name is fixed so interrupted saves cannot accumulate; `sync_all` runs before the rename so the bytes are on disk before the name points at them. State the honest limit — macOS `fsync` does not force the drive's own cache the way `F_FULLFSYNC` does, so this defends against process crash and kill, not against sudden power loss on a drive that lies about flushing. Note that `load` reads only the path it was handed, so a stale sibling is inert.
- [X] T015 [US1] Verify: `cargo test` and `cargo clippy -- -D warnings` green from `src-tauri/`, Rust count strictly above 29. Existing `corrupt_file_yields_an_empty_list_a_warning_and_is_left_on_disk` and `writes_create_the_parent_directory` must pass **unmodified** (FR-005, FR-006).

**Checkpoint**: A save is all-or-nothing. The user's list cannot be truncated by a crash mid-write.

---

## Phase 4: User Story 2 - `HTTPS://` normalizes to `https://` (Priority: P2)

**Goal**: `normalize_url` in `src-tauri/src/model.rs` lowercases the scheme it returns. Today
`has_leading_scheme` accepts `HTTPS` (uppercase is ASCII-alphanumeric) and `url::Url::parse`
lowercases the scheme *internally*, so the `matches!(parsed.scheme(), "http" | "https")` check
already passes — but the function returns `candidate`, the caller's raw text, so
`HTTPS://example.com` persists verbatim.

**Why P2**: Cosmetic — the URL still works, the check still runs. It ranks above US3 only
because it is reachable by a real user (paste a URL from a document that shouts) whereas US3
is not reachable at all.

**Independent Test**: the input→output table in
[contracts/normalize-url.md](./contracts/normalize-url.md), in full.

### Tests for User Story 2

- [X] T016 [P] [US2] In `src-tauri/src/model.rs`'s `mod tests`, add the contract table: `lowercases_an_uppercase_scheme` (`HTTPS://example.com` → `https://example.com`, `HtTp://example.com/health` → `http://example.com/health`); `lowercases_only_the_scheme` (`HTTP://Example.COM/Path?Q=1` → `http://Example.COM/Path?Q=1`, plus `https://EXAMPLE.com` unchanged) — this is the test that stops the fix from being a lazy `to_ascii_lowercase()` on the whole string, which would corrupt case-sensitive paths and query values; `an_uppercase_scheme_in_a_query_is_left_alone` (`example.com?next=HTTP://x.dev` → `https://example.com?next=HTTP://x.dev`, spec scenario 4); and `rejects_a_non_http_scheme_regardless_of_case` (`FTP://example.com` is still `Err`, spec scenario 5). Confirm they are red before T017.

### Implementation for User Story 2

- [X] T017 [P] [US2] In `src-tauri/src/model.rs`, change `has_leading_scheme(s: &str) -> bool` to return `Option<usize>` — the byte index of `://` when the prefix is a valid scheme, `None` otherwise. The body already computes that index: return `Some(i)` where it currently returns `true` and `None` where it returns `false` (including the `Some(0)` empty-scheme case). Rename it to read as a lookup rather than a predicate (`leading_scheme_end`) and keep its doc comment's explanation of the `contains("://")` false positive it guards against.
- [X] T018 [US2] In `normalize_url`, build `candidate` from the new return: on `Some(i)`, `format!("{}{}", &trimmed[..i].to_ascii_lowercase(), &trimmed[i..])`; on `None`, keep `format!("https://{trimmed}")` unchanged. `to_ascii_lowercase`, not `to_lowercase` — a scheme is ASCII by the rule `leading_scheme_end` already enforces, and Unicode case folding here would be misleading. The slice needs no boundary check: `find` returns a byte index at a character boundary and the guard proves every preceding byte is ASCII (R6).
- [X] T019 [US2] Add a comment in `src-tauri/src/model.rs` noting the one-time consequence for existing data: there is no migration — a site already persisted as `HTTPS://…` keeps that value until the user next edits it, because `load()` does not call this function. On that edit, `src/main.ts`'s `upsertSite` sees a URL change (it compares `previous.url !== site.url`) and drops the row to Pending, and `update_site` in `src-tauri/src/commands.rs` sees a changed URL and clears `method_override`, costing one extra request to re-learn HEAD support. Correct-but-surprising once, then stable. Confirm the seven existing `normalize_url` tests pass **unmodified** — none uses an uppercase scheme, so none should need changing.

**Checkpoint**: The persisted scheme is always lowercase; host, path, and query case are untouched.

---

## Phase 5: User Story 3 - `Store::add` refuses a duplicate id (Priority: P3)

**Goal**: `Store::add` currently pushes unconditionally. Two sites with the same id would make
`get`, `update`, and `delete` behave inconsistently (`get`/`update` hit the first, `delete`
removes both). The roadmap calls this "belt-and-braces only" — unreachable today because the
only caller, `add_site` in `src-tauri/src/commands.rs`, generates a fresh v4 UUID per site.

**Why P3**: Unreachable. It is here to make the invariant explicit at the layer that owns it
rather than leaving it as a property of one caller's id generator — a future importer,
migration, or restore path would reopen it.

**Independent Test**: Automated only. See the quickstart's three rows.

### Tests for User Story 3

- [X] T020 [US3] In `src-tauri/src/store.rs`'s `mod tests`, add `add_rejects_a_duplicate_id`: `add` `a_site("one")`, then `add` a **mutated** `a_site("one")` (change `interval_secs`, so a silent replace is distinguishable from a silent no-op), assert the second call returns `Err`, and assert via a **reload** that the file holds exactly one entry for `"one"` with the **original** interval. Asserting on the reload rather than the in-memory list is what proves the rejection happened before any write (spec scenarios 1–2, FR-009).
- [X] T021 [P] [US3] In the same module, add `add_still_accepts_a_distinct_id`: `add` `a_site("one")` then `a_site("two")`, assert both are `Ok`, the list length is 2, and order is preserved. Guards against a refusal predicate that is too broad (spec scenario 3).

### Implementation for User Story 3

- [X] T022 [US3] In `src-tauri/src/store.rs`, make `add` reject **before** mutating: if `self.sites.iter().any(|s| s.id == site.id)`, return `Err(…)` without pushing and without calling `save`. Rejecting before the mutation (rather than pushing, failing, and unwinding) is what keeps the in-memory list and the file in agreement on this path — that ordering is the contract, not an implementation detail.
- [X] T023 [US3] Add a comment on `add` in `src-tauri/src/store.rs` naming the caller-contract seam this introduces, because it is not obvious from either side: `warn_on_write_failure` in `src-tauri/src/commands.rs` documents its `Err` channel as "the in-memory change stands and the UI shows a banner", which is true for a failed disk write but **not** for this new branch, where nothing was applied at all. Record that this is strictly the safer direction (nothing persisted, nothing in memory) and that the branch is unreachable while `add_site` mints a v4 UUID per site.
- [X] T024 [US3] Read `add_site` in `src-tauri/src/commands.rs` against the new `Err` branch and confirm the behaviour rather than assuming it: on `Err`, `warn_on_write_failure` emits a `store-warning` banner carrying the refusal message, but `add_site` then still calls `state.engine.start(site)` and returns `Ok(site)`, so the frontend would add a row for a site that was never stored — a row that vanishes on the next launch. Per research R7 this stays as-is (unreachable branch; distinguishing refusal from write failure means a typed error and a shell that branches on it — real work in the untested layer for a branch the shipped app cannot reach). **Do not edit `commands.rs`.** Record the deferral in `docs/ROADMAP.md` §2 (T027).

**Checkpoint**: A duplicate id is impossible to persist, and the caller-contract seam is written down rather than discovered later.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T025 Run the full Constitution "Quality Gates" set on the finished branch: `cargo test` and `cargo clippy -- -D warnings` from `src-tauri/`, plus `pnpm test` and `pnpm build`. All four green. Rust strictly above 29 with none removed; frontend **exactly 30** — this feature touches no frontend file, so any other number means something out of scope was edited.
- [X] T026 [P] Amend the spec's symlink edge case (`specs/003-durability/spec.md`, "A directory or symlink where `sites.json` should be") to state the two outcomes separately, per verified research R5: a **directory** at the path makes the rename fail and nothing is destroyed (unchanged from today), whereas a **symlink** is now *replaced* by the rename rather than followed — the target keeps every byte it held, but the indirection is gone. This is inherent to `rename` and cannot be avoided without reopening the truncation window the feature exists to close.
- [X] T027 [P] Update `docs/ROADMAP.md`: replace section 1 with a "Done" note pointing at `specs/003-durability/`, mirroring the two existing notes, and renumber the sections below. Append the two deliberate deferrals under the (renumbered) concurrency section: the `add_site` refusal-vs-write-failure inconsistency (R7, T024) and the symlink replacement behaviour (R5, T026). Note that `docs/` is gitignored — this edit is local-only and will not appear in the PR.
- [X] T028 [P] Add a `CHANGELOG.md` entry following the format of the existing "Robustness — 2026-08-05" entry: short preamble, a Tasks/Source line pointing at `specs/003-durability/`, then `### Fixed` / `### Changed` / `### Added` sections. Atomic writes and the duplicate-id refusal belong under Fixed; the scheme lowercasing is a Changed (it alters what gets persisted for a class of input). Unlike 002, this feature **does** have a `spec.md` and `plan.md` — say so rather than copying 002's "there is no spec.md" line.
- [X] T029 Confirm the live data file survived: compare `~/Library/Application Support/com.clintparker.site-checker/sites.json` against the T004 safety copies, and confirm no `sites.json.tmp` is left in the app-support directory. Remove the safety copies only after the comparison.
- [X] T030 Re-read the three changed hunks against Constitution Principles II and IV: confirm the `sites.json` **shape** is unchanged (US1 changes how bytes land, never what they are; US2 changes the value of one field for uppercase-scheme input only), that no logic moved into an untested shell, and that every new branch in `save`, `stage`, `normalize_url`, and `add` is reachable from a test. Record anything deliberately left untested in `docs/ROADMAP.md` §4 rather than dropping it silently.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies. T004 must happen before any `pnpm tauri dev` launch.
- **Foundational (Phase 2)**: Depends on Setup. Blocks **US1 only**. US2 and US3 can start without it.
- **User Stories (Phases 3–5)**: Depend on Setup. Ordered P1 → P3 by value; US2 is genuinely concurrent, US1 and US3 are not (shared file).
- **Polish (Phase 6)**: Depends on every story taken.

### User Story Dependencies

All three stories are functionally independent — none reads, calls, or relies on another's
change. The ordering constraints are:

- **US1 (P1)** and **US3 (P3)** both edit `src-tauri/src/store.rs` — US1 the write path, US3
  `add`, and both add tests to the same `mod tests`. Land US1 first: it is the item with real
  value, and US3's T020 asserts on a reload, which exercises the new write path.
- **US2 (P2)** owns `src-tauri/src/model.rs` alone and conflicts with nothing.
- **T024** reads `src-tauri/src/commands.rs` but must not edit it.

### Within Each User Story

- Tests before implementation in all three stories — write them against the current code,
  confirm they are red (or fail to compile, where the seam does not exist yet), then implement.
- Implementation → comment → verify, in that order.
- Each story ends green (`cargo test` + `cargo clippy -- -D warnings`) before the next begins.

### Parallel Opportunities

- **T003** is read-only verification — parallel with anything in Phase 1.
- **T011** covers a different concern (`load` vs the write path) than T006–T010.
- **US2 in its entirety (T016–T019)** is parallel with US1 and US3 — different file, no shared
  symbol. This is the only cross-story parallelism in the feature.
- **T021** is a different test from T020 and independent of it.
- **T026** (`spec.md`), **T027** (`docs/ROADMAP.md`), and **T028** (`CHANGELOG.md`) are three
  independent documents.
- US1 and US3 are **not** parallel with each other.

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. Phase 1: Setup (T001–T004).
2. Phase 2: T005 — confirm the staging mechanism.
3. Phase 3: US1 — atomic saves (T006–T015).
4. **STOP and VALIDATE**: `cargo test`, `cargo clippy -- -D warnings`, `pnpm test`.
5. This alone is a shippable PR, and the right one to ship first — it is the only item on the
   entire roadmap that prevents data loss.

### Incremental Delivery

1. Setup → Foundational → US1 → validate → ship (MVP).
2. Add US2 → validate → ship. One-file change, seven existing tests as the regression net.
3. Add US3 → validate → ship. Smallest and most latent.
4. Phase 6 polish over whatever shipped.

Bundling all three into one PR is reasonable — one M and two S — and matches how
`002-robustness` shipped. The increments exist so a partial run still lands something
coherent, and US1 alone is coherent.

### Parallel Team Strategy

Not applicable, and worth saying so. Three edits across two Rust files: a second developer
would spend more time coordinating `store.rs` than the split saves. Constitution Principle I
("One Mac, One Person") describes the product; this feature matches it in scale.

---

## Notes

- `[P]` tasks = different files, no dependencies. Used sparingly here on purpose.
- No frontend source is edited. `pnpm test` and `pnpm build` still gate, per the Constitution's
  Quality Gates.
- Nothing here touches the Rust/TS snake_case contract (Principle V). US1 changes *how*
  `sites.json` is written, never *what* it contains; US2 changes the persisted `url` value only
  for input that carried an uppercase scheme.
- **No new runtime dependency.** `tempfile` stays in `[dev-dependencies]`; if `Cargo.toml`
  gained an entry, something went wrong (R1).
- `docs/` is gitignored: T027's roadmap edit stays local and will not show in the PR.
- The live `sites.json` is at risk in a way it was not during `002-robustness` — T004's
  out-of-tree copies are the recovery path, not bookkeeping.

## Not done: the manual `pnpm tauri dev` pass

[quickstart.md](./quickstart.md) ends with a manual confirmation of FR-011 — launch the app,
add a site typed as `HTTPS://…`, edit and delete one, and check the app-support directory.
**That pass was not run**, because this implementation ran in a non-interactive session where
a Tauri dev launch has previously leaked a stale dev server holding the live `sites.json` —
exactly the file this feature rewrites the write path for. The risk of running it unattended
outweighed the confirmation.

What stands in for it, and what does not:

- **Covered by other evidence.** FR-011 says nothing about the app changes. `git status`
  confirms exactly two files differ from `main` — `src-tauri/src/model.rs` and
  `src-tauri/src/store.rs`. No frontend file, no `index.html`, no `Cargo.toml`, no command
  signature. `pnpm test` is unchanged at 30 and `pnpm build` succeeds.
- **Not covered.** That the app launches and behaves normally end-to-end against a *real*
  app-support directory rather than a `tempfile::tempdir()`, and that no `sites.json.tmp`
  survives a real session. The unit tests pin both properties against a temp dir; they do not
  prove the app boots.

The safety copies from T004 (`/tmp/sites.json.safety`, `~/sites.json.003-durability.safety`)
were **left in place** so this pass can be run later. Remove them once it has.
