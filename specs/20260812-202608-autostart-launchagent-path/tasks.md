# Tasks: Launch-at-login survives upgrades

**Feature**: `20260812-202608-autostart-launchagent-path` ·
**Issue**: [#25](https://github.com/clintcparker/site-checker/issues/25)

**Input**: Design documents from `specs/20260812-202608-autostart-launchagent-path/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/launch-agent-plist.md](./contracts/launch-agent-plist.md),
[quickstart.md](./quickstart.md)

**Tests**: INCLUDED. Not because the spec asked in so many words, but because the plan's Constitution
Check commits to it — Principle IV ("Testable Core, Thin Shell") makes path derivation, plist
parsing, and the repair decision pure functions "under `cargo test`", and [quickstart.md](./quickstart.md) §1
enumerates the exact cases. Every test task below maps to a row of that table.

**Organization**: Grouped by user story. Phases 1–2 make the app compile and behave *exactly as it
does today* on our own `AutoLaunch` instead of the plugin; US1 then changes the recorded path, US2
adds repair, US3 documents removal.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Paths are relative to the repository root (the worktree checkout).

## Path Conventions

Single-project Tauri layout, per plan.md → Project Structure:

- Backend: `src-tauri/src/`, manifest `src-tauri/Cargo.toml`
- Frontend: `src/` — **not touched by this feature**
- Living specs: `capabilities/backend/spec.md`
- Docs: `README.md`, `install/homebrew/site-checker.rb`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Swap the dependency and create the module the rest of the feature lives in.

> This phase intentionally leaves the tree **not compiling** — removing `tauri-plugin-autostart`
> breaks `lib.rs:11,20-23` and `commands.rs:136,139-155` until Phase 2 rewires them. Phase 1 and
> Phase 2 are one commit's worth of work; do not stop between them.

- [ ] T001 Remove `tauri-plugin-autostart = "2.5"` and add `auto-launch = "0.5"` to the `[dependencies]` table in `src-tauri/Cargo.toml` (research D1 — net dependency count is unchanged; `auto-launch` was already in the tree transitively)
- [ ] T002 Run `cargo check --locked --manifest-path src-tauri/Cargo.toml` to refresh `src-tauri/Cargo.lock`, and confirm the lockfile diff drops `tauri-plugin-autostart` while keeping `auto-launch` at the version already resolved
- [ ] T003 Create `src-tauri/src/autostart.rs` with a module doc comment stating that it owns the launch-at-login registration, and add `mod autostart;` to the module list at the top of `src-tauri/src/lib.rs` (lines 1-6, alphabetical order — before `mod check;`)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Own the `AutoLaunch` instead of the plugin, with **zero behaviour change** — still
registering `current_exe().canonicalize()`, exactly the path the plugin recorded. This is the seam
US1 and US2 both build on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [ ] T004 Implement `pub fn manager(app: &tauri::AppHandle) -> Result<auto_launch::AutoLaunch, Box<dyn std::error::Error>>` in `src-tauri/src/autostart.rs` using `AutoLaunchBuilder` with `app_name = app.package_info().name` (**must** stay `package_info().name` — research D1's compatibility note: any other name orphans existing users' `~/Library/LaunchAgents/Site Checker.plist` instead of repairing it), `use_launch_agent(true)`, `args(&[] as &[&str])`, and `app_path = std::env::current_exe()?.canonicalize()?` — the running path, unchanged from today. US1 replaces only this last argument.
- [ ] T005 In `src-tauri/src/lib.rs`, delete `use tauri_plugin_autostart::{MacosLauncher, ManagerExt};` (line 11) and the `.plugin(tauri_plugin_autostart::init(...))` call on the builder (lines 20-23); at the top of the `setup` closure build `autostart::manager(app.handle())` and `app.manage(...)` it, so the `AutoLaunch` is retrievable as managed state before any command can run
- [ ] T006 In `src-tauri/src/lib.rs`, change the first-run marker block (lines 45-62) to call `enable()` on the managed `AutoLaunch` instead of `app.autolaunch()`, leaving the marker path, the write-even-on-failure rule, the warning text, and the `get_or_insert_with` precedence over the store warning **byte-for-byte as they are**
- [ ] T007 In `src-tauri/src/commands.rs`, delete `use tauri_plugin_autostart::ManagerExt;` (line 136) and change `get_autostart` (line 139) and `set_autostart` (line 146) to read `app.state::<auto_launch::AutoLaunch>()` instead of `app.autolaunch()`, keeping both names, both argument lists, both `Result<bool, String>` return types, and `set_autostart`'s trailing `is_enabled()` call so the checkbox can still correct itself (contract: Tauri command surface — unchanged)
- [ ] T008 Verify the seam is behaviour-neutral: `cargo test --locked --manifest-path src-tauri/Cargo.toml` and `cargo clippy --locked --manifest-path src-tauri/Cargo.toml -- -D warnings` are green, and `grep -rn "tauri_plugin_autostart\|tauri-plugin-autostart" src-tauri/ src/ package.json` returns nothing
- [ ] T009 Confirm `src-tauri/capabilities/default.json` still grants `core:default` only and needs no edit — the plugin's JS commands were never permitted (research D1), so nothing is added or removed here. This task is a check, not a change; record the result rather than editing the file.

**Checkpoint**: The app compiles, starts, ticks and unticks the box, and writes the same plist it
wrote before — on our own code. Nothing user-visible has changed yet.

---

## Phase 3: User Story 1 - Launch at login keeps working after an upgrade (Priority: P1) 🎯 MVP

**Goal**: Register the version-independent `…/opt/<formula>/…` path whenever the running copy has
one, and today's exact path whenever it does not. This is the defect in the issue.

**Independent Test**: Enable launch at login on an installed copy, upgrade so the previous install
directory is removed, and confirm the recorded login item still resolves to a working copy.
Automatically: the derivation tests below decide the same question without a Homebrew install.

### Tests for User Story 1 ⚠️

> Write these first and watch them fail — `stable_path` and `desired_path` do not exist yet.

- [ ] T010 [P] [US1] Add `#[cfg(test)]` tests for `stable_path` in `src-tauri/src/autostart.rs` covering FR-001/FR-002: `/opt/homebrew/Cellar/site-checker/1.0.0/libexec/Site Checker.app/Contents/MacOS/site-checker` → `/opt/homebrew/opt/site-checker/libexec/…`, the same for a `/usr/local` prefix, and the same for a relocated prefix such as `/Users/x/brew`
- [ ] T011 [P] [US1] Add `stable_path` negative tests in `src-tauri/src/autostart.rs` covering FR-004: no `Cellar` component at all (`/Applications/Site Checker.app/…`, `src-tauri/target/debug/…`) → `None`; and a truncated shape (`Cellar` with no version, or nothing after the version) → `None`
- [ ] T012 [P] [US1] Add a `stable_path` test in `src-tauri/src/autostart.rs` for the "last `Cellar` wins" rule from data-model.md derivation rule 1: a path containing more than one `Cellar` component splits on the last one
- [ ] T013 [US1] Add `tempfile`-backed `desired_path` tests in `src-tauri/src/autostart.rs` (following the `tempfile` pattern already used in `src-tauri/src/store.rs`) covering FR-003: derived path that does not exist → running path; derived path that exists but canonicalises to a *different* file → running path; derived path that exists and canonicalises back to the running path → the derived path. These three are the whole of data-model.md rule 4 and the "unrelated `Cellar` directory" edge case.

### Implementation for User Story 1

- [ ] T014 [US1] Implement `fn stable_path(running: &Path) -> Option<PathBuf>` in `src-tauri/src/autostart.rs`: split into components, find the **last** `Cellar`, require `<formula>` and `<version>` plus a non-empty remainder after them, and rebuild as `<prefix>/opt/<formula>/<remainder>` (data-model.md rules 1-3). Pure — no filesystem access.
- [ ] T015 [US1] Implement `fn desired_path(running: &Path) -> PathBuf` in `src-tauri/src/autostart.rs`: return `stable_path(running)` only if it exists **and** `canonicalize()`s to `running`; otherwise return `running` (data-model.md rule 4 = FR-001–FR-004 in one place)
- [ ] T016 [US1] Change `manager()` in `src-tauri/src/autostart.rs` (from T004) to pass `desired_path(&current_exe()?.canonicalize()?)` as `app_path` instead of the running path directly
- [ ] T017 [US1] Run `cargo test --locked --manifest-path src-tauri/Cargo.toml` and `cargo clippy --locked --manifest-path src-tauri/Cargo.toml -- -D warnings`; confirm green

**Checkpoint**: US1 is complete and independently shippable. A fresh install records the `opt` path;
a hand-built copy and a dev build record exactly what they recorded before (SC-004). Nothing repairs
an existing registration yet — that is US2.

---

## Phase 4: User Story 2 - An already-broken login item repairs itself (Priority: P2)

**Goal**: On every start, rewrite an existing registration that names the wrong path — never create
one, never delete one, never warn.

**Independent Test**: Point an existing registration at a stale version-numbered path, launch Site
Checker, and confirm the file now names the desired path and still exists.

**Dependency note**: US2 shares `src-tauri/src/autostart.rs` and `src-tauri/src/lib.rs` with US1, so
the two stories are *logically* independent (US2's reader and decision functions do not call US1's
derivation) but cannot be edited concurrently by two people without conflict. Sequence them.

### Tests for User Story 2 ⚠️

- [ ] T018 [P] [US2] Add `recorded_path` positive tests in `src-tauri/src/autostart.rs`: the exact `auto-launch` 0.5 template from `contracts/launch-agent-plist.md` yields `ProgramArguments[0]`, including a path containing spaces (`…/Site Checker.app/…`). Assert against the template verbatim so a future `auto-launch` template change fails the test instead of silently disabling repair (plan.md Risks, last row).
- [ ] T019 [P] [US2] Add `recorded_path` negative tests in `src-tauri/src/autostart.rs` covering FR-007: empty file, truncated XML, non-UTF-8/binary bytes, a plist with no `ProgramArguments` key, and `ProgramArguments` with an empty `<array>` → all `None`
- [ ] T020 [P] [US2] Add `needs_repair` tests in `src-tauri/src/autostart.rs`: recorded ≠ desired → `true` (FR-005); recorded == desired → `false` (FR-005, third scenario); `None` → `false`, covering both "no registration at all" (FR-006) and "unreadable" (FR-007), which collapse deliberately (data-model.md → Internal values)

### Implementation for User Story 2

- [ ] T021 [P] [US2] Implement `fn recorded_path(plist: &str) -> Option<String>` in `src-tauri/src/autostart.rs`: a small pure string scan for the first `<string>…</string>` inside the `<array>` that follows the `ProgramArguments` key; any deviation from that shape returns `None` (research R3 — no `plist` crate, no `PlistBuddy`)
- [ ] T022 [P] [US2] Implement `fn needs_repair(plist: Option<&str>, desired: &str) -> bool` in `src-tauri/src/autostart.rs`: `true` only when a path was read **and** differs by exact string equality; `None` → `false` (plan.md Judgment Call 4 — exact equality, not canonicalised comparison, so a genuinely dead path is not hidden behind a failed `canonicalize`)
- [ ] T023 [US2] Implement `pub fn repair_if_stale(manager: &AutoLaunch, plist_path: &Path)` in `src-tauri/src/autostart.rs`: `read_to_string` the file, call `needs_repair`, and on `true` call `manager.enable()` — which truncates and rewrites the same file. Every step swallows its own error and returns `()`; the function has no failure mode that can escape (FR-008, I4). Add a comment naming FR-006/I1: the absent-file branch does nothing, because `read_to_string` failing yields `None` yields `false`.
- [ ] T024 [US2] In `src-tauri/src/lib.rs`, call `autostart::repair_if_stale(...)` **after** the first-run marker block (line 62) and **before** `app.manage(AppState { … })` (line 64), passing `~/Library/LaunchAgents/{package_info().name}.plist`. Add a comment recording the ordering guarantee from plan.md: on a genuine first run `enable()` has already written the desired path, so repair finds nothing to do.
- [ ] T025 [US2] Confirm the repair cannot reach the store: check by reading `src-tauri/src/lib.rs` that the site list was loaded and the engine started *above* the touched block, and that nothing in `autostart.rs` references `store`, `lock`, or `AppState` (`grep -n "store\|lock\|AppState" src-tauri/src/autostart.rs` returns nothing). FR-008 / SC-006 / Constitution Principle II.
- [ ] T026 [US2] Run `cargo test --locked --manifest-path src-tauri/Cargo.toml` and `cargo clippy --locked --manifest-path src-tauri/Cargo.toml -- -D warnings`; confirm green

**Checkpoint**: US1 and US2 both work. A stale registration corrects itself on the next manual
launch; an absent one stays absent; an unreadable one is left byte-for-byte alone.

---

## Phase 5: User Story 3 - Removing Site Checker leaves nothing behind (Priority: P3)

**Goal**: Both removal surfaces name the login item. Documentation only — no code.

**Independent Test**: `grep -n "LaunchAgents" README.md install/homebrew/site-checker.rb` names the
removal command in both; then follow the README's `### Uninstall` end to end and confirm nothing
under `~/Library/LaunchAgents` still references Site Checker.

**Dependency note**: US3 touches neither `src-tauri/` file, so it genuinely can be done in parallel
with (or before) US1 and US2 by a second person.

- [ ] T027 [P] [US3] Add `rm ~/Library/LaunchAgents/"Site Checker.plist"` to the `### Uninstall` shell block in `README.md` (lines 58-61), directly after the existing `/Applications` symlink line, with a short note that it is the launch-at-login registration (FR-010)
- [ ] T028 [P] [US3] Add the same removal line to `def caveats` in `install/homebrew/site-checker.rb` (lines 108-128), placed next to the existing `ln -s … /Applications/` guidance so the two hand-managed files are described together (FR-011). This file is a template the release workflow renders into `clintcparker/homebrew-tap`, which is what `brew install` prints.
- [ ] T029 [US3] Confirm the two surfaces agree word-for-word on the command: `grep -n "LaunchAgents" README.md install/homebrew/site-checker.rb` shows the identical `rm ~/Library/LaunchAgents/"Site Checker.plist"` in both (quickstart.md §5)

**Checkpoint**: All three user stories are independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T030 [P] Update the "Launch-at-login is on by default, but turning it off sticks" section of `capabilities/backend/spec.md` (around line 270): the existing first-run/opt-out behaviour is unchanged, so **add** the new behaviour — the registration names a location that survives an upgrade, and a registration naming some other location is rewritten on start without being created or removed — rather than rewriting what is there. Living-specs config maps `src-tauri/src/**` to the `backend` capability (`living-specs.yml`).
- [ ] T031 [P] Verify the frontend is untouched: `git diff --stat origin/main -- src/ index.html package.json` is empty, and `pnpm test` passes with the same test count as on `main` (spec assumption: no user-visible UI change; Constitution Principle V)
- [ ] T032 Run the full quickstart §1 gate: `cargo test --locked --manifest-path src-tauri/Cargo.toml`, `cargo clippy --locked --manifest-path src-tauri/Cargo.toml -- -D warnings`, `pnpm test` — all green
- [ ] T033 Run quickstart.md §2, §2a-§2d by hand against `pnpm tauri dev`. **Back up `~/Library/LaunchAgents/"Site Checker.plist"` to `/tmp` first and restore it afterwards** — these steps rewrite your real login item. Record the observed result for each of §2a (repair never creates), §2b (stale repairs), §2c (unreadable left alone), §2d (unwritable directory cannot block startup).
- [ ] T034 Record in the PR description that quickstart.md §3 (a real `brew install`, FR-001 end to end) and §4 (an actual `brew upgrade` across two builds, SC-001) are **not run** by this branch — they need a released bottle containing this change, which does not exist until it merges. Spec assumption "Verification of the upgrade path requires two real builds" says so explicitly; naming it in the PR is what keeps the gap honest rather than silent.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: no dependencies — start immediately. Leaves the tree non-compiling by design.
- **Phase 2 (Foundational)**: depends on Phase 1. **Blocks US1 and US2.** Restores a compiling,
  behaviour-identical app on our own `AutoLaunch`.
- **Phase 3 (US1, P1)**: depends on Phase 2. Independent of US2 and US3.
- **Phase 4 (US2, P2)**: depends on Phase 2. Logically independent of US1 — `recorded_path` and
  `needs_repair` never call `desired_path`; only the shell in T023/T024 uses both. Shares files with
  US1, so sequence rather than parallelise.
- **Phase 5 (US3, P3)**: depends on nothing. Can be done first, last, or concurrently.
- **Phase 6 (Polish)**: depends on every story you intend to ship.

### User Story Dependencies

- **US1 (P1)**: after Phase 2. No dependency on US2 or US3.
- **US2 (P2)**: after Phase 2. Delivers value on its own (it would repair a registration to the
  running path even without US1's derivation), but the *point* of repairing is to reach the path US1
  derives — so shipping US2 without US1 is coherent yet near-pointless. Ship in priority order.
- **US3 (P3)**: no dependency on anything in this feature.

### Within Each User Story

- Tests are written first and must fail before the implementation task that satisfies them.
- Pure functions before the shells that call them (`stable_path` → `desired_path` → `manager`;
  `recorded_path` → `needs_repair` → `repair_if_stale` → `lib.rs` wiring).
- `cargo clippy -- -D warnings` green before the story is called done.

### Parallel Opportunities

- **T010, T011, T012** — three independent `stable_path` test cases, same file, no interdependence.
- **T018, T019, T020** — the reader and decision tests, likewise.
- **T021, T022** — `recorded_path` and `needs_repair` are separate pure functions.
- **T027, T028** — the two documentation surfaces are different files entirely.
- **T030, T031** — living spec versus frontend verification.
- **US3 (Phase 5) in full**, against any other phase.

Honest caveat: every `[P]` inside Phase 3 and Phase 4 lands in the same file
(`src-tauri/src/autostart.rs`). They are parallel in the sense that no one blocks another's
reasoning, not in the sense that two agents can write them simultaneously without conflict.

---

## Parallel Example: User Story 1

```bash
# The three derivation test groups are independent of each other:
Task: "T010 stable_path happy-path tests for /opt/homebrew, /usr/local, relocated prefix"
Task: "T011 stable_path negative tests — no Cellar component, truncated shape"
Task: "T012 stable_path test — last Cellar component wins"

# Then, sequentially: T014 (stable_path) → T015 (desired_path) → T016 (manager) → T017 (gate)
```

## Parallel Example: User Story 3

```bash
# Different files, no shared state:
Task: "T027 README.md ### Uninstall gains the LaunchAgent removal line"
Task: "T028 install/homebrew/site-checker.rb caveats gain the same line"
```

---

## Implementation Strategy

### MVP (US1 only)

1. Phase 1 → Phase 2 as one unit (the tree does not compile between them).
2. Phase 3 (US1).
3. **STOP and VALIDATE**: `cargo test`, `cargo clippy`, `pnpm test`; quickstart §2 confirms a dev
   build still records `target/debug/…` unchanged.
4. This alone closes the defect for every install created after it ships.

### Incremental Delivery

1. Setup + Foundational → the app runs on our own `AutoLaunch`, behaviour identical.
2. \+ US1 → new installs record the upgrade-proof path. **MVP.**
3. \+ US2 → installs that are *already* broken fix themselves on next launch.
4. \+ US3 → removal is documented in both places a user might look.

Each step is independently valuable and independently revertable. US2 in particular is one commit
that can be dropped wholesale if review rejects Open Decision 1 (below).

### Parallel Team Strategy

One developer takes Phases 1→2→3→4 in sequence (they share two files). A second developer can take
Phase 5 at any time. There is no third stream worth opening.

---

## Judgment Calls Made in This Step (unattended run — for the PR to surface)

No user was present. These are decisions taken *here*, in addition to the three Open Decisions in
[spec.md](./spec.md) and the four in [plan.md](./plan.md), all of which the ship step should carry
into the PR description.

1. **Test tasks are included even though the spec never says "write tests".** The plan's Constitution
   Check commits to pure functions under `cargo test` (Principle IV) and quickstart §1 lists the
   cases; generating the phases without them would contradict both. If review disagrees, T010-T013
   and T018-T020 are the tasks to drop.
2. **Phase 2 exists as a separate, behaviour-neutral phase** rather than folding the plugin swap into
   US1. It makes the risky part (replacing the autostart implementation) verifiable on its own — the
   app must behave identically before any path changes — at the cost of one phase that delivers no
   user-visible value. The alternative, one big US1, makes a regression in the swap indistinguishable
   from a regression in the derivation.
3. **Phase 1 deliberately leaves the tree non-compiling.** Removing the plugin dependency breaks
   `lib.rs` and `commands.rs` until T005-T007. Splitting it this way keeps "change the manifest" and
   "rewire the callers" as separate reviewable steps; the phase header warns not to stop between
   them.
4. **T009 and T025 are verification tasks that change no file.** They record that
   `capabilities/default.json` needs no edit and that the repair path cannot reach the store — both
   are claims the plan makes and review will otherwise have to re-derive. Included as tasks so the
   evidence is produced rather than assumed.
5. **The unrunnable verification (quickstart §3, §4) is a task (T034), not an omission.** It cannot
   be executed on this branch — it needs a released bottle containing the change. T034 makes the
   implementer state that in the PR rather than let the checklist imply full coverage.

---

## Notes

- `[P]` = no ordering dependency; see the caveat above about same-file `[P]` tasks.
- The plist filename and `Label` both derive from `package_info().name` and **must not change** — a
  different name orphans every existing user's registration instead of repairing it.
- Commit after each phase; each phase boundary is a green `cargo test` + `cargo clippy` (except the
  Phase 1 → Phase 2 boundary, which has none by design).
- `src/`, `index.html`, and `src/api.ts` are not edited by any task in this file. If a task makes you
  want to, something has gone wrong.

## Task Summary

| Phase | Story | Tasks | Count |
|---|---|---|---|
| 1 — Setup | — | T001-T003 | 3 |
| 2 — Foundational | — | T004-T009 | 6 |
| 3 — Launch at login survives upgrade | US1 (P1) | T010-T017 | 8 (4 test, 4 impl) |
| 4 — Stale registration repairs itself | US2 (P2) | T018-T026 | 9 (3 test, 6 impl) |
| 5 — Removal leaves nothing behind | US3 (P3) | T027-T029 | 3 |
| 6 — Polish | — | T030-T034 | 5 |
| **Total** | | | **34** |
