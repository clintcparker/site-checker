# Quickstart: Validating Author Attribution in About

**Feature**: `20260813-175859-clint-parker-about` | **Date**: 2026-08-13

How to prove this feature works end to end. Scenario numbering maps to the spec's acceptance
scenarios and success criteria so QA can cite them directly.

See [`contracts/about-surface.md`](./contracts/about-surface.md) for the element and behaviour
contract, and [`data-model.md`](./data-model.md) for the exact strings asserted below.

---

## Prerequisites

```bash
cd /Users/clint/src/clintcparker/site-checker--20260813-175859-clint-parker-about
pnpm install          # this worktree starts with no node_modules
```

Rust toolchain as usual for `cargo`. macOS with a default browser configured.

---

## 1. Automated gates

The constitution's merge bar. All four must be green; the first two are where this feature's
own coverage lives.

```bash
pnpm test                              # frontend logic — the new About tests land here
pnpm exec tsc --noEmit                 # type check
cargo test  --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

**Expected**: all green. `cargo test` and `clippy` should be *unchanged* by this feature — it
adds no Rust. A diff in the Rust suites means something was added that the plan says should not
have been.

### What the frontend tests must cover

Written against `happy-dom`, which implements `showModal()`/`close()` as `open`-attribute
toggles — so dialog state is directly assertable. Follow the established `vi.mock("./api")`
convention from `src/open.test.ts:7`.

| # | Assertion | Requirement |
|---|---|---|
| T1 | Activating the footer control opens the dialog (`dialog.open === true`) | FR-001 |
| T2 | Dialog contains the exact string `Created by Clint Parker` | FR-004 |
| T3 | Version line renders the string `getVersion()` resolved with, verbatim | FR-003 |
| T4 | `getVersion()` rejecting still opens the dialog, still shows the attribution and link, and renders `Version unavailable` — no banner raised | FR-003, research R-005 |
| T5 | The link element carries `data-open-url="https://clintparker.com"` exactly | FR-005 |
| T6 | Activating the link calls `openUrl` once with that address; the page is not navigated | FR-006 |
| T7 | **Ten activations inside one second produce exactly one `openUrl` call** (inject `now` as `open.ts` allows, rather than waiting out a real second) | FR-008, SC-004 |
| T8 | An `openUrl` rejection closes the dialog *first*, then writes the message to the banner | FR-009, SC-005 |
| T9 | Opening and closing the dialog leaves `sites` and `statuses` untouched | FR-007, SC-006 |

T7 and T8 are the two most likely to be skipped and the two that carry named success criteria.

---

## 2. Manual validation in the real app

```bash
pnpm tauri dev
```

The version will read `0.0.0` here — correct and expected for a local build (research R-003).

| # | Steps | Expected | Maps to |
|---|---|---|---|
| M1 | From launch, find who made the app using only visible controls; time it | Under 15 seconds; the About control is visible in the footer without opening a menu | SC-001, FR-001 |
| M2 | Open About and read it | Shows `Site Checker`, a version line, and `Created by Clint Parker` — spelled exactly, no email or handle | US1-1, US1-2, FR-002/3/4 |
| M3 | Click the `clintparker.com` link once | Default browser comes forward at clintparker.com. Site Checker stays running; no dialog or confirmation step intervened | US2-1, SC-003, FR-006 |
| M4 | Reopen About and click the link ten times rapidly | Exactly one browser navigation / one new tab | US2-2, SC-004 |
| M5 | Close About; confirm the site list and its check schedule are as before | No site added, removed, re-scheduled, or re-checked | US1-3, FR-007 |
| M6 | Resize the window to its 480×320 minimum, then open About | Attribution readable and the link still activatable | FR-012 |
| M7 | Disconnect the network, then activate the link | The browser opens and reports its own failure. Site Checker shows no error of its own and keeps checking — it never tested clintparker.com | Edge case "machine is offline", FR-011 |

### M8 — forced open failure (SC-005)

Requires making the address impossible to open. `open_url` shells out to `/usr/bin/open` and
reports macOS's own words back, so the practical route is to temporarily point the link
constant at an address the guard or the OS refuses (e.g. an `ftp://` address, which
`openable_url` rejects by scheme) and rebuild.

**Expected**: within one second the dialog closes and the banner shows a plain-language
message; the app stays usable and every configured site keeps being checked on schedule — zero
missed checks. **Revert the constant afterwards.**

### M9 — config untouched (SC-006)

```bash
CFG=~/Library/Application\ Support/com.clintparker.site-checker/sites.json
shasum -a 256 "$CFG"      # before launching
# launch, open About, activate the link, quit
shasum -a 256 "$CFG"      # after
```

**Expected**: identical hashes — byte-for-byte unchanged (FR-010, SC-006).

> Note: the app has **no config-directory override**, so this reads and writes the real
> `sites.json`. Take a copy outside the repo before an M-series run rather than trusting a
> throwaway config dir, which does not exist.

---

## 3. Browser-driven alternative

If the Tauri window cannot be driven or captured (screen-recording or accessibility permission
refused), run the frontend under Vite instead and stub the boundary:

```bash
pnpm dev      # http://localhost:1420
```

Stub `window.__TAURI_INTERNALS__` so `invoke` resolves — return a version string for
`app|version` and record calls to `open_url` rather than performing them. This exercises every
DOM-level assertion (T1–T9, M1, M2, M6) in a real renderer.

**What it cannot prove**: M3, M4, and M7 — that a real browser actually comes forward. Those
require the shipped app and must not be reported as passing from a stubbed run.

---

## Done when

- [ ] All four automated gates green; the Rust suites unchanged by this feature
- [ ] T1–T9 present and passing
- [ ] M1–M7 observed in the real app
- [ ] M8 forced-failure run done and the constant reverted
- [ ] M9 hashes identical
- [ ] `Cargo.toml`, `package.json` and `tauri.conf.json` version invariants untouched (CI enforces)
