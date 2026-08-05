---
description: "Task list for Scaffold Cleanup (ROADMAP section 1)"
---

# Tasks: Scaffold Cleanup

**Input**: Design documents from `/specs/001-scaffold-cleanup/`

**Prerequisites**: plan.md, spec.md

**Tests**: No test tasks. The spec requests none, no logic changes, and asserting on
cosmetic metadata would pin the wrong thing. The existing suite (29 Rust + 12 frontend)
plus `cargo clippy -- -D warnings` is the regression net, run as a gate after every story.

**Organization**: Grouped by user story so each is independently implementable, testable,
and revertable.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3, US4)
- Exact file paths are included in every task

## Path Conventions

Desktop app: Rust backend in `src-tauri/`, vanilla-TS frontend in `src/`, manifests at
repository root. Paths below are repo-relative.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Establish an isolated workspace and a known-green baseline, so any later
failure is attributable to this feature's edits rather than pre-existing drift.

- [X] T001 Create a worktree and branch for `001-scaffold-cleanup` (run `/speckit-worktrees-create`, or `git worktree add`). Because `docs/` is gitignored, copy `docs/ROADMAP.md` into the new worktree afterward — T017 needs it.
- [X] T002 Record the baseline: run `cargo test` (in `src-tauri/`), `pnpm test`, and `cargo clippy -- -D warnings`, and confirm all three are green before any edit. If any is red, stop and report — do not start the cleanup on a red tree.
- [X] T003 [P] Capture the pre-change bundle facts for later comparison: note `productName` and `identifier` from `src-tauri/tauri.conf.json` (expected `Site Checker` / `com.clintparker.site-checker`) and list `dist/` contents after a `pnpm build`.

**Checkpoint**: Isolated workspace, green baseline, recorded reference values.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Confirm the two "this is dead code" claims that US1 and US2 rest on. If
either is false, the corresponding story changes shape.

**⚠️ CRITICAL**: US1 must not begin until T004 confirms the opener plugin is unreferenced.

- [X] T004 Verify the opener plugin is truly unregistered: confirm `src-tauri/src/lib.rs` initializes only `tauri_plugin_autostart` and that `grep -rn "opener" src/ src-tauri/src/` returns nothing. If any source file references it, halt US1 and report.
- [X] T005 [P] Verify the three SVGs are unreferenced: confirm `grep -rn "tauri.svg\|typescript.svg\|vite.svg\|assets/" src/ index.html` returns nothing. If any is referenced, halt US2 and report.

**Checkpoint**: Both deletion claims verified against the tree. User story work can begin.

---

## Phase 3: User Story 1 - Remove the dead opener plugin (Priority: P1) 🎯 MVP

**Goal**: The build no longer declares, compiles, or grants a capability for a plugin
that is never registered — eliminating both unused permission surface and dead compiled
code from the shipped binary.

**Independent Test**: After T006–T010, `grep -rn "opener" package.json src-tauri/Cargo.toml src-tauri/capabilities/ src/ src-tauri/src/`
returns nothing; `cargo build`, `pnpm build`, `cargo test`, and `pnpm test` all pass; and
the app launches, lists sites, and checks them exactly as before.

### Implementation for User Story 1

- [X] T006 [P] [US1] Remove the `"opener:default"` entry from the `permissions` array in `src-tauri/capabilities/default.json`, leaving `"core:default"` as the only permission. Keep the JSON valid (no trailing comma).
- [X] T007 [P] [US1] Remove the `"@tauri-apps/plugin-opener": "^2"` line from `dependencies` in `package.json`, leaving `@tauri-apps/api` as the only runtime dependency.
- [X] T008 [P] [US1] Remove the `tauri-plugin-opener = "2"` line from `[dependencies]` in `src-tauri/Cargo.toml`.
- [X] T009 [US1] Regenerate `pnpm-lock.yaml` by running `pnpm install` at the repo root (depends on T007), and commit the updated lockfile so a clean checkout does not reinstall the removed dependency.
- [X] T010 [US1] Regenerate `src-tauri/Cargo.lock` by running `cargo build` in `src-tauri/` (depends on T008), confirming the build succeeds with no unresolved-crate error, and commit the updated lockfile.
- [X] T011 [US1] Run the quality gates — `cargo test`, `pnpm test`, `cargo clippy -- -D warnings` — and confirm all three are green.

