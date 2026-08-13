# Tasks: Clickable URLs Open in the Default Browser

**Input**: Design documents from `/specs/20260813-105424-urls-clickable-and/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Test tasks ARE included. The spec does not ask for TDD in those words, but the constitution's Quality Gates make `cargo test` and `pnpm test` the merge bar, and [quickstart.md](./quickstart.md#1-automated-the-merge-bar) names the exact cases that must be proven. Those named cases are transcribed here as tasks rather than left to the implementer's discretion. Recorded as a judgement call — see [Decisions Made in This Step](#decisions-made-in-this-step).

**Organization**: Tasks are grouped by user story so each can be implemented, tested, and demoed on its own.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- File paths are repo-relative

## Path Conventions

This is a desktop app split into two capabilities by `living-specs.yml`:

- **backend** — `src-tauri/src/**` (Rust, tested by `cargo test` from `src-tauri/`)
- **frontend** — `src/**` + `index.html` (TypeScript, tested by `pnpm test` → vitest + happy-dom)

Rust unit tests live in a `#[cfg(test)] mod tests` inside the file under test — there is no separate `tests/` tree. TS tests live beside their module as `*.test.ts`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Establish the pre-change baseline and confirm the first-party route is the only route. This feature adds **no new dependency in either lockfile**, so there is nothing to install.

- [ ] T001 Record a green pre-change baseline: run `cd src-tauri && cargo test && cargo clippy -- -D warnings` and `pnpm test` from the repo root, and note that all three pass before any edit
- [ ] T002 [P] Confirm `tauri-plugin-opener` is absent from `src-tauri/Cargo.toml`, `package.json`, and that `src-tauri/capabilities/default.json` needs no new permission entry — the first-party command route ([research.md](./research.md) §1) requires none

**Checkpoint**: Baseline green, plugin route confirmed closed.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The dispatch spine every story rides on — the authoritative scheme guard, the command that hands an address to macOS, the single typed frontend boundary, and the frontend's rendering-side guard.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [ ] T003 Add pure `pub fn openable_url(input: &str) -> Result<String, String>` in `src-tauri/src/model.rs`, directly beside `normalize_url`, returning the input **byte-identical** on success per [contracts/open-url-command.md](./contracts/open-url-command.md#guard-openable_url). It MUST NOT call `normalize_url` — that function repairs a scheme-less string, which is the opposite of this contract. Add a doc comment stating that contrast explicitly.
- [ ] T004 Add `openable_url` unit tests to the `#[cfg(test)] mod tests` in `src-tauri/src/model.rs`, one case per row of the contract table: `https://example.com`, `http://example.com/health?q=A` (path and query case preserved), `HTTPS://example.com` (accepted, returned untouched), `ftp://example.com`, `file:///etc/hosts`, `javascript:alert(1)`, `example.com` (refused, **not** repaired), `""`, `"   "`, `https://` (no host). Assert byte-identity on every `Ok` — `https://example.com` must not come back as `https://example.com/` (FR-006).
- [ ] T005 Add `#[tauri::command(async)] pub fn open_url(url: String) -> Result<(), String>` in `src-tauri/src/commands.rs`: validate with `openable_url`, return its `Err` unchanged without spawning anything, otherwise spawn `/usr/bin/open` (absolute path, so the launched binary does not depend on inherited `PATH`) with the URL as its single argument and wait. Exit `0` → `Ok(())`; non-zero → `Err` carrying the child's trimmed stderr wrapped in a sentence naming the address; spawn `io::Error` → `Err` naming the failure. The `(async)` is load-bearing ([research.md](./research.md) §3) — without it Tauri runs the body on the main thread and the wait stalls the window.
- [ ] T006 Register `commands::open_url` in the `tauri::generate_handler!` list in `src-tauri/src/lib.rs` (currently ends at `commands::set_autostart`)
- [ ] T007 [P] Add `export function openUrl(url: string): Promise<void> { return invoke("open_url", { url }); }` to `src/api.ts`, with a comment noting that `url` is a single lowercase word so Tauri's camelCase→snake_case argument conversion is a no-op on it (Constitution V)
- [ ] T008 [P] Add exported pure `isOpenable(url: string): boolean` to `src/render.ts` — true iff, after trimming, the string begins with a case-insensitive `http://` or `https://` ([data-model.md](./data-model.md#row-url-activatability)). Add a comment marking this as the deliberate frontend half of a rule also held in `model.rs`, per [research.md](./research.md) §4; it is duplicated rather than fetched because it is a rendering decision made 60 times a minute.
- [ ] T009 Add `isOpenable` unit tests to `src/render.test.ts` mirroring the T004 case list (accepts both schemes and mixed case, rejects `ftp://`, `file://`, `javascript:`, bare `example.com`, and empty/whitespace)

**Checkpoint**: The backend can open an address and refuse a bad one, the frontend has one typed way to ask, and both sides agree on what is openable. User stories can now begin.

---

## Phase 3: User Story 1 - Visit a site from its row (Priority: P1) 🎯 MVP

**Goal**: Clicking a site's URL in the table opens that exact address in the system's default browser, without the dashboard navigating away.

**Independent Test**: Add `https://example.com` with no label, click its URL in the table, confirm the default browser comes forward on that address and the dashboard still shows the table, still ticking. Then add a label and confirm the URL beneath it behaves the same while the label itself does nothing.

### Tests for User Story 1

- [ ] T010 [P] [US1] Add render cases to `src/render.test.ts`: an unlabelled site renders `<button type="button" class="site-primary site-url" data-open-url="…">`; a labelled site renders the label as an inert `<span class="site-primary">` plus `<button class="site-secondary site-url" data-open-url="…">`; the button's `data-open-url` is the **full** stored URL even when the rendered text is very long (FR-006)
- [ ] T011 [P] [US1] Create `src/open.test.ts` with the pure `shouldOpen` ledger cases: a first activation is accepted; a repeat inside `ACTIVATION_WINDOW_MS` is suppressed; a repeat after the window is accepted; two distinct URLs have independent windows; and a **suppressed** activation does not extend the window (otherwise drumming on the control suppresses indefinitely) — see [data-model.md](./data-model.md#activation-ledger)
- [ ] T012 [P] [US1] Add delegated-listener cases to `src/open.test.ts` over a fixture `<tbody>`: a click on the button calls `openUrl` with the value of `data-open-url`; a click on a labelled row's `<span>` label calls nothing (FR-008); a click on a row's Edit/Delete button calls nothing. Mock with `vi.mock("./api", …)`, the convention `form.test.ts` and `main.test.ts` already use.

### Implementation for User Story 1

- [ ] T013 [US1] Rewrite the name-cell construction in `renderRow` in `src/render.ts` to the structure in [contracts/row-url-element.md](./contracts/row-url-element.md#structure-of-the-name-cell): when `isOpenable(site.url)`, the URL is a `<button type="button">` carrying `data-open-url` set to the full stored URL, in the primary slot when there is no label and the secondary slot when there is; the label, when present, stays an inert `<span class="site-primary">`. Use `data-open-url` and **not** `data-action` — `form.ts`'s listener matches `.closest("[data-action]")`, so a different attribute makes FR-010 hold structurally rather than by convention.
- [ ] T014 [US1] Update `updateName` in `src/render.ts` to reconcile the new cell: write `textContent` and `data-open-url` only on change, through the existing change-guarded helper style (`setText`/`setClass`); rebuild the cell's two children on the one transition where a label is added or removed, because the URL moves between slots ([research.md](./research.md) §6); replace the node when `isOpenable` flips. A repaint must still not read `site.url` in a way that touches this element (FR-011).
- [ ] T015 [P] [US1] Create `src/open.ts` exporting `ACTIVATION_WINDOW_MS = 1000`, the pure `shouldOpen(ledger, url, now)` rule, and `mountUrlOpener(…)` — a delegated `click` listener on the table body matching `.closest("[data-open-url]")`, reading the address from the attribute (never from `textContent`, so truncation and wrapping are irrelevant) and calling `api.openUrl`. It sits beside `form.ts` as a second thin shell over the `api.ts` boundary.
- [ ] T016 [US1] Mount the opener from `src/main.ts` inside `main()`, alongside the existing `mountForm({…})` call, passing the module's `tbody` and wiring failures to the existing `showBanner`
- [ ] T017 [P] [US1] Add `.site-url` rules to `src/styles.css` per [contracts/row-url-element.md](./contracts/row-url-element.md#styling-contract-srcstylescss): `background: none; border: none; padding: 0; font: inherit; color: inherit` so the primary and secondary lines keep their existing size and opacity treatment; `display: block; text-align: left` to match `.site-primary`/`.site-secondary` (a `<button>` centres its text by default, which would visibly shift the URL against the other rows); `text-decoration: underline; cursor: pointer` for the FR-004 affordance

**Checkpoint**: User Story 1 is fully functional — a click opens the browser, the dashboard stays put, and the label is not a target. This is the MVP.

---

## Phase 4: User Story 2 - Open a site without a mouse (Priority: P2)

**Goal**: The URL is reachable and activatable by keyboard alone, with a visible focus indication, and focus survives the 1 s repaint.

**Independent Test**: Tab through a row until the URL takes focus (before that row's Edit button), confirm a visible focus ring, press Enter and confirm the browser opens the same address a click would have. Hold focus through a status arriving and a few age ticks and confirm it stays.

> **Note**: The `<button>` decision made in [plan.md](./plan.md) delivers Enter/Space activation and tab-order membership natively, with no `tabindex`. US2 is therefore mostly a focus-affordance task plus the tests that lock the behaviour in. Recorded as a judgement call below.

### Tests for User Story 2

- [ ] T018 [P] [US2] Add reconciliation cases to `src/render.test.ts` in the existing style: the URL button's **element identity is preserved** across a `renderTable` call with only `now` advanced, and across one where a status event has landed (FR-011, US2 scenario 3)
- [ ] T019 [P] [US2] Add a `src/render.test.ts` case asserting the URL button carries **no** `tabindex` attribute — it is in the tab order natively, and an explicit value would be a regression waiting to happen

### Implementation for User Story 2

- [ ] T020 [US2] Add a `:focus-visible` outline for `.site-url` in `src/styles.css` — `:focus-visible` rather than `:focus`, so a mouse click does not leave a ring behind (FR-005)

**Checkpoint**: Every task achievable with a pointer in this feature is achievable with the keyboard alone (SC-004), and User Stories 1 and 2 both work independently.

---

## Phase 5: User Story 3 - A refusal explains itself (Priority: P3)

**Goal**: An activation that cannot be completed produces a visible explanation, and an entry that will not be opened is never presented as though it could be.

**Independent Test**: Force `openUrl` to reject and confirm the reason appears in the banner and the dashboard stays fully usable. Separately, hand-edit a `sites.json` entry to `ftp://example.com`, relaunch, and confirm the row appears with its URL as plain text — no underline, no pointer cursor, not reachable by Tab, and clicking it does nothing.

### Tests for User Story 3

- [ ] T021 [P] [US3] Add a `src/render.test.ts` case: a site whose URL is not http/https renders as `<span class="site-primary">` with **no** `data-open-url`, no `site-url` class, and no `tabindex`. It is shown, not hidden and not flagged as invalid (FR-007, spec Open Decision 3).
- [ ] T022 [P] [US3] Add a `src/open.test.ts` case: a rejected `openUrl` passes the backend's bare message to the error hook using the `String(message)` idiom `form.ts` already relies on, and the listener does not throw
- [ ] T023 [US3] Add a `src/main.test.ts` case: an open failure reaches `showBanner`, and `renderTable` still runs afterwards — the table keeps updating and nothing else is disturbed (FR-009, US3 scenario 2, SC-006)

### Implementation for User Story 3

- [ ] T024 [US3] Add the failure path to `mountUrlOpener` in `src/open.ts`: `catch` the rejected `openUrl` and route `String(message)` to the injected banner hook. It must go to the banner (`#banner` via `showBanner`) and **not** to the form's `#site-error` — the failure is not about anything the user typed. The command mutates nothing, so there is no partial state to unwind.

**Checkpoint**: All three user stories are independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T025 Run the full merge bar from [quickstart.md](./quickstart.md#1-automated-the-merge-bar): `cd src-tauri && cargo test && cargo clippy -- -D warnings`, then `pnpm test` from the repo root. All three must be green. Confirm no test invokes the `open_url` command — `cargo test` must never launch a browser.
- [ ] T026 [P] Confirm `git diff` touches **no** dependency manifest: not `src-tauri/Cargo.toml`, not `src-tauri/Cargo.lock`, not `package.json`, not `pnpm-lock.yaml`, and not `src-tauri/capabilities/default.json`. A change in any of them means the plugin route crept back in ([research.md](./research.md) §1).
- [ ] T027 Run the manual pass in [quickstart.md](./quickstart.md#2-manual-a-real-browser-a-real-click) §2 — US1 click, US2 tab-and-Enter, US3 hand-edited `ftp://` entry, and the FR-012 double-click check. **Back up `~/Library/Application Support/com.clintparker.site-checker/sites.json` outside the repo first** and confirm no earlier `pnpm tauri dev` is still running: the app has no config-directory override, so a dev run reads and writes the real file.
- [ ] T028 Confirm the feature stayed in its lane per [quickstart.md](./quickstart.md#3-nothing-else-changed) §3: `sites.json` byte-identical after clicking (no field added, no timestamp written), and adding, editing, deleting, status updates, and the age counters all still work (FR-010, Constitution II)
- [ ] T029 [P] Update the affected living specs via `/speckit-companion-living-sync` — `capabilities/frontend/spec.md` gains the URL control, its keyboard path, and the banner surface; the backend capability gains `open_url` and `openable_url`. Do not hand-edit these files.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Setup — **BLOCKS all user stories**
- **User Stories (Phases 3–5)**: All depend on Foundational. Can then run in parallel, or sequentially in priority order (P1 → P2 → P3)
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **US1 (P1)**: Depends only on Foundational. No dependency on US2 or US3.
- **US2 (P2)**: Depends only on Foundational. Its focus ring lands on `.site-url`, so in practice T020 is written after T017 has created that rule — but the story is independently testable the moment the button exists.
- **US3 (P3)**: Depends only on Foundational. Its inert-rendering assertion (T021) tests the `isOpenable` branch built in T013; its banner path (T024) extends the listener built in T015. Independently testable in both halves.

### Within Each Story

- Tests are listed before the implementation they cover; write them first and watch them fail
- `src/render.ts` tasks (T013 → T014) are sequential — same file, and `updateName` must match what `renderRow` produced
- `src/styles.css` tasks (T017, T020) are sequential — same file
- `src/open.ts` tasks (T015 → T024) are sequential — same file

### Parallel Opportunities

- **Foundational**: T007 (`api.ts`) and T008 (`render.ts`) touch different files and neither depends on the Rust work — run alongside T003–T006. T004 follows T003; T009 follows T008; T006 follows T005.
- **US1**: all three test tasks (T010, T011, T012) are parallel — two different files, no shared state. In implementation, T013/T014 (`render.ts`), T015 (`open.ts`), and T017 (`styles.css`) are three independent files.
- **US2**: T018 and T019 are parallel.
- **US3**: T021 (`render.test.ts`) and T022 (`open.test.ts`) are parallel.
- **Cross-story**: once Foundational is done, all three stories can proceed at once, with the two same-file orderings noted above as the only coordination points.

---

## Parallel Example: User Story 1

```bash
# Launch all three US1 test tasks together:
Task: "Add render cases to src/render.test.ts for the button structure and full-URL attribute"
Task: "Create src/open.test.ts with the pure shouldOpen ledger cases"
Task: "Add delegated-listener cases to src/open.test.ts over a fixture tbody"

# Then the three independent implementation files together:
Task: "Rewrite the name-cell construction in renderRow in src/render.ts"
Task: "Create src/open.ts with ACTIVATION_WINDOW_MS, shouldOpen, and mountUrlOpener"
Task: "Add .site-url rules to src/styles.css"
```

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. Phase 1: Setup — baseline green
2. Phase 2: Foundational — **blocks everything**
3. Phase 3: User Story 1
4. **STOP and VALIDATE**: add a site, click its URL, watch the browser come forward and the dashboard stay put
5. This alone satisfies SC-001 (five steps down to one) and is demoable

### Incremental Delivery

1. Setup + Foundational → dispatch spine ready
2. US1 → click opens the browser → **MVP**
3. US2 → keyboard parity (SC-004)
4. US3 → refusals explain themselves (SC-006), and non-http/https entries go inert (SC-002)
5. Polish → merge bar, dependency guard, manual pass, living specs

Each story adds value without breaking the ones before it.

---

## Decisions Made in This Step

No user was present for this run. These were decided here and should be surfaced in the pull request description alongside the three in [spec.md](./spec.md#open-decisions-for-review) and the five in [plan.md](./plan.md#open-decisions-for-review).

1. **Test tasks are included even though the spec never asked for TDD.** The template treats tests as opt-in. Omitting them here would have been wrong: the constitution's Quality Gates make `cargo test`, `pnpm test`, and clean `clippy` the merge bar, and [quickstart.md](./quickstart.md#1-automated-the-merge-bar) already enumerates the exact cases that must be proven. Transcribing those into tasks makes the bar visible in the checklist rather than discovered at the gate. Revisit only if the tests are meant to be written in one sweep at the end instead.

2. **`isOpenable` is Foundational, not part of US3.** FR-007's "must not be presented as activatable" reads like a US3 concern, and quickstart validates it under US3. But putting the guard there would mean US1 first renders every URL as a button and US3 then rewrites that branch. Placing the pure function in Phase 2 lets T013 build the branch once and leaves US3 owning the assertions (T021) and the banner path (T024). The cost: US3's implementation is a single task, which is an honest reflection of its size, not an oversight.

3. **US2 carries one implementation task.** The `<button>` decision in plan.md gives Enter/Space activation and tab-order membership natively, so the only new code US2 needs is a `:focus-visible` ring; the rest of the story is two tests that lock the behaviour in. This is a real outcome of the element choice rather than a thin phase — but if US2 is expected to be a substantive work item, the element decision is what to revisit, not this task list.

4. **The living-spec sync (T029) is a Polish task, not a per-story one.** Both capabilities change, and `speckit-companion-living-sync` groups working-tree changes by capability in one pass — running it once at the end produces a coherent update rather than three partial ones.
