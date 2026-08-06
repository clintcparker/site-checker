---
description: "Task list for Robustness (ROADMAP section 1)"
---

# Tasks: Robustness (small correctness wins)

**Input**: Section 1 ("Robustness — small correctness wins") of `docs/ROADMAP.md`

**Prerequisites**: None generated. This feature has **no `spec.md` and no `plan.md`** —
the roadmap section is specific enough (named file, named function, named symptom, per-item
effort) to serve as the spec directly. `.specify/memory/constitution.md` v1.0.0 is the
governing document; every story below cites the principle it answers to. If a full
specify → plan cycle is wanted, run `/speckit-specify` against ROADMAP §1 first and
regenerate this file.

**Tests**: Included, but scoped. Unlike `001-scaffold-cleanup` (cosmetic, no tests), three
of these five items change runtime behavior, and Constitution Principle IV ("Testable Core,
Thin Shell") pushes testable logic into tests. `happy-dom` is already the vitest environment
and `src/render.test.ts` is the precedent, so a DOM unit test on `form.ts` is cheap. Test
tasks are marked **(optional)** in each phase heading — dropping them still leaves each
story shippable, and the existing suite (29 Rust + 12 frontend) plus
`cargo clippy -- -D warnings` remains the regression net.

> **Not in scope**: ROADMAP §5's test-coverage backlog (`check.rs` transport-fallback,
> `store.rs` no-op update, `render.ts` empty/front-insert). This feature adds `form.ts`
> coverage only where §1's own changes need it.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1–US5)
- Exact file paths are included in every task

## Path Conventions

Desktop app: vanilla-TS frontend in `src/`, markup at `index.html`, Rust backend in
`src-tauri/`. Paths below are repo-relative. **Every code change in this feature is
frontend** — no Rust source is touched, though `cargo test`/`clippy` still run as gates.

### File contention (why [P] is rare here)

- `src/main.ts` — US1, US2, US5 (three different functions, one file)
- `src/form.ts` — US3, US4
- `index.html` — US4 only
- `src/form.test.ts` — US3, US4 (new file, created in Phase 2)

Only two files carry all five stories, so cross-story parallelism is mostly unavailable.
Marking these `[P]` would be a lie that produces merge conflicts. They are fast (all S)
and sequence cleanly instead.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Isolated workspace and a known-green baseline, so any later red is
attributable to this feature.

- [X] T001 Create a worktree and branch for `002-robustness` (run `/speckit-worktrees-create`, or `git worktree add`). Worktrees fork from `origin/main`, so fast-forward local `main` first; and because `docs/` is gitignored, copy `docs/ROADMAP.md` into the new worktree afterward — T003 and T023 read it.
- [X] T002 Record the baseline: run `cargo test` (from `src-tauri/`), `pnpm test`, and `cargo clippy -- -D warnings` (from `src-tauri/`), and confirm all three are green before any edit. If any is red, stop and report — do not start on a red tree.
- [X] T003 [P] Re-verify the five ROADMAP §1 claims still describe the tree: `src/main.ts` registers `onSiteStatus`/`onStoreWarning` after `await mountAutostart()` and `await getWarning()`; `upsertSite` in `src/main.ts` writes `sites` but not `statuses`; `src/form.ts` has no in-flight guard on submit or delete; `index.html`'s `#site-interval` has `min="10"` but no `max`; `src/main.ts`'s `mountAutostart` uses `querySelector<HTMLInputElement>("#autostart")!`. Any claim that no longer holds — report and drop that story rather than inventing work.

**Checkpoint**: Isolated workspace, green baseline, all five claims confirmed against the tree.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Stand up the one shared artifact (a `form.ts` DOM harness) that US3 and US4
both need, and confirm the one precondition US3 rests on.

**⚠️ CRITICAL**: T004 blocks the optional test tasks in US3 and US4 (T014, T018). It does
not block their implementation tasks. Skip T004 only if skipping both test tasks.