**Checkpoint**: US1 complete and independently verifiable. The app is functionally
identical with less surface. Safe to stop here and ship.

---

## Phase 4: User Story 2 - Delete orphaned scaffold assets (Priority: P2)

**Goal**: `src/assets/` contains only assets the app actually uses, so nobody reading
the tree assumes the Tauri/Vite/TypeScript logos are part of the UI.

**Independent Test**: `src/assets/` no longer contains the three SVGs, `pnpm build`
succeeds, `dist/` matches the T003 reference listing, and the app's UI is visually
unchanged on launch.

### Implementation for User Story 2

- [X] T012 [P] [US2] Delete `src/assets/tauri.svg`, `src/assets/typescript.svg`, and `src/assets/vite.svg`. If `src/assets/` is then empty, remove the directory too.
- [X] T013 [US2] Run `pnpm build` and `pnpm test`, confirm both pass, and compare the emitted `dist/` listing against the T003 reference — it should be unchanged, since Vite already excluded these files from the bundle.

**Checkpoint**: US1 and US2 both complete and independently verifiable.

---

## Phase 5: User Story 3 - Fix scaffold identity metadata (Priority: P2)

**Goal**: The npm package and Cargo crate identify themselves as Site Checker with a
real description and author, instead of the scaffold's `tauri-app` / `"A Tauri App"` /
`authors = ["you"]`.

**⚠️ Build-breaking ripple**: `src-tauri/Cargo.toml` sets `[lib] name = "tauri_app_lib"`
and `src-tauri/src/main.rs` calls `tauri_app_lib::run()`. T015 and T016 must land
together — renaming either alone fails to compile.

**Conflict note**: T014 edits `package.json` and T015 edits `src-tauri/Cargo.toml`, both
also touched by US1. Do not run this phase concurrently with Phase 3.

**Independent Test**: `grep -rn 'tauri-app\|tauri_app\|A Tauri App' package.json src-tauri/Cargo.toml src-tauri/src/`
returns nothing; `cargo build`, `cargo test`, `cargo clippy -- -D warnings`, and
`pnpm build` all pass; and `pnpm tauri build` still produces `Site Checker.app` with
identifier `com.clintparker.site-checker`.

### Implementation for User Story 3

- [X] T014 [P] [US3] Change `"name": "tauri-app"` to `"name": "site-checker"` in `package.json`. Leave `version`, `private`, `type`, and all scripts untouched.
- [X] T015 [US3] In `src-tauri/Cargo.toml`, set `name = "site-checker"`, `description` to a real one-line description of the app (e.g. "A small macOS menu-less dashboard that checks whether your sites are up"), and `authors = ["Clint Parker <me@clintparker.com>"]`. In the same edit, rename `[lib] name = "tauri_app_lib"` to `name = "site_checker_lib"`.
- [X] T016 [US3] Update `src-tauri/src/main.rs` to call `site_checker_lib::run()` instead of `tauri_app_lib::run()` (must land with T015).
- [X] T017 [US3] Run `cargo build` in `src-tauri/` and confirm it compiles with the new crate and lib names; commit the regenerated `src-tauri/Cargo.lock`.
- [X] T018 [US3] Run `pnpm install` to refresh `pnpm-lock.yaml` for the renamed npm package (depends on T014) and commit it.
- [X] T019 [US3] Run the quality gates — `cargo test`, `pnpm test`, `cargo clippy -- -D warnings` — and confirm all three are green.
- [X] T020 [US3] Run `pnpm tauri build` and confirm the bundle is still `Site Checker.app` with identifier `com.clintparker.site-checker`. Per ROADMAP section 5, the DMG layout step may hang on `osascript` in a headless session — the `.app` completing is sufficient evidence; interrupt the DMG step and note it if so.

**Checkpoint**: US1–US3 complete. The repo no longer identifies itself as the scaffold.

---

