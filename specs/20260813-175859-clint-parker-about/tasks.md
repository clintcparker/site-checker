# Tasks: Author Attribution in About

**Input**: Design documents from `/specs/20260813-175859-clint-parker-about/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/about-surface.md](./contracts/about-surface.md),
[quickstart.md](./quickstart.md)

**Tests**: INCLUDED. [quickstart.md](./quickstart.md) §1 specifies nine automated assertions
(T1–T9) as the feature's required coverage and [plan.md](./plan.md) lists `src/about.test.ts` as
a new file. That is an explicit request for tests, so test tasks are generated and are written
before the implementation they cover.

**Organization**: Tasks are grouped by user story so each can be implemented and verified
independently.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2)
- Paths are relative to the repository root of this worktree

## Path Conventions

Single project, per [plan.md](./plan.md) "Project Structure": a Rust core under `src-tauri/`
(**untouched by this feature**) and a flat vanilla-TypeScript frontend under `src/`, each module
paired with a colocated `*.test.ts`. No new directory is introduced.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Make the worktree buildable and record the baseline this feature must not disturb.

- [X] T001 Install frontend dependencies: run `pnpm install` at the repository root — this
      worktree starts with no `node_modules` (quickstart.md "Prerequisites")
- [X] T002 Record the pre-change baseline by running all four gates and saving their output:
      `pnpm test`, `pnpm exec tsc --noEmit`,
      `cargo test --manifest-path src-tauri/Cargo.toml`, and
      `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`. The two Rust results
      are the baseline that T022 compares against — plan.md asserts they must be *unchanged*.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The About surface's shell — the dialog element, its open/close wiring, its mount
point, and its layout. Both user stories render into it, so neither can start until it exists.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T003 Add the About shell to `index.html`: an empty `<dialog id="about">` containing a
      `<button data-about-close>` dismissal control, plus a `<button id="about-open">About</button>`
      inside the existing `<footer id="footer">` beside the "Launch at login" label (research
      R-006, FR-001, SC-001). Use `data-about-close` — not `data-action`, not `data-open-url` —
      so it cannot collide with `form.ts`'s or `open.ts`'s delegated listeners
      (contracts/about-surface.md §2).
- [X] T004 Create `src/about.ts` exporting `mountAbout(hooks: { onError: (message: string) => void; now?: () => number })`:
      query `#about`, `#about-open` and `[data-about-close]`, open with `showModal()` and close
      with `close()`. Follow `main.ts:70-74`'s precedent — if an element is missing, report via
      `hooks.onError` and return rather than asserting non-null. Accept the optional `now` seam
      purely so T015 need not wait out a real second, mirroring `open.ts:34-35`.
- [X] T005 Call `mountAbout({ onError: showBanner })` from `main()` in `src/main.ts`, alongside
      the existing `mountUrlOpener(tbody, { onError: showBanner })` call at `src/main.ts:124`
- [X] T006 [P] Add dialog layout rules to `src/styles.css` (currently 184 lines; `footer` at
      :134, `.setting` at :177) so the About content is readable and its controls activatable at
      the window's 480×320 minimum from `tauri.conf.json` (FR-012)
- [X] T007 Create `src/about.test.ts` with the shared fixture: `vi.mock("./api")` returning
      mocked `openUrl` and `getVersion`, following the convention at `src/open.test.ts:7-10`,
      plus a helper that builds the `index.html` About markup into the document and calls
      `mountAbout`. `happy-dom` implements `showModal()`/`close()` as `open`-attribute toggles,
      so dialog state is directly assertable (quickstart.md §1).

**Checkpoint**: The About dialog opens and closes from the footer. It is empty — content is the
user stories' work, and they can now proceed in parallel.

---

## Phase 3: User Story 1 - See who made this (Priority: P1) 🎯 MVP

**Goal**: The About surface names the app, its version, and Clint Parker as its creator.

**Independent Test**: Launch the app, open About from the footer, and read it. The name is
present and spelled "Clint Parker". Nothing else in the app changes.

### Tests for User Story 1 ⚠️

> **Write these first; confirm they FAIL before implementing T011–T013.**