- [X] T004 Create `src/form.test.ts` with a shared harness and no behavioral assertions yet: a `mountFixture()` helper that writes the `#site-form` / `#site-id` / `#site-url` / `#site-label` / `#site-interval` / `#site-submit` / `#site-cancel` / `#site-error` / `#rows` markup (copy the field ids and types from `index.html`) into `document.body`, plus a `vi.mock("./api", ...)` that stubs `addSite`, `updateSite`, `deleteSite` with controllable deferred promises. Follow the local-fixture style of `src/render.test.ts` (no shared test-utils module). Add one smoke test asserting `mountForm` puts the form in Add mode (`#site-submit` reads `Add`, `#site-cancel` is hidden, `#site-interval` is `60`). Confirm `pnpm test` is green.
- [X] T005 [P] Confirm the row-action elements are real `<button>` elements: `button()` in `src/render.ts` returns `HTMLButtonElement` for both `edit` and `delete`. US3's delete guard sets `.disabled` on the clicked element; if these were `<span>`s, US3 needs a different mechanism (a data attribute or a closure flag) and changes shape.

**Checkpoint**: `src/form.test.ts` exists and is green; the delete-guard mechanism is confirmed available.

---

## Phase 3: User Story 1 - No dropped status event during startup (Priority: P1) 🎯 MVP

**Goal**: A `site-status` event emitted during app startup is never dropped. Today
`src/main.ts` registers `onSiteStatus`/`onStoreWarning` only after `await mountAutostart()`
and `await getWarning()`; Tauri events have no replay, so anything emitted in that window
is lost.

**Why P1**: This is the one item the final v1 review flagged as a real — if rare — bug.
Rare because the first check is jittered 0–10 s while the startup IPC calls take
milliseconds, and a dropped event self-heals on the next interval. Still a real hole,
and the fix is a statement reorder.

**Independent Test**: Reorder, then confirm in `src/main.ts` that no `await` precedes the
two `listen` registrations inside `main()`. Launch the app (`pnpm tauri dev`) with at
least one site configured and confirm the first status dot still lands normally and the
store-warning banner still appears for a corrupt `sites.json`. No behavior should change
in the ordinary case — the fix only closes the window.

### Implementation for User Story 1

- [X] T006 [US1] In `src/main.ts`, move the `await onSiteStatus(...)` and `await onStoreWarning(showBanner)` registrations to the top of `main()`, above the `listSites()` loop and above `mountForm`/`mountAutostart`/`getWarning`. Both handlers close over module-level bindings (`statuses`, `repaint`, `showBanner`) that are initialized at module load, before `main()` is called, so hoisting is safe.
- [X] T007 [US1] Add a comment in `src/main.ts` above the hoisted registrations recording *why* they come first: Tauri events have no replay, so any `site-status` emitted before `listen` resolves is gone. Note the benign consequence of the new order — a status can now arrive for an id not yet in `sites`; it lands in the `statuses` map and renders as soon as `listSites()` populates `sites`, because `currentRows()` iterates `sites`, not `statuses`. Keep it to the density of the surrounding comments.
- [X] T008 [US1] Verify: `pnpm build` (runs `tsc`) and `pnpm test` green; launch `pnpm tauri dev` and confirm sites list, status dots arrive, and the autostart checkbox still reflects the OS state.

**Checkpoint**: The startup event window is closed and startup behaves identically otherwise.

---

## Phase 4: User Story 2 - A row returns to Pending when its URL changes (Priority: P2)

**Goal**: Editing a site's URL drops its stale status immediately instead of showing the
old dot until the next check lands. `upsertSite` in `src/main.ts` updates `sites` but
never `statuses`, so changing a good URL to a bad one keeps showing green.

**Why P2**: Directly contradicts what the UI claims. Constitution Principle II makes
Pending the honest UI state for "we have not confirmed this yet" — after a URL change
that is exactly the truth, and the current code says otherwise.

**Independent Test**: Add a site with a reachable URL, wait for a green dot, then edit it
to an unreachable URL and save. The dot must go to Pending immediately (not stay green),
then resolve to down on the next check. Editing only the *label* or the *interval* must
**not** reset the dot — the last confirmed result is still valid for that URL.

