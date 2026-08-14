# Implementation Plan: Author Attribution in About

**Branch**: `20260813-175859-clint-parker-about` | **Date**: 2026-08-13 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/20260813-175859-clint-parker-about/spec.md`

## Summary

Give Site Checker an About surface that names its author and links to his site. The app today
has one window, no custom menu, and no About view of its own; the request's "about window" has
to be established, not merely edited.

**Approach**: a small in-app About view — a native `<dialog>` in `index.html`, opened from a
button in the existing footer, wired by a new `src/about.ts`. It shows the app name, the
version read via `getVersion()`, the line "Created by Clint Parker", and an activatable
`clintparker.com` control that reuses `src/open.ts`'s existing opener — including its 1000 ms
per-URL repeat-suppression ledger, which is what FR-008 and SC-004 ask for by name.

**The system About panel was investigated and ruled out on evidence.** Tauri 2.11 renders it
through `muda` 0.19.3, whose macOS branch never reads `website`/`website_label` and wraps
`credits` in a plain `NSAttributedString` with no link attribute — so an activatable link is
not expressible there, and FR-005/FR-006 could not be met. It would also cost *more* code, since
`lib.rs` installs no menu today and taking that route means building a full macOS menu first.
Evidence and file/line citations are in [research.md](./research.md) R-001.

**Net shape**: frontend-only. No Rust changes, no new Tauri command, no capability edit, no new
persisted data, no new file the app owns.

## Technical Context

**Language/Version**: TypeScript ~5.6 (frontend, vanilla — no framework); Rust 2021 / Tauri 2.11
(backend, **untouched by this feature**)

**Primary Dependencies**: `@tauri-apps/api` v2 — specifically `core.invoke` (already used) and
`app.getVersion` (newly used, already permitted). No dependency is added.

**Storage**: N/A. FR-010 forbids reading, writing, or migrating `sites.json`, and forbids any
new file. Nothing here touches `store.rs`.

**Testing**: `vitest` + `happy-dom` for the frontend logic; `cargo test` and
`cargo clippy -D warnings` must stay green and, for this feature, **unchanged**.

**Target Platform**: macOS desktop (Tauri 2 app, single window)

**Project Type**: Desktop application — Rust core with a vanilla-TypeScript frontend

**Performance Goals**: None specific. The version fetch happens on first open; the dialog is
static markup. SC-005 sets the only timing bound — a failure message visible within one second.

**Constraints**: Content must remain readable and the link activatable at the window's minimum
size, 480×320 per `tauri.conf.json` (FR-012). No network request of the app's own (FR-011). The
version sentinel invariants in `Cargo.toml`, `package.json` and `tauri.conf.json` are enforced
by CI (`ci.yml:90-102`) and must not be edited.

**Scale/Scope**: One dialog, roughly six lines of static content, one new frontend module and
its test file. No NEEDS CLARIFICATION remains — all six were resolved in Phase 0.

## Constitution Check

*GATE: passed before Phase 0 research; re-checked after Phase 1 design — see below.*

| Principle | Assessment | Verdict |
|---|---|---|
| **I. One Mac, One Person** | Adds no alerting, notification, history, SLA math, sync, or auth-gated check. The spec asserts attribution is chrome around the product's one question, not a new capability — and FR-011 forbids the feature from checking clintparker.com, which is what would have widened scope. The constitution requires a scope change to be stated explicitly; this is stated as a non-change and the design holds to it. | PASS |
| **II. Results Are Ephemeral, Config Is Sacred** | No new file, no new persisted field, no read or write of `sites.json`. SC-006 is validated by a byte-for-byte hash comparison (quickstart M9). | PASS |
| **III. Be a Polite Client** | No request is issued at all. The address is handed to the browser; the app never fetches, pings, or checks it. Request behaviour is untouched — a WAF would see nothing new. | PASS |
| **IV. Testable Core, Thin Shell** | The one piece of real logic — repeat suppression — already exists as the pure, tested `shouldOpen` in `open.ts` and is reused rather than duplicated. `about.ts` is a thin DOM shell in the shape `form.ts` and `open.ts` already established. Choosing `getVersion()` over a new Rust command keeps the backend from growing a shell at all. | PASS |
| **V. The Rust/TS Contract Is snake_case** | Not engaged. No struct crosses the boundary, no field is added or renamed, no `serde` attribute is introduced. | PASS |
| **Quality Gates** | `cargo test`, `pnpm test`, `clippy -D warnings` all required green; the Rust suites should be *unchanged*, which is itself a check that no backend surface crept in. | PASS |

**Post-design re-check**: unchanged. The Phase 1 design added no entity, no command, no
capability, and no file the app owns; every gate above still holds against the concrete design
in [contracts/about-surface.md](./contracts/about-surface.md). **No violations, so Complexity
Tracking is empty.**

## Project Structure

### Documentation (this feature)

```text
specs/20260813-175859-clint-parker-about/
├── spec.md              # Input
├── plan.md              # This file
├── research.md          # Phase 0 — six resolved unknowns, with evidence
├── data-model.md        # Phase 1 — no entities; the fixed strings and transient values
├── quickstart.md        # Phase 1 — validation guide (T1-T9 automated, M1-M9 manual)
├── contracts/
│   └── about-surface.md # Phase 1 — Tauri boundary (unchanged) + the DOM contract
├── checklists/          # From the specify step
└── tasks.md             # Phase 2 — NOT created by /speckit-plan
```

### Source Code (repository root)

```text
index.html               # MODIFIED — <dialog id="about"> + footer #about-open button
src/
├── api.ts               # MODIFIED — re-export getVersion() from @tauri-apps/api/app
├── about.ts             # NEW      — mountAbout(): the dialog's thin shell
├── about.test.ts        # NEW      — T1-T9 from quickstart.md
├── main.ts              # MODIFIED — call mountAbout({ onError: showBanner }) in main()
├── open.ts              # UNCHANGED — mountUrlOpener reused as-is on the dialog
├── render.ts            # UNCHANGED
├── form.ts              # UNCHANGED
└── styles.css           # MODIFIED — dialog layout; readable at the 480×320 minimum