- [X] T008 [US1] Add tests T1, T2 and T9 to `src/about.test.ts`: activating `#about-open` sets
      `dialog.open === true` (FR-001); the dialog contains the exact string
      `Created by Clint Parker` and the literal `Site Checker` (FR-002, FR-004); opening and
      closing the dialog leaves `sites` and `statuses` untouched and invokes no store command
      (FR-007, SC-006)
- [X] T009 [US1] Add tests T3 and T4 to `src/about.test.ts`: the version line renders the string
      `getVersion()` resolved with, verbatim and unparsed (FR-003); a `getVersion()` rejection
      still opens the dialog, still shows the attribution and the link slot, renders
      `Version unavailable`, and raises **no** banner (FR-003, research R-005)

### Implementation for User Story 1

- [X] T010 [P] [US1] Re-export `getVersion` from `@tauri-apps/api/app` in `src/api.ts`, beside
      the existing `openUrl` export at `src/api.ts:78`. **Do not edit
      `src-tauri/capabilities/default.json`** — `core:default` already grants
      `core:app:allow-version` (contracts/about-surface.md §1); widening a capability for an
      already-granted permission is a review finding, not a fix.
- [X] T011 [US1] Add the static About content to `<dialog id="about">` in `index.html`: the app
      name `Site Checker`, a version line element, and the attribution `Created by Clint Parker`
      — exact strings from [data-model.md](./data-model.md) "Constants introduced". No email, no
      handle, no username; the address in `Cargo.toml`'s `authors` is deliberately not surfaced.
- [X] T012 [US1] In `src/about.ts`, fetch the version with `getVersion()` when the dialog is
      first opened and write it into the version line verbatim; on rejection write
      `Version unavailable` and still open the dialog, without raising a banner (research R-005).
      A version failure must never block the open.
- [X] T013 [US1] Run `pnpm test` and `pnpm exec tsc --noEmit`; confirm T1–T4 and T9 pass

**Checkpoint**: User Story 1 is fully functional and testable on its own — the About surface
carries the attribution with no link present.

---

## Phase 4: User Story 2 - Get to the author's site (Priority: P2)

**Goal**: An activatable `clintparker.com` control in the About surface hands the address to the
default browser, suppresses repeat activations, and surfaces a refusal visibly.

**Independent Test**: With About open, activate the link and confirm the default browser comes
forward at clintparker.com while Site Checker stays running and its checks continue.

### Tests for User Story 2 ⚠️

> **Write these first; confirm they FAIL before implementing T017–T019.**

- [X] T014 [US2] Add tests T5 and T6 to `src/about.test.ts`: the link element carries
      `data-open-url="https://clintparker.com"` exactly — secure scheme, apex domain, no path,
      no trailing slash (FR-005); activating it calls `openUrl` once with that address and does
      not navigate the page (FR-006)
- [X] T015 [US2] Add test T7 to `src/about.test.ts`: ten activations inside one second produce
      exactly one `openUrl` call, driven by injecting `now` through `mountAbout` rather than
      waiting out a real second (FR-008, SC-004)
- [X] T016 [US2] Add test T8 to `src/about.test.ts`: an `openUrl` rejection closes the dialog
      **first**, then writes the bare rejection string to the banner — assert the ordering, not
      just the outcome, since a modal covers the banner (FR-009, SC-005, research R-005)

### Implementation for User Story 2

- [X] T017 [US2] Add the link control to `<dialog id="about">` in `index.html`:
      `<button data-open-url="https://clintparker.com">clintparker.com</button>`. **No `<a href>`,
      no `target="_blank"`, no `window.open`** — an anchor can navigate the dashboard away from
      itself if the handler does not run, the reasoning already recorded at `render.ts:184-187`.
      The link text omits the scheme; the full address travels in the attribute
      (`open.ts:70-73`).
- [X] T018 [US2] In `src/about.ts`, call `mountUrlOpener(dialog, { … })` from `src/open.ts`
      **unchanged**, passing the injectable `now` through. It is written against a generic
      `HTMLElement` container and matches `[data-open-url]` by delegated listener, so it needs
      no edit. Repeat suppression comes entirely from its existing 1000 ms per-URL ledger — write
      no second rule. The dialog's ledger is deliberately separate from the table's (research
      R-004).
