# Tasks: Concurrency & Robustness Hardening

**Input**: Design documents from `/specs/20260806-120353-concurrency-hardening/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: **REQUIRED, and required to fail first.** FR-017 and SC-006 mandate that each of the
three behaviours be pinned by an automated test verified to fail against the code as it stands
before the fix lands. This is not the template's optional TDD — it is a spec requirement, so the
test tasks below are not skippable.

**Where this work happens**: the worktree
`~/src/site-checker--20260806-120325-section-1-docs` (branch
`20260806-120325-section-1-docs`). Every path below is relative to that worktree root, **with one
deliberate exception**: T041 edits `docs/ROADMAP.md` in the *primary* checkout
`~/src/site-checker`, because `docs/` is gitignored and an edit made here
never reaches `main` (research R10 — this is exactly how `003-durability` lost its roadmap edit).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- ~~**T025 and T040 are deliberately left `[ ]`.** Both are part-done: their by-eye code reads
  were performed and recorded, but the GUI click-through was not. They stay unchecked so the
  companion's task sync reports `implementing` rather than `implemented` — a partly-verified
  task must not read as a finished one. See "Not done, and not claimable" in the Notes.~~
  **Closed 2026-08-11.** Verification round 2 drove the real window through the macOS
  accessibility API with synthetic mouse clicks and performed a successful **add, edit, and
  delete**, hashing and parsing `sites.json` after each — the exact click-through both tasks
  were waiting on. Recorded in `CHANGELOG.md`'s *"The shell layer was finally exercised against
  a running window"*. Both are now `[X]`.

## Path Conventions

Desktop app, existing layout kept. Backend is `src-tauri/src/`; the frontend `src/` **must not
appear in the diff at all** (FR-016, and see [contracts/command-surface.md](./contracts/command-surface.md)).
Rust tests live in `#[cfg(test)] mod tests` inside the module they cover — there is no `tests/`
directory in this project and this feature does not add one.

---

## Phase 1: Setup

**Purpose**: make the worktree runnable and record the baseline every later gate is measured against.

- [X] T001 Run `pnpm install` at the worktree root — a fresh worktree has no `node_modules` and `pnpm test` cannot run without it (quickstart Prerequisites)
- [X] T002 Record the baseline in this file's Notes section: `cd src-tauri && cargo test` (expect 42 passed / 0 failed), `pnpm test` from the root (expect 30 passed / 0 failed), `cd src-tauri && cargo clippy -- -D warnings` (expect clean). If any count differs from 42/30, stop — the plan's baseline was re-confirmed in this worktree on 2026-08-06 and a drift means something else changed underneath

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: none. **This phase is intentionally empty.**

There is no shared prerequisite to build. The three stories touch four Rust files between them
(`lock.rs`, `lib.rs`, `commands.rs`, `engine.rs`, `store.rs`) but share no new type, no new
module, and no new dependency: `lock.rs` belongs wholly to US1, `AddError` wholly to US2,
`Store::replace` wholly to US3. Inventing a foundational task here would only be scaffolding for
its own sake.