src-tauri/               # ENTIRELY UNCHANGED
├── src/…                #   no new command, no invoke_handler entry, no menu
├── capabilities/        #   core:default already grants core:app:allow-version
├── Cargo.toml           #   version sentinel must stay 0.0.0 (CI-enforced)
└── tauri.conf.json      #   must keep carrying no "version" key (CI-enforced)
```

**Structure Decision**: The existing single-project layout is kept exactly as it stands — a
Rust core under `src-tauri/` and a flat vanilla-TypeScript frontend under `src/`, each module
paired with a colocated `*.test.ts`. `about.ts` follows the convention `form.ts` and `open.ts`
established: a module that mounts one region of the page, takes its side effects as injected
hooks, and reaches the backend only through `api.ts`. No new directory is introduced; the
feature is small enough that one would be overhead.

That `src-tauri/` is listed above as *entirely unchanged* is a design assertion, not an
omission: if implementation finds itself editing Rust, a premise of this plan is wrong and the
plan should be revisited rather than worked around.

## Judgment Calls Made In This Unattended Run

No user was present. These were decided here, with reasoning, and should be surfaced in the
pull request description.

1. **In-app `<dialog>`, not the system About panel** — resolves the spec's Open Decision #1.
   Decided on evidence rather than preference: `muda` 0.19.3's macOS implementation cannot
   express an activatable link (research R-001, with file and line citations). This is the
   route the spec itself named as the safer bet if that turned out to be true. It did.

2. **The version is read via `getVersion()`, not a new Rust command** — the existing
   `core:default` capability already grants `core:app:allow-version` (confirmed in the shipped
   crate's own permission reference), so this needs no backend surface and no capability edit.
   A `get_app_info` command was the alternative and was rejected as more shell for a read of a
   constant (research R-003).

3. **A failed version fetch degrades to "Version unavailable" rather than raising a banner** —
   the dialog still opens and still shows the attribution and the link. A missing version stamp
   is not something the user can act on, and FR-001/FR-004/FR-005 are unaffected. The spec did
   not cover this case. If it should be louder, this is the line to change (research R-005).

4. **On a failed *open*, the dialog closes before the banner is shown** — a modal covers the
   banner, so writing the message behind it would satisfy the letter of FR-009 and fail its
   intent within SC-005's one-second bound. The spec did not specify the ordering.

5. **The About link keeps its own activation ledger, separate from the table's** — a
   consequence of `mountUrlOpener` closing over a per-mount `Map`. Deliberate: sharing one would
   let a site whose URL happens to be `https://clintparker.com` suppress the About link. SC-004
   concerns ten activations of the link itself, which one ledger delivers (research R-004).

6. **FR-003 (showing the version) is retained** — the spec's Open Decision #2 flagged it as an
   addition beyond the literal request. Kept, on the spec's own reasoning that a version-less
   About surface is not one users recognise. Nothing else depends on it, so it remains cheap to
   drop.

7. **The attribution reads "Created by Clint Parker"** — the spec's Open Decision #3 assumed
   phrasing, retained. Alternatives satisfy FR-004 equally as long as the name is spelled
   exactly.

## Complexity Tracking

No Constitution Check violations. Nothing to justify.
