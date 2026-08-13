# Implementation Plan: Clickable URLs Open in the Default Browser

**Branch**: `20260813-105424-urls-clickable-and` | **Date**: 2026-08-13 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/20260813-105424-urls-clickable-and/spec.md`

## Summary

Make each site's URL in the dashboard table an activatable control that hands the
address to macOS to open in the user's default browser, without navigating the
dashboard's own webview.

The technical approach is a **first-party Tauri command over `/usr/bin/open`**,
not the `tauri-plugin-opener` plugin, mirroring the decision already recorded in
this repo for `auto-launch` over `tauri-plugin-autostart` (see
[research.md](./research.md) §1). The scheme guard demanded by FR-007 lives in a
pure, unit-tested Rust function (`openable_url`) beside `normalize_url`; the
command itself is a thin shell that spawns `open` and reports its exit status.

On the frontend the URL becomes a real `<button>` — not an `<a href>` — because
FR-003 treats a webview navigation as an unrecoverable state and a `<button>`
has no URL for the webview to follow under any modifier, middle-click, or
JS-failure path ([research.md](./research.md) §2). It carries a `data-open-url`
attribute, dispatched by a new delegated listener in `main.ts` (which owns the
banner FR-009 needs), deliberately keyed on a *different* attribute from the
`data-action` the row's Edit/Delete handler in `form.ts` matches, so FR-010
holds structurally rather than by convention.

## Technical Context

**Language/Version**: Rust (edition 2021, `tauri` 2.11) + TypeScript 5.6 (vanilla, no framework)

**Primary Dependencies**: `tauri` 2.11, `url` 2.5, `serde`; `@tauri-apps/api` ^2. **This feature adds no new dependency, in either lockfile.**

**Storage**: N/A — nothing about this feature is persisted. `sites.json` is read-only to this feature and its shape is unchanged.

**Testing**: `cargo test` (pure functions in `model.rs`); `pnpm test` → vitest + happy-dom (`src/*.test.ts`)

**Target Platform**: macOS desktop (Tauri v2 bundle, `targets: ["app"]`). macOS-only is a product constraint, not an accident — Constitution I.

**Project Type**: Desktop app — Rust backend (`src-tauri/src/`) + vanilla-TS frontend (`src/`), split by `living-specs.yml` into `backend` and `frontend` capabilities.

**Performance Goals**: Browser forward within 2 s of activation (SC-003). The open must not block the UI thread — hence `#[tauri::command(async)]` ([research.md](./research.md) §3).

**Constraints**: No webview navigation away from the dashboard (FR-003). No new persisted field. Repaint runs every 1000 ms and must not disturb focus or hover on the new control (FR-011, frontend living spec "Repainting must not disturb what the user is doing").

**Scale/Scope**: One user, one Mac, a handful of sites. Two new pure functions, one new command, one new frontend module, one new API-boundary function.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Checked against `.specify/memory/constitution.md` v1.0.0.

| Principle | Verdict | Evidence |
|---|---|---|
| **I. One Mac, One Person** | PASS | Adds no alerting, history, sync, or auth. It removes four steps from an interaction the user already performs by hand (SC-001). The spec explicitly declines a browser-preference setting (Assumptions) and scopes itself to the site table only. |
| **II. Results Are Ephemeral, Config Is Sacred** | PASS | Nothing is written. `sites.json` is neither read nor written by this feature's own code paths — it reads the already-loaded in-memory `Site`. FR-010 forbids altering stored data, schedule, or status. No "last visited" state (spec Assumptions). |
| **III. Be a Polite Client** | PASS | This app issues no HTTP request here; it hands a string to LaunchServices. Check behaviour, User-Agent, HEAD/GET discovery, and the interval floor are all untouched. Nothing a WAF could notice originates from this app. |
| **IV. Testable Core, Thin Shell** | PASS | The scheme guard is `openable_url(&str) -> Result<String, String>` in `model.rs` — pure, no `AppHandle`, no filesystem, tested by plain `cargo test`. The activation-dedupe rule is a pure TS function tested without a DOM. `open_url`'s process spawn and the DOM wiring stay thin, matching the `engine.rs` precedent for what may go unit-untested. |
| **V. The Rust/TS Contract Is snake_case, As-Is** | PASS | No persisted field and no event payload changes. `open_url` takes one command *argument*, `url` — a single lowercase word, so Tauri's camelCase↔snake_case argument conversion is a no-op on it and no naming trap is introduced. |

**Quality gates** (constitution "Quality Gates") are unchanged and all apply:
`cargo test`, `pnpm test`, `cargo clippy -- -D warnings` clean.

**Post-Phase-1 re-check**: PASS, unchanged. The Phase 1 design added no
dependency, no persisted field, no HTTP behaviour, and kept both new decision
functions pure. See [research.md](./research.md) §1 for the dependency decision
that keeps Principle I/IV honest — adopting `tauri-plugin-opener` would have
pulled a Linux DBus stack into a macOS-only personal app for one `open` call.

## Project Structure

### Documentation (this feature)

```text
specs/20260813-105424-urls-clickable-and/
├── plan.md              # This file
├── spec.md              # Input
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── open-url-command.md   # Tauri command contract
│   └── row-url-element.md    # DOM / interaction contract
├── checklists/          # Pre-existing
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
src-tauri/src/
├── model.rs         # CHANGED: + openable_url() (pure) + its unit tests
├── commands.rs      # CHANGED: + open_url command (thin shell over /usr/bin/open)
├── lib.rs           # CHANGED: register open_url in generate_handler!
├── store.rs         # untouched
├── check.rs         # untouched
├── engine.rs        # untouched
├── autostart.rs     # untouched
└── lock.rs          # untouched

src/
├── api.ts           # CHANGED: + openUrl(url) — the one typed backend boundary
├── render.ts        # CHANGED: name cell renders the URL as an activatable
│                    #          control; + isOpenable() guard; updateName handles
│                    #          the label-presence flip
├── open.ts          # NEW: mountUrlOpener() delegated listener + pure dedupe rule
├── main.ts          # CHANGED: mounts the opener, wired to showBanner
├── styles.css       # CHANGED: .site-url link affordance + :focus-visible ring
├── open.test.ts     # NEW
├── render.test.ts   # CHANGED: + URL-element cases
└── main.test.ts     # CHANGED: + open-failure banner case

index.html           # untouched — no new mount anchor is needed
capabilities/frontend/spec.md  # CHANGED by /speckit-companion-living-sync, not by hand
```

**Structure Decision**: The existing two-capability split (`src-tauri/src/**` =
backend, `src/**` + `index.html` = frontend, per `living-specs.yml`) is kept as-is.
This feature lands in both, and each side carries the half of the work that
matches its capability's stated purpose: the backend owns the privileged act of
handing an address to the OS and the authoritative scheme guard; the frontend
owns the affordance, the keyboard path, and the notice. No new module directory
is introduced — `open.ts` sits beside `form.ts` as a second thin shell over the
`api.ts` boundary, which is the shape `form.ts` already established.

## Complexity Tracking

> No Constitution Check violations. Table intentionally empty.

## Open Decisions for Review

No user was present for this run. These were decided here and should be
surfaced in the pull request description alongside the three already recorded in
[spec.md](./spec.md#open-decisions-for-review).

1. **A first-party command over `/usr/bin/open`, not `tauri-plugin-opener`.**
   The plugin was removed from this repo in `522367e` as unregistered scaffold
   residue, costing "an unused capability grant and a compiled crate"; its
   removal took 368 lines out of `Cargo.lock`, largely a Linux DBus stack this
   macOS-only app cannot use. Its scope allowlist would have to be
   `http://*` + `https://*` — any host — because the site list is user-supplied,
   so it would add no guard that FR-007's own check does not already provide.
   Revisit if the app ever targets a second platform. ([research.md](./research.md) §1)

2. **A `<button>`, not an `<a href>`.** An anchor is the more semantic element
   and gets keyboard activation free, but it carries a URL the webview can
   follow on a middle-click (`auxclick`, not `click`), a Cmd-click, or any path
   where the JS handler did not run — and FR-003 calls that outcome
   unrecoverable. A `<button>` has nothing to navigate to. Cost: assistive
   technology announces "button" rather than "link". ([research.md](./research.md) §2)

3. **The frontend repeats the scheme guard rather than asking the backend.**
   FR-007's second sentence ("A URL that is not opened MUST NOT be presented as
   activatable") is a *rendering* decision made 60 times a minute during
   repaint, so it cannot be an IPC round-trip. This creates a deliberate
   duplication of the http/https rule across `model.rs` and `render.ts`, of
   exactly the kind the frontend living spec already flags for the interval
   floor. Recorded as known duplication rather than hidden. ([research.md](./research.md) §4)

4. **Rapid repeats are suppressed by a 1000 ms per-URL window** (FR-012, a
   SHOULD). Chosen over disabling the control during the in-flight command,
   because the command resolves as soon as LaunchServices accepts the address —
   long before the browser is actually forward — so an in-flight guard would
   close a window that is effectively zero-width. 1000 ms is a judgement call:
   long enough to absorb a double-click, short enough that a user deliberately
   re-opening a page is not told "no". ([research.md](./research.md) §5)

5. **The name cell is rebuilt when a site's label is added or removed.**
   Adding a label moves the URL from the primary slot to the secondary one, so
   the element holding it changes role. Rather than thread that transition
   through `updateName`, the two child nodes are rebuilt on that one transition.
   This does not weaken FR-011: a label change is a user-initiated edit, not a
   repaint, and the repaint path (status arriving, age ticking) still touches
   nothing. ([research.md](./research.md) §6)