### Implementation for User Story 2

- [X] T009 [US2] In `src/main.ts`, change `upsertSite(site)` to read the previous entry (`sites.get(site.id)`) *before* `sites.set(site.id, site)`, and call `statuses.delete(site.id)` when a previous entry exists and `previous.url !== site.url`. Do nothing to `statuses` when there is no previous entry (an Add already has no status) or when the URL is unchanged (a label/interval edit must preserve the dot).
- [X] T010 [US2] Add a comment in `src/main.ts` noting that both sides of the comparison are backend-normalized — `addSite`/`updateSite` return the saved `Site` after `normalize_url`, so comparing `previous.url` to the returned `site.url` will not false-positive on a purely cosmetic difference the backend already collapsed.
- [X] T011 [US2] Verify by hand against the Independent Test above: URL change resets to Pending; label-only and interval-only edits do not. Then `pnpm build` and `pnpm test` green.

**Checkpoint**: "Last confirmed" is honest the instant a URL changes.

---

## Phase 5: User Story 3 - Double-click cannot create or delete twice (Priority: P3)

**Goal**: A fast double-click on Add/Save cannot create two identical sites, and a fast
double-click on a row's Delete cannot fire two deletes. `src/form.ts` awaits its IPC calls
with no in-flight guard.

**Why P3**: Real but low-impact for a single-user tool (Constitution Principle I) — you
have to out-click an IPC round trip. Also the precondition for closing ROADMAP §3's
`update_site` read-modify-write race, which is only reachable via overlapping saves.

**Independent Test**: With the form filled in, double-click Add fast — exactly one row
appears. Double-click a row's Delete fast — one delete fires and no error banner shows.
Force the failure path (submit an invalid URL): the button must re-enable and the inline
error must still appear, so a failed submit is retryable.

### Tests for User Story 3 (optional) ⚠️

- [X] T012 [US3] In `src/form.test.ts`, using the T004 harness, add a test that resolves `addSite` from a deferred promise, dispatches two `submit` events back to back, and asserts `addSite` was called exactly once and that `#site-submit` is `disabled` while the call is in flight and re-enabled after it settles.
- [X] T013 [US3] In `src/form.test.ts`, add the failure-path test: reject the `addSite` deferred, then assert `#site-submit` is re-enabled and `#site-error` is visible with the rejection string — a failed submit must stay retryable.

### Implementation for User Story 3

- [X] T014 [US3] In `src/form.ts`, guard the submit handler: set `submit.disabled = true` immediately before the awaited `updateSite`/`addSite` call and restore it in a `finally` block so both the success and the existing `catch` path re-enable it. Keep `resetToAddMode()` on success and `showError(String(message))` on failure exactly as they are.
- [X] T015 [US3] In `src/form.ts`, guard the delete handler in the `tbody` click listener: narrow the `closest<HTMLElement>("[data-action]")` result to `HTMLButtonElement` (confirmed available by T005), return early if it is already `disabled`, disable it around the awaited `deleteSite(id)`, and re-enable it in the error path before `showError`. On success the row is removed by `hooks.onDeleted(id)`, so no re-enable is needed there — note that in a comment so the asymmetry reads as deliberate.

**Checkpoint**: Neither form submission nor row deletion can fire twice; failures stay retryable.

---

## Phase 6: User Story 4 - The interval field has an upper bound (Priority: P4)

**Goal**: A pasted very large interval is rejected at the input instead of at the IPC
boundary. `index.html`'s `#site-interval` has `min="10"` and no `max`; `src/form.ts`
clamps the floor only. Rust's `clamp_interval` (`src-tauri/src/model.rs`) also enforces
a floor only, and `interval_secs` is a `u64` — a large enough JS number fails to
deserialize.

**Why P4**: Already fails gracefully — the command rejects, `form.ts`'s `catch` renders
the error inline, nothing crashes. This is prevention at the source, not a bug fix.