**What replaces it is ordering, not blocking** — see [Dependencies](#dependencies--execution-order).
The stories are sequenced US1 → US2 → US3 because US1 rewrites every store lock site in
`commands.rs` and `engine.rs`; landing US2 or US3 first means writing `.lock().unwrap()` into code
US1 then has to rewrite. That is a merge-cost argument, not a correctness one — each story remains
independently shippable with all three gates green, exactly as the plan requires.

**Checkpoint**: Setup done — User Story 1 can begin.

---

## Phase 3: User Story 1 - The app stays usable after an internal fault (Priority: P1) 🎯 MVP

**Goal**: one internal fault costs at most the operation it happened in. Every one of the app's
shared-state accesses recovers from a prior poisoning instead of cascading a panic, the recovered
data is preserved exactly as the faulting thread left it, and a recovered *site-list* lock raises
exactly one banner — while a recovered task-registry or startup-warning lock stays silent.

**Independent Test**: poison a mutex from a test thread, then confirm the next access returns a
usable guard over the half-applied data and reports the recovery once. Then
`grep -rn 'lock()\.unwrap()' src-tauri/src/` returns no matches (SC-002).

**Covers**: FR-001 – FR-007 · [contracts/lock-recovery.md](./contracts/lock-recovery.md) ·
data-model "State: lock poisoning", "Which locks warn"

### Tests for User Story 1 ⚠️ WRITE FIRST, CONFIRM RED

> These must fail against today's behaviour before the fix lands. The stub in T003 exists
> precisely so they can compile *and* fail rather than fail to compile.

- [X] T003 [US1] Create `src-tauri/src/lock.rs` with `pub fn recover<T>(mutex: &Mutex<T>) -> (MutexGuard<'_, T>, bool)` **stubbed to today's behaviour** — `(mutex.lock().unwrap(), false)` — and register `mod lock;` in `src-tauri/src/lib.rs`. This is the deliberate red state: the tests below compile against it and fail
- [X] T004 [US1] Add the poison-injection helper to `lock.rs`'s `#[cfg(test)] mod tests`: spawn a thread that panics while holding the guard, `join()` it discarding the `Err`, wrapped in a scoped `std::panic::set_hook` / `take_hook` pair so `boom` does not print on every green run (research R6). No sleeps, no subprocess, no timing — the mechanism is deterministic. If `cargo test`'s own per-thread output capture already suppresses the message, drop the hook dance and say so in a comment
- [X] T005 [US1] Test in `src-tauri/src/lock.rs` — `recover` on a poisoned mutex returns a usable guard and reports `true` (guarantee L1/L3, FR-001). Confirm it FAILS against the T003 stub (it panics)
- [X] T006 [US1] Test in `src-tauri/src/lock.rs` — the recovered guard holds the data **exactly as the panicking thread left it**: poison the mutex partway through a multi-step mutation and assert the half-applied part is still present (guarantee L3, FR-006). Confirm it FAILS against the stub
- [X] T007 [US1] Test in `src-tauri/src/lock.rs` — a second `recover` after the first returns `false`, proving the poison is one-shot and no permanent degraded mode accumulates (guarantee L4, spec edge case). Confirm it FAILS against the stub
- [X] T008 [US1] Test in `src-tauri/src/lock.rs` — on an unpoisoned mutex `recover` returns the guard and `false`, indistinguishable from `lock().unwrap()` (guarantee L2, FR-007). This one PASSES against the stub; that is correct and is the FR-007 no-op pin

### Implementation for User Story 1

- [X] T009 [US1] Implement `recover` in `src-tauri/src/lock.rs` per the contract's shape: `match mutex.lock()` → `Ok` returns `(guard, false)`; `Err(poisoned)` calls `mutex.clear_poison()` **while the guard is still live** (guarantee L5) then returns `(poisoned.into_inner(), true)`. Confirm T005–T007 now pass and T008 still does
- [X] T010 [US1] Add `pub struct SharedStore` to `src-tauri/src/lock.rs`: private `inner: Arc<Mutex<Store>>` and private `app: AppHandle`, **no accessor for either**, `#[derive(Clone)]`, plus `SharedStore::new(app: AppHandle, store: Store) -> Self`. The absent getter is the whole point — it makes `.lock().unwrap()` on the site list unwritable outside this module (research R2, data-model "New: `SharedStore`")
- [X] T011 [US1] Implement `SharedStore::lock(&self) -> MutexGuard<'_, Store>` in `src-tauri/src/lock.rs`: delegate to `recover`, and when and only when the flag is `true` emit one `store-warning` event whose message says the saved list may not reflect the user's most recent change (guarantees S1–S4, FR-004). No fault → no emit, byte-identical behaviour to today (S3, FR-007)
- [X] T012 [US1] Move `StoreWarning` and `warn_on_write_failure` out of `src-tauri/src/commands.rs` and into `src-tauri/src/lock.rs`, the latter becoming `SharedStore::warn_on_write_failure(&self, result: Result<(), String>)`. Same event name, same `{ message }` payload, same behaviour — the move exists so every banner in the app is raised from one place, which is what FR-004's "using the existing warning banner rather than a new mechanism" requires
- [X] T013 [US1] Rewrite `setup()` in `src-tauri/src/lib.rs`: build one `SharedStore` from `loaded.store`, hand clones to `Engine::new` and `AppState`, and take the startup `list()` through `SharedStore::lock()`. Add a comment at that call site noting the startup caveat — the window's JS has not registered its `store-warning` listener yet, so a warning emitted here would be dropped; this is inert because the store was constructed a few lines above and cannot be poisoned yet (contract "Startup caveat")
- [X] T014 [US1] Update `src-tauri/src/engine.rs`: `Inner.store` and `Engine::new`'s parameter become `SharedStore`; `persist_get_fallback` (line ~115) takes `self.store.lock()`. Switch the two `tasks` locks in `start` (line ~56) and `stop` (line ~69) to `lock::recover(&self.inner.tasks).0`, each with a comment saying **why** the flag is discarded — the registry is ephemeral by design, rebuilt every launch, and every scheduling call replaces rather than accumulates (FR-005, Constitution II). A bare `.0` with no comment is a review finding
- [X] T015 [US1] Update `src-tauri/src/commands.rs`: `AppState.store` becomes `SharedStore`; `list_sites`, `add_site`, `update_site` (both sites, for now), and `delete_site` call `state.store.lock()`; `get_warning` becomes `lock::recover(&state.warning).0.take()` with a comment saying why it is silent — a warning about the warning channel names no consequence the user can act on (FR-003). Delete the now-moved `StoreWarning` / `warn_on_write_failure` and route callers to the `SharedStore` methods
- [X] T016 [US1] Add the source-text guard test to `src-tauri/src/lock.rs` (research R7): `include_str!` each of the crate's modules (`lib.rs`, `commands.rs`, `engine.rs`, `store.rs`, `check.rs`, `model.rs`, `main.rs`, `lock.rs`) and assert none contains `.lock().unwrap()`. Build the needle with `concat!(".lock()", ".unwrap()")` so the assertion's own source text is not a match. This is the pin for the three call-discipline sites that `SharedStore` cannot make safe by construction, and the only thing that catches the eleventh site added next year. It is a blunt instrument — say so in a comment
- [X] T017 [US1] Gate the story: `cd src-tauri && cargo test` (42 + the new lock tests, 0 failed), `cargo clippy -- -D warnings` clean, `pnpm test` still exactly 30 and unmodified, and `grep -rn 'lock()\.unwrap()' src-tauri/src/` returns nothing (SC-002)

**Checkpoint**: US1 is shippable on its own. A fault that used to require a relaunch no longer does.

---

## Phase 4: User Story 2 - A refused add never leaves a ghost row (Priority: P2)

**Goal**: a duplicate-id refusal is reported as a refusal — no row, no timer, no "could not be
saved" banner — while a genuine write failure keeps every one of today's behaviours.

**Independent Test**: ask `Store::add` to store a site whose id is already present and confirm it
yields `AddError::DuplicateId` (not `Write`), with the in-memory list and the file both unchanged;
then read `add_site` and confirm the refusal returns `Err` above `engine.start`.

**Covers**: FR-008 – FR-012 · [contracts/store-mutation-api.md](./contracts/store-mutation-api.md)
(`add`, guarantees A1–A4) · [contracts/command-surface.md](./contracts/command-surface.md)
(guarantees C1–C5) · data-model "New: `AddError`"

### Tests for User Story 2 ⚠️ WRITE FIRST, CONFIRM RED

- [X] T018 [US2] Introduce `pub enum AddError { DuplicateId(String), Write(String) }` in `src-tauri/src/store.rs` with the contract's doc comment on each variant (`DuplicateId` — nothing was applied, in memory or on disk; `Write` — the site is in memory, the save failed), change `add`'s signature to `Result<(), AddError>`, and **map both failure paths to `AddError::Write` for now**. Fix the call sites in `src-tauri/src/commands.rs` so the crate compiles. This is the deliberate red state — the type exists, the distinction does not
- [X] T019 [US2] Tighten the two existing tests in `src-tauri/src/store.rs`: `add_rejects_a_duplicate_id` asserts `AddError::DuplicateId`, and `a_failed_save_leaves_the_previous_file_intact` asserts `AddError::Write`. Both compile untouched against `.is_err()` — which is the problem, and why the contract calls this out. Confirm the first now FAILS against T018
- [X] T020 [US2] Add a test to `src-tauri/src/store.rs` asserting a refused add leaves **both** the in-memory list (`list()` unchanged) and the on-disk file (byte-identical contents, no new staging artifact) exactly as they were (FR-012, guarantee A1)

### Implementation for User Story 2

- [X] T021 [US2] Make `Store::add` in `src-tauri/src/store.rs` return `AddError::DuplicateId` from the id-already-present branch and `AddError::Write` from the save failure, preserving today's ordering — refusal checked, then push, then save (guarantee A4). Confirm T019 and T020 now pass
- [X] T022 [US2] Shrink `Store::add`'s long prose comment in `src-tauri/src/store.rs` to the two variant doc comments; the paragraph about the caller's obligation moves to `commands.rs`'s `add_site`, which is where that obligation actually lives (contract note). The comment exists today only because there was no type to say it with — now there is
- [X] T023 [US2] Branch `add_site` in `src-tauri/src/commands.rs` on the `AddError` variants: `DuplicateId` returns `Err(<refusal message>)` **above** `state.engine.start(...)` so no timer is created and no `site-status` is ever emitted for that site (guarantee C1); `Write` takes today's `warn_on_write_failure` path unchanged (C4); `Ok` proceeds as today (C5). No `store-warning` on the refusal path — one message, not two contradictory ones (C2)
- [X] T024 [US2] Word the refusal message in `src-tauri/src/commands.rs` so it states the site was **not added** and never uses the word "saved", which is what makes the current message a lie (FR-010, guarantee C3). The drafted candidate in the contract is a starting point, not a mandate: *"That site was not added — the list already has an entry with the same identity. Nothing was changed."*
- [X] T025 [US2] Manual check per [quickstart.md](./quickstart.md) §3 — the shell branch has no unit test because `add_site` needs a Tauri `State` (research R7). Run `pnpm tauri dev`, add a site, confirm the happy path is unchanged (FR-007); then read `commands.rs` and confirm by eye that the `DuplicateId` arm returns above `engine.start`, and that the message contains no "saved". Then gate: `cargo test`, `cargo clippy -- -D warnings`, `pnpm test` (still 30, unmodified) ~~**[~] Partially done:** the two by-eye reads were performed and recorded; the `pnpm tauri dev` add-a-site click-through was NOT (non-interactive session).~~ **[X] Closed 2026-08-11** — verification round 2 performed the add against the real running window via the accessibility API. Gate re-run green.

**Checkpoint**: US1 and US2 both shippable independently.

---

## Phase 5: User Story 3 - Two edits to one site cannot discard each other (Priority: P3)

**Goal**: an edit reads the current entry, decides `method_override` from it, writes the result,
and saves as one indivisible step — enforced by a single `&mut self` borrow, so there is no moment
for a second edit to interleave into rather than a rule a future caller must remember.

**Independent Test**: two threads editing one site through `Store::replace` over a shared
`Arc<Mutex<Store>>` leave the second edit applied on top of the first's *result*, never on top of
the state before either.

**Covers**: FR-013, FR-014 · [contracts/store-mutation-api.md](./contracts/store-mutation-api.md)
(`replace`, guarantees R1–R6) · [contracts/command-surface.md](./contracts/command-surface.md)
(guarantees U1–U5) · data-model "New: `Replaced`"

### Tests for User Story 3 ⚠️ WRITE FIRST, CONFIRM RED

> The contention test is the story. Quickstart §4: *"It should fail against a deliberately re-split
> read-then-write implementation, so if it passes both ways it is testing the wrong thing."* T026
> builds that re-split shape as a permanent negative control so the claim stays checkable rather
> than being a one-time observation at implementation time.

- [X] T026 [US3] Add a negative-control test to `src-tauri/src/store.rs` named `the_old_two_lock_shape_would_lose_the_earlier_edit`: a local test helper that reproduces today's `commands.rs` shape over an `Arc<Mutex<Store>>` — lock, `get`, **drop the guard**, decide `method_override`, lock again, `update` — driven by two threads. Sequence it with a `std::sync::Barrier` (or channel), never sleeps, so thread B completes entirely inside thread A's read→write window and the loss is deterministic. Assert the earlier edit **is** discarded. Comment that this test exists to prove T028 is not vacuous
- [X] T027 [US3] Add `pub struct Replaced { pub site: Site, pub write: Result<(), String> }` and `pub fn replace(&mut self, id: &str, url: String, label: Option<String>, interval_secs: u64) -> Option<Replaced>` to `src-tauri/src/store.rs`, moving the `method_override` decision rule **verbatim** from `commands.rs` (unchanged URL carries it forward, changed URL clears it). FR-014 requires the behaviour be identical; moving it is the only change. Inputs arrive pre-shaped — `normalize_url`, `clamp_interval`, and `empty_to_none` stay in `commands.rs`, because they are input shaping, not list invariants
- [X] T028 [US3] Add the contention test to `src-tauri/src/store.rs`: the same two-thread, `Barrier`-sequenced shape as T026 but through `Store::replace`, asserting the later edit is applied on top of the earlier one's result and nothing was decided from a picture the earlier edit had already replaced (FR-013, guarantee R1). Confirm it FAILS if pointed at T026's helper and PASSES against `replace`

### Tests for the FR-014 rules (single-threaded)

- [X] T029 [US3] Test in `src-tauri/src/store.rs` — an unchanged URL carries the existing `method_override` forward (guarantee R2, FR-014)
- [X] T030 [US3] Test in `src-tauri/src/store.rs` — a changed URL sets `method_override` to `None` so HEAD support is re-learned against the new address (guarantee R3, FR-014, Constitution III)
- [X] T031 [US3] Test in `src-tauri/src/store.rs` — an id not in the list returns `None` and writes **nothing**: no save, no touch of `sites.json`, no staging artifact (guarantee R4, FR-014)
- [X] T032 [US3] Test in `src-tauri/src/store.rs` — list order is preserved across a `replace`, as `update` already guarantees (guarantee R6, FR-015)
- [X] T033 [US3] Test in `src-tauri/src/store.rs` — when the save fails, `Replaced.write` is `Err` **and** the edited site is present in the in-memory list, so the caller can keep the row and fire the banner (guarantee R5, FR-011 analogue). Reuse the existing failure-injection approach from `a_failed_save_leaves_the_previous_file_intact`

### Implementation for User Story 3

- [X] T034 [US3] Rewrite `update_site` in `src-tauri/src/commands.rs` as a caller of `Store::replace`: `normalize_url` first (guarantee U1), then one store lock; `None` → `Err("That site no longer exists")` with nothing written (U2); `Some(Replaced { site, write })` → `warn_on_write_failure(write)`, reschedule, `Ok(site)` (U3, U4, U5). Delete the `method_override` decision from this file — it now lives in `store.rs`
- [X] T035 [US3] Close the guard-lifetime trap in `update_site` in `src-tauri/src/commands.rs` — **a correctness trap, not a style point** (contract's implementation note): bind the `SharedStore` guard to a named variable inside an explicit scope and let it drop **before** `state.engine.reschedule(...)`, rather than leaving it as a temporary in a `let ... else`. Holding the store lock across the scheduling call is not a deadlock today, but it is a lock-ordering hazard nobody should have to reason about later. Verify by reading the resulting code, not by assuming the borrow checker caught it
- [X] T036 [US3] Confirm `Store::update` in `src-tauri/src/store.rs` is **unchanged** and still used by `engine::Inner::persist_get_fallback`. It is deliberately not absorbed into `replace`: the GET-fallback write is a legitimate blind write, and giving it the edit rules would mean it had opinions about URLs and labels that it has no business having (contract, research R5)
- [X] T037 [US3] Gate the story: `cd src-tauri && cargo test` (all green, including `cargo test store::replace`), `cargo clippy -- -D warnings` clean, `pnpm test` still exactly 30 and unmodified

**Checkpoint**: all three stories independently functional. Lock-site count is now 9, not 10 — `update_site`'s two collapsed into one, and that collapse *is* this story (research R9).

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T038 Verify the scope boundary by reading the diff's **file list**, not the diff: no file under `src/` may appear, and no file under `src-tauri/src/` other than `lock.rs`, `lib.rs`, `commands.rs`, `engine.rs`, `store.rs`. Any frontend change is a scope violation under FR-016 and should be challenged at review (contract "What the frontend must not need")
- [X] T039 Confirm FR-018 is honoured: `git diff` shows no change to `Store::stage`, `save`, `staging_path`, or `load` in `src-tauri/src/store.rs`. The symlink-at-the-path behaviour stays as `003-durability` left it — undoing it would reopen the truncation window that feature exists to close
- [X] T040 Run [quickstart.md](./quickstart.md) §5, the seam checked by eye rather than by machine: `pnpm tauri dev`, confirm the app launches, the list loads, and checks start (the `SharedStore` rewrite touched every store access, so this is the smoke test for all of them); add, edit, and delete a site and confirm all three behave exactly as before (FR-007); read `SharedStore::lock` and confirm the recovery flag is wired to a `store-warning` emit with a message about the saved list; read `engine.rs`'s two `tasks` locks and `get_warning`'s `warning` lock and confirm none of them emits (FR-005, FR-003) ~~**[~] Partially done:** `pnpm tauri dev` was run — the app compiled, launched, loaded the list and ran without panic, and left `sites.json` byte-identical. The add/edit/delete click-through was NOT performed (non-interactive session).~~ **[X] Closed 2026-08-11** — verification round 2 performed all three (add, edit, delete) against the real running window via the accessibility API, hashing and parsing `sites.json` after each; the edit kept its list index and the delete survived a relaunch. All three by-eye code reads were done.
- [X] T041 Drain §1 of `docs/ROADMAP.md` **in the primary checkout** `~/src/site-checker`, NOT this worktree — `docs/` is gitignored and an edit made here never reaches `main` (research R10). Retire three of the four items; **keep the fourth** (the symlink-at-the-path note) with FR-018's reason attached. The retirement note should say the section closed three of four and why the fourth stays
- [X] T042 Full merge bar (SC-007), nothing disabled, nothing skipped, no `#[ignore]`: `cd src-tauri && cargo test` (42 + new, 0 failed), `pnpm test` from the root (**exactly 30**, unmodified — SC-008), `cd src-tauri && cargo clippy -- -D warnings` clean
- [X] T043 Update this file's Notes section with the final test counts and record which checks were verified by eye rather than by machine (the `SharedStore::lock` emit wiring and `add_site`'s refusal ordering), so the ship record states it rather than leaving it to be discovered at review

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies — start immediately
- **Foundational (Phase 2)**: empty by design; nothing blocks
- **User Stories (Phase 3–5)**: each depends only on Setup. Sequenced P1 → P2 → P3 by preference, not by requirement — see below
- **Polish (Phase 6)**: depends on all three stories

### User Story Dependencies

No story depends on another for correctness. Each leaves `cargo test`, `cargo clippy -- -D
warnings`, and `pnpm test` green on its own, which is what makes each independently shippable.

The **recommended order is US1 → US2 → US3**, for one concrete reason: US1 replaces every store
lock site in `commands.rs` and `engine.rs` with `SharedStore::lock()`. Landing US2 or US3 first
means writing `state.store.lock().unwrap()` into `add_site` / `update_site` that US1 then has to
rewrite. That is rework, not breakage. The riskiest structural change also lands first, with the
two smaller ones on top of it.

**If a different order is chosen**, the only adjustment is T016's source-text guard test, which
cannot pass until US1 is complete — move it to whichever position US1 occupies.

### Within Each Story

- Red before green: T003–T008 before T009; T018–T020 before T021; T026 before T027–T028
- `store.rs` before `commands.rs` in US2 and US3 — the pure layer defines the type, the shell branches on it
- Each story's gate task (T017, T025, T037) closes it

### Parallel Opportunities

**There are almost none, and that is a property of the feature rather than an oversight.** The
whole change lives in five Rust files, and within each story most tasks append to the *same* file
— `lock.rs` for US1's tests, `store.rs` for US2's and US3's. Under the "[P] means different files"
rule, that disqualifies nearly everything.

What is genuinely parallel:

- **Across stories, with one caveat**: US1, US2, and US3 are logically independent and could be
  built by three people at once. They collide on `commands.rs` (US1 rewrites its lock sites, US2
  rewrites `add_site`, US3 rewrites `update_site`) and on `store.rs` (US2 and US3), so the cost is
  merge conflict resolution in two files, not redesign
- **Within US3**: T029–T033 are five independent `#[test]` functions with no shared fixture beyond
  the existing `a_site` helper. They append to one file, so they are not marked `[P]`, but they can
  be written in any order and by anyone once T027 exists
- **T038 and T039** are read-only diff inspections and can run alongside T040

Sequential-by-nature and worth naming: T003→T009 (stub, then red tests, then the real
implementation) and T018→T021 (type without the distinction, then the failing assertion, then the
distinction). Both are the FR-017 red-then-green discipline, and collapsing either into one step
destroys the evidence the spec asks for.

---

## Implementation Strategy

### MVP (User Story 1 only)

1. Phase 1 — Setup (T001–T002)
2. Phase 3 — US1 (T003–T017)
3. **STOP and VALIDATE**: poison a lock from a test, confirm recovery; confirm `grep -rn 'lock()\.unwrap()' src-tauri/src/` is empty; run the three gates
4. Shippable here. This is the widest-blast-radius item of the three — the only one whose failure
   mode is "the whole app stops accepting input" — and it stands alone

### Incremental Delivery

1. Setup → US1 → gates green → **MVP**
2. + US2 → gates green → a refused add can no longer produce a ghost row
3. + US3 → gates green → overlapping edits cannot discard each other
4. + Phase 6 → roadmap drained in the primary checkout, full merge bar, ready to ship

Each increment leaves the tree merge-ready. None of the three depends on a later one.

### If time runs short

US3 is the one to defer, and the spec says why: the roadmap already downgraded it, and the only
route to it from the shipped window — a fast double-submit on one row — was closed by
`002-robustness`'s in-flight submit guard. It remains real one layer down, so deferring it means
the roadmap keeps that item rather than retiring it. **Do not** defer US1: it is the only one of
the three whose failure mode is total loss of function.

---

## Notes

- `[P]` = different files, no dependencies. This feature has very few — see Parallel Opportunities
- Every task's file path is relative to the worktree root **except T041**, which is explicitly in
  the primary checkout
- Commit after each task or logical group; the `after_implement` hook chain expects per-task
  journaling
- **Test counts to beat**: Rust 42 → 42 + new (US1 adds ~5, US2 ~1 new plus 2 tightened, US3 ~7);
  frontend 30 → **exactly 30, unmodified**. A frontend count other than 30 means a scope violation
- Two things are verified by eye, not by machine, and both are declared in advance rather than
  discovered at review: `SharedStore::lock`'s emit wiring (research R7 — `tauri`'s `test` feature
  and a mock-app harness were declined for three lines of `emit`) and `add_site`'s refusal-before-
  `engine.start` ordering (needs a Tauri `State`)