- [X] T019 [US2] In `src/about.ts`, wrap the `onError` handed to `mountUrlOpener` so it closes
      the dialog before calling `hooks.onError(message)` (FR-009, SC-005). Do not touch
      `src/open.ts` to achieve this.
- [X] T020 [US2] Run `pnpm test` and `pnpm exec tsc --noEmit`; confirm T1–T9 all pass

**Checkpoint**: Both user stories work independently. The feature is functionally complete.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Prove the design assertions that make this feature safe, then validate it in the
real app.

- [X] T021 [P] Confirm the backend is untouched: `git diff --stat -- src-tauri/` reports no
      changes. plan.md states `src-tauri/` is *entirely unchanged*; a diff here means a premise
      of the plan is wrong and the plan should be revisited, not worked around.
- [X] T022 [P] Confirm the version invariants CI enforces at `ci.yml:90-102` are untouched:
      `src-tauri/Cargo.toml` still `version = "0.0.0"`, `package.json` still `"version": "0.0.0"`,
      and `src-tauri/tauri.conf.json` still carries no `version` key
- [X] T023 Run all four automated gates green — `pnpm test`, `pnpm exec tsc --noEmit`,
      `cargo test --manifest-path src-tauri/Cargo.toml`,
      `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` — and confirm the two
      Rust results are identical to the T002 baseline
- [~] T024 **PARTIAL — the real-app half is blocked.** Run quickstart.md §2 manual checks M1–M7
      under `pnpm tauri dev`, including M6 at the 480×320 minimum window size (FR-012). M3, M4
      and M7 require the shipped app and must not be reported as passing from a browser-stubbed
      run (quickstart.md §3).
      **Done, via quickstart §3's browser route** (Vite + Playwright, `__TAURI_INTERNALS__`
      stubbed) at a 480×320 viewport: **M1** — `#about-open` visible in the footer at launch, no
      menu needed; **M2** — the dialog reads `Site Checker / Version 1.2.3 / Created by Clint
      Parker / clintparker.com / Close`, no `@` anywhere in it; **M5** — after ten activations
      the two seeded rows and their URLs are unchanged and `add_site`/`update_site`/`delete_site`
      were never called; **M6** — dialog (320×188), link and Close button all measured fully
      inside the 480×320 viewport, `document.elementFromPoint` at each control's centre returns
      that control, and the dialog needs no scrolling (`scrollHeight === clientHeight`); the
      dialog's text renders at 16/14/12px. **M4's frontend half** — ten `click()`s produced
      exactly one `open_url`, with `https://clintparker.com` verbatim.
      **NOT done — M3, M4's browser half, and M7**, which need a real browser to come forward.
      The Mac's screen is LOCKED this run (`screencapture` fails with "could not create image
      from rect"), so no native window can be shown, driven, or observed at all. Hand to
      `/speckit-qa-run` on an unlocked screen.
- [~] T025 **DONE by a safer route than the task specified.** Run quickstart.md M8, the forced
      open-failure check (SC-005).
      The task's method — repoint the link constant at an `ftp://` address, rebuild, revert —
      was **not** used: it needs the same real-app window a locked screen denies, and it edits
      shipped markup, which is the one way this feature could ship broken. Instead the refusal
      was injected at the boundary it actually arrives from, by making the stubbed `open_url`
      reject with the real guard's message. Observed in the real renderer: the dialog closed
      **first** (`dialog.open === false` when the banner was written), the banner then showed the
      bare message unmodified, both rows survived, and the whole path took **57 ms** against
      SC-005's one-second bound. `src/about.test.ts` asserts the same ordering from inside the
      callback. **No constant was edited, so nothing needed reverting.**