**Independent Test**: Type `999999999999999999999` into the interval field and submit —
the value is clamped to the ceiling (or the browser's native `max` validation blocks
submission) and no IPC error banner appears. Submit `5` — still clamps up to `10`, the
existing floor behavior, unchanged.

### Tests for User Story 4 (optional) ⚠️

- [X] T016 [US4] In `src/form.test.ts`, using the T004 harness, add a clamp test table covering the floor (`5` → `10`), the pass-through (`60` → `60`), the new ceiling (`999999999` → `MAX_INTERVAL`), and the non-numeric fallback (`""`/`abc` → `60`), asserting on the `intervalSecs` argument passed to the mocked `addSite`.

### Implementation for User Story 4

- [X] T017 [P] [US4] In `index.html`, add `max="86400"` to the `#site-interval` input alongside the existing `min="10" step="1" value="60"`, so the browser blocks the value at the source.
- [X] T018 [US4] In `src/form.ts`, add `const MAX_INTERVAL = 86400;` next to `MIN_INTERVAL`/`DEFAULT_INTERVAL` and extend the existing clamp from `Math.max(MIN_INTERVAL, parsed)` to also apply the ceiling (`Math.min(MAX_INTERVAL, Math.max(MIN_INTERVAL, parsed))`). Keep the `Number.isNaN` → `DEFAULT_INTERVAL` branch as-is. Add a comment stating that 86400 (24 h) is a product-level guardrail chosen here, not a protocol limit — the backend enforces only the `MIN_INTERVAL_SECS` floor — and that the number lives in exactly two places (here and `index.html`'s `max`), which must stay in sync.
- [X] T019 [US4] Verify: `pnpm build` and `pnpm test` green, and walk the Independent Test above by hand in `pnpm tauri dev`.

**Checkpoint**: The interval field is bounded on both ends, at the input and in the clamp.

---

## Phase 7: User Story 5 - A missing `#autostart` element cannot halt startup (Priority: P5)

**Goal**: Remove the latent non-null assertion in `src/main.ts`'s `mountAutostart`. If
`#autostart` ever went missing, `querySelector(...)!` yields `null`, the `try` throws,
and the `catch` block *itself* dereferences `checkbox` (`checkbox.disabled = true`) and
throws again — aborting the rest of `main()`.

**Why P5**: Latent only. The element is static in `index.html`, so this is unreachable
today. It is last because it protects against a future edit, not a present defect.

**Independent Test**: Temporarily delete the `#autostart` input from `index.html`, run
`pnpm tauri dev`, and confirm the sites table still renders, checks still run, and a
banner (rather than a dead page) reports the missing control. Restore `index.html`
afterward and confirm normal operation.

### Implementation for User Story 5

- [X] T020 [US5] In `src/main.ts`, drop the `!` from the `#autostart` lookup in `mountAutostart` and early-return with `showBanner("The autostart control is missing from the page.")` when the element is `null`, before the `try`. This leaves the existing `catch` block operating only on a checkbox known to exist, so its `checkbox.disabled = true` can no longer throw.
- [X] T021 [US5] Verify by hand against the Independent Test above (delete the input, confirm the rest of the app survives, restore it), then `pnpm build` and `pnpm test` green.

**Checkpoint**: No non-null assertion in `mountAutostart`; a missing control degrades to a banner.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [X] T022 Run the full Constitution "Quality Gates" set on the finished branch: `cargo test` and `cargo clippy -- -D warnings` from `src-tauri/`, plus `pnpm test` and `pnpm build`. All four must be green. Rust is untouched by this feature, so a Rust failure means a bad merge, not a bad edit.
- [X] T023 [P] Update `docs/ROADMAP.md`: remove the five shipped bullets from section 1 and replace the section with a "Done" note pointing at `specs/002-robustness/` (mirroring the existing note that supersedes the original section 1), renumbering the sections below. Note that `docs/` is gitignored — this edit is local-only and does not appear in the PR.
- [X] T024 [P] Add a `CHANGELOG.md` entry for this feature, following the format of the existing scaffold-cleanup entry.
- [X] T025 Re-read the five changed hunks against Constitution Principle IV: confirm no logic moved into an untested shell and that `src/form.test.ts` (if the optional tests were taken) covers the clamp bounds and the in-flight guard. Record anything deliberately left untested in `docs/ROADMAP.md` §5 rather than dropping it silently.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on Setup. Blocks T012/T013 (US3 tests) and T016 (US4 test) only. Implementation tasks in every story can begin without it.
- **User Stories (Phases 3–7)**: Depend on Setup. Ordered P1 → P5 by value, but see the file-contention note — they are sequential in practice, not by design.
- **Polish (Phase 8)**: Depends on every story that is being taken.

### User Story Dependencies

Every story is functionally independent — none reads, calls, or relies on another's change.
The ordering constraint is textual, not logical:

- **US1 (P1)**, **US2 (P2)**, **US5 (P5)** all edit `src/main.ts`, in three separate
  functions (`main`, `upsertSite`, `mountAutostart`). Land them in sequence to avoid
  conflicts; any order works, and each is independently revertable.
- **US3 (P3)** and **US4 (P4)** both edit `src/form.ts` — US3 the submit/delete handlers,
  US4 the clamp expression and module constants. Land US3 first (it touches the same
  submit handler US4's clamp sits in).
- **US4** also owns the only `index.html` edit (T017), which conflicts with nothing.

### Within Each User Story

- Optional tests before implementation where present (US3, US4) — write them against the
  T004 harness, confirm they fail, then implement.
- Implementation → comment → verify, in that order.
- Each story ends green (`pnpm build` + `pnpm test`) before the next begins.

### Parallel Opportunities

Genuinely few, and named rather than sprinkled:

- **T003** and **T005** are read-only verifications — parallel with anything.
- **T017** (`index.html`) is the only edit outside `src/main.ts` / `src/form.ts` — it can
  land alongside any `src/` work.
- **T023** (`docs/ROADMAP.md`) and **T024** (`CHANGELOG.md`) are independent documents.
- The two `src/main.ts` stories and the two `src/form.ts` stories are **not** parallel.
  Nothing in Phases 3–7 is safely concurrent beyond the above.

---

## Parallel Example: Phase 1 + Phase 2 verifications

```bash
# The read-only checks can run together:
Task: "T003 Re-verify the five ROADMAP §1 claims against the tree"
Task: "T005 Confirm render.ts's row actions are HTMLButtonElement"
```

```bash
# Late in the feature, the two document updates are independent:
Task: "T023 Update docs/ROADMAP.md section 1 to a Done note"
Task: "T024 Add the CHANGELOG.md entry"
```

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. Phase 1: Setup (T001–T003).
2. Phase 3: US1 — the listener hoist (T006–T008).
3. **STOP and VALIDATE**: `pnpm build`, `pnpm test`, and a `pnpm tauri dev` launch.
4. This alone is a shippable PR — it is the only item the v1 review called a real bug,
   and it is a three-line reorder plus a comment.

Phase 2 is skippable for the MVP: nothing in US1 needs the `form.ts` harness.

### Incremental Delivery

1. Setup → US1 → validate → ship (MVP).
2. Add US2 → validate → ship. Both `src/main.ts`; small, honest UI win.
3. Add Phase 2 + US3 → validate → ship. First `form.ts` tests land here.
4. Add US4 → validate → ship.
5. Add US5 → validate → ship.
6. Phase 8 polish over whatever shipped.

All five are S-effort. Bundling them into one PR is reasonable given the size — the
increments above exist so a partial run still lands something coherent.

### Parallel Team Strategy

Not applicable, and worth saying so rather than inventing one. Five S-sized edits across
two files means a second developer would spend more time resolving conflicts in
`src/main.ts` and `src/form.ts` than the work saves. Constitution Principle I ("One Mac,
One Person") describes the product; this feature matches it in scale.

---

## Notes

- `[P]` tasks = different files, no dependencies. Used sparingly here on purpose.
- No Rust source is edited. `cargo test` and `cargo clippy -- -D warnings` still gate,
  per the Constitution's Quality Gates.
- Nothing here touches the Rust/TS snake_case contract (Principle V) or the persisted
  `sites.json` shape (Principle II).
- US2 changes when a status is *discarded*, never when one is written — check results
  stay ephemeral and in-memory, per Principle II.
- Commit after each task or logical group; stop at any checkpoint to validate a story.
- `docs/` is gitignored: T023's roadmap edit stays local and will not show in the PR.

---

## Verification Record

How each story was actually confirmed, including where the method differed from
the task text. Written after the fact so the checkmarks above are not read as
claiming more than was done.

### Automated (the primary net)

Every behavioral assertion added by this feature was confirmed to **fail against
the pre-fix code** before being kept — a test that passes either way pins
nothing. The reverts were temporary and each was restored immediately:

| Story | Pinned by | Failed correctly when reverted |
|-------|-----------|-------------------------------|
| US1 | `main.test.ts` — mock `invocationCallOrder`, both listeners before any startup IPC call | ✅ moved registrations back below `mountAutostart` |
| US2 | `main.test.ts` — drop on URL change; keep on label-only, interval-only, first-add | ✅ restored the original `upsertSite` |
| US3 | `form.test.ts` — 5 tests: double-submit, double-delete, retryable failure, Add-vs-Edit | ✅ all 5 red before `T014`/`T015` |
| US4 | `form.test.ts` — clamp table, 5 cases | ✅ ceiling case red before `T018` |
| US5 | `main.test.ts` — fixture omits `#autostart`; banner text + `main()` continues | ✅ restored the non-null assertion |

Final gates: `cargo test` 29 ✅ · `cargo clippy -- -D warnings` clean ✅ ·
`pnpm test` 30 ✅ · `pnpm build` clean ✅.

### By hand

- **T008 / T021 (launch)** — done. `pnpm tauri dev` from this worktree compiled
  and ran; a screenshot confirmed all five sites rendering with live status dots,
  a ticking "Last checked" column, and "Launch at login" reflecting OS state. No
  panic, no console error. The app was then stopped and port 1420 confirmed free.
- **T011 / T019 (interaction) — NOT walked in the GUI.** Both Independent Tests
  require driving the running app (add a site, edit its URL, paste an oversized
  interval). That means mutating the real
  `~/Library/Application Support/com.clintparker.site-checker/sites.json`, and
  the session had no GUI-automation tool available, so the alternative was
  blind-clicking coordinates in a live desktop session. Covered by the unit tests
  above instead — `main.test.ts` asserts exactly T011's four cases, and
  `form.test.ts` asserts exactly T019's clamp table. **Still worth a two-minute
  manual pass** before merge, since neither unit test exercises the real IPC
  round trip.
- **Data safety** — `sites.json` was copied out of tree before launch and
  verified byte-identical afterward; no stray backup file was left behind.

### Deviations from the task text

- **T001** forked the worktree from local `main`, not `origin/main`: `main` was
  one commit ahead (`da424ea`, the before-screenshots this feature needs).
  `docs/` (gitignored) and the untracked `tasks.md` were copied in afterward, and
  `.specify/feature.json` was pointed at `specs/002-robustness`.
- **T014** adds an explicit `if (submit.disabled) return;` on top of the
  disable/`finally` the task describes. Disabling a button stops a *click*, but
  this handler also runs for a programmatic submit, which never consults
  `disabled` — without the early return the guard is not actually closed.
- **`src/main.test.ts` is new and was not in the task list.** T025 asks that
  anything left untested be recorded rather than dropped; testing US1/US2/US5
  turned out to be cheaper than documenting why they could not be, and it is
  what replaces the skipped GUI walkthroughs. The structural obstacle the task
  list assumed — `main.ts` calling `main()` at module load — is handled by
  stubbing `./api` and mounting the fixture before the dynamic import.
- Two stale worktrees from earlier aborted runs
  (`--20260805-173433-robustness-fixes-section`, `--20260805-173949-robustness-fixes`)
  were left in place, untouched. Neither holds real work; `/speckit-worktrees-clean`
  would reclaim them.