- Baseline (T002, re-run in this worktree 2026-08-06): Rust **42 passed / 0 failed** · frontend
  **30 passed / 0 failed** (4 files) · clippy **clean**. Matches the plan's recorded baseline
  exactly, so nothing changed underneath. `pnpm install` was a no-op — `node_modules` was already
  present from the plan-time baseline run
- Final (T043): Rust **55 passed / 0 failed / 0 ignored** · frontend **30 passed / 0 failed**
  (4 files, unmodified — SC-008 holds) · clippy **clean**, both `cargo clippy -- -D warnings`
  and the stricter `--all-targets`. No `#[ignore]`, no skipped frontend test. Rust 42 → 55:
  US1 +5, US2 +1 new plus 2 tightened, US3 +7 (one of which asserts the *old* shape's bug)

### What was verified by eye rather than by machine

Declared in advance by the plan, and confirmed here:

1. **`SharedStore::lock`'s emit wiring.** Read at T040: the `recovered` flag from `recover`
   gates a single `self.warn(...)` whose message says the saved list may not reflect the most
   recent change (FR-004). The three silent sites — `engine.rs`'s two `tasks` locks and
   `get_warning`'s `warning` lock — were read and confirmed to call `lock::recover(..).0` and
   emit nothing (FR-003, FR-005).
2. **`add_site`'s refusal ordering.** Read at T025: the `AddError::DuplicateId` arm returns
   `Err` above `state.engine.start(...)`, so no timer is created (C1), and raises no
   `store-warning` (C2). The message contains no form of the word "saved" (C3) — the only
   occurrences in `commands.rs` are comments describing the *write-failure* path.

### Not done, and not claimable

**The GUI click-through in T025 step 1 and T040 (add, edit, and delete a site in the running
window) was not performed.** This session is non-interactive and cannot drive the window. What
*was* verified by launching `pnpm tauri dev`: the app compiles, launches, loads the site list,
and runs with no panic — which is the smoke test the `SharedStore` rewrite most needed, since
it touched every store access including the startup one. Afterwards the live `sites.json` was
byte-identical to a pre-launch safety copy, with no staging artifact left beside it.

The three manual UI actions remain genuinely unverified and should be run by hand before
merge. They are the FR-007 "nothing else changed" check, not a check of any new behaviour.