- [~] T026 **PARTIAL — the strong form needs a real app session.** Run quickstart.md M9, the
      config-untouched check (FR-010, SC-006).
      The full check wants a hash either side of a session that opens About, activates the link
      and quits — which needs the app window a locked screen denies. What **is** established:
      `~/Library/Application Support/com.clintparker.site-checker/sites.json` is byte-identical
      to its pre-run state — md5 `27364d6a5bad0746748bf046ffc15ba8`, mtime 2026-08-10 15:22, the
      same pair the screenshots step recorded before any of this work — so this run never touched
      it. The browser route seeds sites in memory over the stubbed IPC and never reaches the real
      file. Hand the end-to-end form to `/speckit-qa-run`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS both user stories
- **User Story 1 (Phase 3)**: Depends on Foundational. No dependency on US2.
- **User Story 2 (Phase 4)**: Depends on Foundational. Does not depend on US1's content, only on
  the shared dialog shell, so the two can run concurrently.
- **Polish (Phase 5)**: Depends on every user story being complete

### User Story Dependencies

- **User Story 1 (P1)**: Independent once Phase 2 is done. Shippable alone — an About surface
  carrying the attribution with no link is a correct, complete increment.
- **User Story 2 (P2)**: Independent once Phase 2 is done. In practice US1 lands first because it
  is the MVP, but nothing in US2 reads US1's content.

### Within Each User Story

- Tests are written first and must FAIL before the implementation tasks in the same phase
- Markup (`index.html`) before the behaviour that queries it (`src/about.ts`)
- `src/api.ts` re-export (T010) before the code that calls it (T012)

### Parallel Opportunities

- T006 (`src/styles.css`) runs alongside T003–T005 — different file, no shared symbol
- T010 (`src/api.ts`) runs alongside T011 (`index.html`) — different files
- T021 and T022 are independent read-only checks
- US1 and US2 can be worked concurrently by two people once Phase 2 is done
- **Not parallel**: every test task (T008, T009, T014, T015, T016) writes `src/about.test.ts`,
  and T011/T017 both write the same `<dialog>` in `index.html`. They are ordered deliberately;
  none carries a [P].

---

## Parallel Example: Phase 2 and User Story 1

```bash
# Foundational — the stylesheet is independent of the shell wiring:
Task: "T006 Add dialog layout rules to src/styles.css"

# User Story 1 implementation — different files:
Task: "T010 Re-export getVersion from @tauri-apps/api/app in src/api.ts"
Task: "T011 Add the static About content to <dialog id=\"about\"> in index.html"

# Polish — independent read-only assertions:
Task: "T021 Confirm git diff --stat -- src-tauri/ reports no changes"
Task: "T022 Confirm the three version sentinels are untouched"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1: Setup (T001–T002)
2. Phase 2: Foundational (T003–T007) — CRITICAL, blocks both stories
3. Phase 3: User Story 1 (T008–T013)
4. **STOP and VALIDATE**: open About in `pnpm tauri dev` and read the attribution (M1, M2)
5. This is already shippable — the request's first half, complete and correct

### Incremental Delivery

1. Setup + Foundational → the dialog opens and closes
2. Add User Story 1 → attribution and version visible → MVP
3. Add User Story 2 → the link opens the browser, suppresses repeats, fails visibly
4. Polish → prove the backend is untouched, then validate in the real app

---

## Judgment Calls Made In This Unattended Run

No user was present. These were decided here, with reasoning, and should be surfaced in the pull
request description.

1. **Tests are treated as explicitly requested.** The spec never says "TDD", and the tasks
   template makes tests optional — but quickstart.md §1 enumerates T1–T9 as the coverage this
   feature must land and plan.md lists `src/about.test.ts` as a new file. Read together that is
   an explicit request, so test tasks are generated and ordered before their implementation.

2. **Test assertions are grouped into five tasks, not nine.** All nine live in one file, so
   one-task-per-assertion would produce nine strictly sequential tasks and imply a parallelism
   that does not exist. They are grouped by concern (open/attribution/isolation, version,
   link identity, repeat suppression, failure ordering) so that each task still fails visibly on
   its own before its implementation lands.

3. **The dialog shell sits in Foundational rather than in US1.** US1 alone would otherwise own
   the markup US2 also needs, which would make US2 un-startable without US1 and break the
   independence the phase structure exists to provide. The cost is that Phase 2 ends with a
   dialog that opens onto nothing — accepted, and the checkpoint says so.

4. **T025 (the M8 forced-failure run) carries its own revert.** Editing a constant to make the
   address unopenable is the only practical way to exercise SC-005, and an edit left behind would
   ship a broken link. The revert and a re-run of `pnpm test` are inside the same task rather
   than in a follow-up, so the task cannot be marked done with the edit still present.

5. **`mountAbout` takes an injectable `now`.** T7/SC-004 needs ten activations inside one second;
   `open.ts:34-35` already established exactly this seam for exactly this reason, so `about.ts`
   threads it through rather than inventing a timer mock. Production passes nothing.

6. **No task exists for `src-tauri/` or for the capability file.** This is deliberate and is why
   T021 exists as a check rather than a change: `core:default` already grants
   `core:app:allow-version`. If implementation finds itself needing a Rust edit, that is a signal
   to revisit the plan, not a task to improvise.

---

## Judgment Calls Made During Implementation

Decided at the implement step, again with no user present. **These are the ones the pull request
description should surface.**

1. **The test fixture reads the shipped `index.html` rather than restating it.** T007 called for
   "a helper that builds the `index.html` About markup into the document", which reads as a
   hand-typed fixture. A hand-typed fixture would make T5 — that the link carries
   `data-open-url="https://clintparker.com"` *exactly* — assert only that the fixture agrees
   with itself; the shipped attribute could differ and every test would still pass. The fixture
   therefore imports `../index.html?raw` and extracts the real `<dialog>` and `#about-open`
   markup, so a drift between markup and behaviour fails the suite. This is the same instinct
   `open.test.ts` follows when it builds its rows with the real `render.ts`.