## Phase 6: User Story 4 - Make two strings say what they mean (Priority: P3)

**Goal**: The `has_leading_scheme` doc comment states the rule its body applies, and the
corrupt-file warning is distinguishable from the unreadable-file warning.

**Independent Test**: `cargo test` and `cargo clippy -- -D warnings` stay green,
including `corrupt_file_yields_an_empty_list_a_warning_and_is_left_on_disk` in
`src-tauri/src/store.rs`.

### Implementation for User Story 4

- [X] T021 [P] [US4] Extend the `has_leading_scheme` doc comment in `src-tauri/src/model.rs` (above line 48) to state the character-class rule the body applies: the text before `://` must be entirely ASCII alphanumeric or one of `+`, `-`, `.`. Keep the existing explanation of why a bare `contains("://")` is wrong.
- [X] T022 [P] [US4] Reword the parse-error warning in `src-tauri/src/store.rs::load` (currently `"sites.json could not be read ({e}). Starting with an empty list; the existing file has been left alone."`) so it names the actual cause — the file is not valid JSON — and no longer reads like the I/O-error message on line 33. Keep the reassurance that the file was left on disk.
- [X] T023 [US4] Run `cargo test` and confirm `corrupt_file_yields_an_empty_list_a_warning_and_is_left_on_disk` still passes; if it asserts on the old exact wording, update the assertion to match the new message.
- [X] T024 [US4] Run the quality gates — `cargo test`, `pnpm test`, `cargo clippy -- -D warnings` — and confirm all three are green.

**Checkpoint**: All four user stories complete and independently verifiable.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Close the loop so the roadmap stays honest and the feature is shippable.

- [X] T025 Verify every success criterion from spec.md: run the SC-002 grep (`opener`) and SC-003 grep (`tauri-app|tauri_app|A Tauri App`) and confirm both return no matches; confirm `src/assets/` has no unreferenced file (SC-004).
- [X] T026 [P] Drain section 1 of `docs/ROADMAP.md`: remove each completed item, and for anything deliberately not done, leave it in place with a one-line reason rather than deleting it silently. If the section ends up empty, remove the heading and renumber the remaining sections.
- [X] T027 [P] Check `README.md` for any reference to the old package name, the opener plugin, or the deleted assets, and update it if found.
- [X] T028 Launch the built app and confirm behavior is unchanged: sites list, checks run, status dots update, the autostart checkbox works. This is the FR-007 check — no runtime behavior changed except the corrupt-file banner wording.
- [ ] T029 Run the full gate one final time — `cargo test`, `pnpm test`, `cargo clippy -- -D warnings` — and open the PR against `main` (or run `/speckit-ship-run`). **Gate run and green; PR not opened — awaiting the user's go-ahead (`/speckit-ship-run` is an optional hook).**

---

## Implementation Notes

Deviations and caveats from the run, recorded so the task list stays honest:

- **T001** — the worktree and branch (`20260805-150909-cleanup-scaffold-leftovers`)
  already existed from an earlier session, so it was reused rather than created.
  `docs/ROADMAP.md` was already present in it; the `specs/001-scaffold-cleanup/`
  tree was copied across from the primary checkout.
- **T018** — `pnpm install` reported the lockfile already up to date. `pnpm-lock.yaml`
  does not record the root package's own `name`, so the US3 rename produced no
  lockfile delta. Nothing to commit for this task.
- **T020** — run as `pnpm tauri build --bundles app`, which sidesteps the
  `bundle_dmg.sh` / `osascript` hang documented in ROADMAP section 4 rather than
  starting it and interrupting. `Site Checker.app` built and was verified from its
  `Info.plist`: identifier `com.clintparker.site-checker`, name `Site Checker`,
  executable now `site-checker`. Bundle size 15 MB, unchanged.
- **T027** — extended past `README.md` (which was already clean). The US3 crate
  rename renames the dev binary, so the `process "tauri-app"` references in
  `.claude/skills/speckit-screenshots-capture/SKILL.md` and
  `.specify/extensions/screenshots/commands/capture.md` would have polled System
  Events for a process that no longer exists. Both were updated to `site-checker`.