2. **One new file beyond the plan's list: `src/vite-env.d.ts`.** Consequence of (1). Reading the
   file with `node:fs` needs `@types/node`, which is not hoisted in this pnpm layout and which
   the plan forbids adding ("No dependency is added"). Vite's own `?raw` suffix needs no Node
   types — only the `vite/client` reference that this one-line file supplies. `vite` is already
   a devDependency, so **nothing new is installed.** It is the standard Vite scaffold file the
   project happened not to have.

3. **`main.test.ts` was extended, not left alone.** Its fixture predates the About markup, so
   `mountAbout` was reporting "The About dialog is missing from the page." into the banner on
   every run of main's suite — harmless only because `mountAutostart` overwrote it a moment
   later. The About markup was added to that fixture, `getVersion` to its `./api` mock, and one
   test asserting `main()` actually wires the dialog — which is the one thing `about.test.ts`
   structurally cannot see. No existing assertion was changed.

4. **The version is fetched *after* `showModal()`, not before.** The contract says "Fetches the
   version, then `showModal()`". Reversed deliberately: awaiting an IPC call before showing the
   dialog lets a slow or hung read hold the whole surface closed, and the version is the one
   thing on it nobody opened it for. `index.html` ships `Version unavailable` as the placeholder,
   so the line is never blank and the failure path needs no extra branch. The observable
   contract — dialog opens, version appears verbatim, failure degrades without a banner — is
   unchanged, and T3/T4 assert it.

5. **T4 spans both user stories, so it could not pass at the US1 checkpoint.** quickstart's T4
   requires a failed version fetch to leave "the attribution *and the link*" showing, but the
   link is US2's markup (T017). At T013 the version and attribution clauses passed and the link
   clause failed, exactly as written; T020 saw the whole of T4 green. Noted rather than worked
   around — weakening the assertion to make a checkpoint look clean would have cost the
   requirement.

6. **M8 was exercised by injecting the refusal, not by editing the link constant.** See T025.
   The task's own justification for the constant-edit was that it is "the only practical way" to
   force a refusal; in the browser harness it is not, and the harness route cannot leave a broken
   address behind. This is a **deviation from the written task** and is called out as one.

7. **Three manual checks are unrun and cannot be faked.** M3, M4's browser half, and M7 all
   require a real browser to come forward from the shipped app, and the Mac's screen is locked
   for the whole of this run. They are recorded as NOT done rather than inferred from the
   stubbed run, which quickstart §3 explicitly forbids. `/speckit-qa-run` on an unlocked screen
   is where they belong.