- **T028** — verified without a visual check, which is TCC-blocked in this session
  (System Events returns `-1743 Not authorized`). Evidence gathered instead: the
  bundled app launched clean with empty stderr, registered with WindowServer as
  `Site Checker` / `com.clintparker.site-checker`, and held live outbound sockets
  (the check engine running). `sites.json` was left untouched. **A human should
  still eyeball the UI once before merge.**

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on Setup. T004 blocks US1; T005 blocks US2.
- **User Stories (Phases 3–6)**: All depend on Foundational.
  - **US3 must not run concurrently with US1** — both edit `package.json` and `src-tauri/Cargo.toml`.
  - US2 and US4 touch disjoint files and can run alongside anything.
- **Polish (Phase 7)**: Depends on all stories you intend to ship.

### User Story Dependencies

- **US1 (P1)**: After T004. Independent of all other stories.
- **US2 (P2)**: After T005. Fully independent — no file overlap with any other story.
- **US3 (P2)**: After Foundational, but **serialized against US1** (shared manifests).
- **US4 (P3)**: After Foundational. Fully independent — touches only `model.rs` and `store.rs`.

### Within Each User Story

- Manifest edits before lockfile regeneration (T007→T009, T008→T010, T014→T018).
- T015 and T016 are a single atomic change — the build is broken between them.
- Each story ends with its own quality-gate task before the next begins.

### Parallel Opportunities

- T003 and T005 run in parallel with their phase-mates.
- **Within US1**: T006, T007, T008 are three different files — fully parallel.
- **Within US4**: T021 and T022 are different files — fully parallel.
- **Across stories**: US2 (T012–T013) and US4 (T021–T024) can run concurrently with each
  other and with either US1 or US3.
- **Polish**: T026 and T027 are different files — parallel.

---

## Parallel Example: User Story 1

```bash
# All three opener declarations live in different files — remove together:
Task: "Remove \"opener:default\" from src-tauri/capabilities/default.json"
Task: "Remove @tauri-apps/plugin-opener from package.json dependencies"
Task: "Remove tauri-plugin-opener from [dependencies] in src-tauri/Cargo.toml"

# Then serialize the lockfile regeneration:
Task: "pnpm install  -> commit pnpm-lock.yaml"
Task: "cargo build   -> commit src-tauri/Cargo.lock"
```

## Parallel Example: Independent Stories

```bash
# US2 and US4 share no files with each other or with US1/US3:
Task: "US2: delete the three SVGs in src/assets/"
Task: "US4: extend the has_leading_scheme doc comment in src-tauri/src/model.rs"
Task: "US4: reword the parse-error warning in src-tauri/src/store.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (worktree, green baseline)
2. Complete Phase 2: Foundational (verify both dead-code claims)
3. Complete Phase 3: User Story 1 (remove the opener plugin)
4. **STOP and VALIDATE**: gates green, `opener` grep empty, app runs unchanged
5. Shippable on its own — this is the only item with a functional dimension

### Incremental Delivery

1. Setup + Foundational → verified baseline
2. US1 → validate → ship (MVP: unused permission and dead crate gone)
3. US2 → validate → ship (tree no longer misleading)
4. US3 → validate → ship (repo identifies as Site Checker)
5. US4 → validate → ship (two strings say what they mean)
6. Polish → drain ROADMAP section 1, final gates, PR

Each story is a standalone commit and a standalone revert.

### Parallel Team Strategy

With multiple developers, after Foundational:

- Developer A: US1, then US3 (must be serialized — shared manifests)
- Developer B: US2 and US4 (disjoint from A's files, and from each other)

---

## Notes

- [P] tasks = different files, no dependencies
- The only cross-story file conflict in this feature is US1 ∩ US3 on `package.json` and
  `src-tauri/Cargo.toml` — everything else is disjoint
- T015 + T016 are one atomic change; do not commit between them
- Commit lockfiles alongside the manifest edit that caused them
- The gates are `cargo test`, `pnpm test`, `cargo clippy -- -D warnings` — all three, every story
- `docs/` is gitignored: a fresh worktree will not have `docs/ROADMAP.md` (see T001)
