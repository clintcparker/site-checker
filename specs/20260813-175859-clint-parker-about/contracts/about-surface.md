# Contract: The About Surface

**Feature**: `20260813-175859-clint-parker-about` | **Date**: 2026-08-13

Site Checker is a desktop app with two interface boundaries worth pinning down: the Tauri
command boundary between the frontend and Rust, and the DOM contract between the modules that
render markup and the modules that wire behaviour to it. This feature changes exactly one of
them.

---

## 1. Tauri command boundary — UNCHANGED

**This feature adds no Tauri command, removes none, and changes no signature.** No `Site` or
`StatusEvent` field is added, renamed, or re-typed, so the snake_case contract of Constitution
Principle V is not engaged. `sites.json` keeps its documented shape.

Two existing pieces of the boundary are *consumed*:

### `open_url(url: String) -> Result<(), String>`

Already registered in `lib.rs`'s `invoke_handler` and already surfaced by `src/api.ts` as
`openUrl`. Called with the literal `https://clintparker.com`.

| Aspect | Contract |
|---|---|
| Argument | The full address, verbatim. Never `textContent` — the attribute is the source. |
| Success | Resolves `void`. macOS has accepted the address; a browser is coming forward. |
| Failure | Rejects with a bare string (Rust `Err(String)` arrives as the string itself, not an `Error`). Callers must `String(message)` it, as `open.ts:82` already does. |
| Guarantee relied on | `openable_url` returns an accepted address byte-identical, so `https://clintparker.com` opens as written — no trailing slash added, no re-rendering. |

### `getVersion(): Promise<string>` — newly surfaced through `api.ts`

Re-exported from `@tauri-apps/api/app`. **Permitted by the existing capability**: the project
grants `core:default`, which includes `core:app:default`, which the shipped crate documents as
including `allow-version`. **No edit to `src-tauri/capabilities/default.json` is required, and
none should be made** — widening a capability for a permission already granted is a review
finding, not a fix.

| Aspect | Contract |
|---|---|
| Returns | The running build's version string. `0.0.0` in dev and local builds — an inert sentinel that `release.yml` replaces from the pushed tag at build time. |
| Rendered as | Verbatim. No parsing, no prettifying, no hiding of `0.0.0`. |
| Failure | Rejects. The About view degrades to `Version unavailable` and still renders name, attribution, and link. It does not raise a banner and does not prevent the dialog opening. |

**Forbidden by CI**: `src-tauri/Cargo.toml` must stay `version = "0.0.0"`, `package.json` must
stay `"version": "0.0.0"`, and `tauri.conf.json` must carry no `version` key. `ci.yml:90-102`
fails the build on any of the three. Nothing in this feature may hand-edit a version.

---

## 2. DOM contract — the part this feature adds

The convention this codebase already uses: a rendering module owns the markup, a behaviour
module attaches by attribute selector, and the attribute *is* the contract between them.

### Structure

```text
<dialog id="about">                     opened with showModal(), closed with close()
  ├── app name          "Site Checker"                          FR-002
  ├── version line      "Version <x.y.z>" | "Version unavailable"  FR-003
  ├── attribution       "Created by Clint Parker"               FR-004
  ├── link  <button data-open-url="https://clintparker.com">clintparker.com</button>
  └── close control     <button data-about-close>              dismissal
```

### Element and attribute contract

| Selector | Owner | Contract |
|---|---|---|
| `#about` | `index.html` | The dialog. Must be a real `<dialog>` — `showModal()` supplies Esc-dismissal, focus containment and top-layer stacking, none of which this project reimplements. |
| `#about-open` | `index.html` (footer) | The control that opens it. Lives beside `#autostart` so it is visible at launch (FR-001, SC-001). |
| `[data-open-url]` | `index.html` | **Reused verbatim from the table's contract.** `mountUrlOpener` matches this selector via a delegated listener and is written against a generic `HTMLElement` container, so the dialog needs no change to `open.ts`. The attribute carries the whole address regardless of what the element renders. |
| `[data-about-close]` | `index.html` | Dismissal. Distinct from `data-action` and `data-open-url` so it cannot collide with the form's or the opener's delegated listeners — the same structural-separation reasoning recorded at `render.ts:188-191`. |

### Behavioural contract

| Behaviour | Requirement | Contract |
|---|---|---|
| Opening | FR-001 | Fetches the version, then `showModal()`. A version failure does not block the open. |
| Link activation | FR-006 | Delegates to `mountUrlOpener` → `openUrl`. The page is never navigated; nothing is rendered in-app. |
| Repeat suppression | FR-008, SC-004 | Supplied entirely by `open.ts`'s existing 1000 ms per-URL ledger. Ten activations inside one second ⇒ exactly one `openUrl` call. No second rule is written. |
| Open failure | FR-009, SC-005 | The dialog closes, **then** the message goes to the existing `#banner` via `showBanner`. Closing first is required: a modal covers the banner, and a message the user cannot see does not satisfy "visible". |
| Isolation | FR-007, SC-006 | The dialog reads no app state and mutates none. It never touches `sites`, `statuses`, or the scheduler, and issues no network request of its own (FR-011). |
| Minimum size | FR-012 | Content readable and the link activatable at the window's 480×320 minimum. Verified by observation during QA, not asserted in a unit test. |

### What must NOT appear

Stated because each is a plausible slip that would break a requirement:

- **No `<a href>`.** A real anchor can navigate the dashboard away from itself if the handler
  does not run, which is unrecoverable — the reasoning already recorded at `render.ts:184-187`.
  The button's cost, "button" rather than "link" in assistive technology, is accepted here for
  the same reason it was accepted there.
- **No `target="_blank"`, no `window.open`.** Opening is the backend's job (FR-006).
- **No email address, handle, or username.** FR-004 wants the name; the address in
  `Cargo.toml`'s `authors` is deliberately not surfaced (spec Assumptions).
- **No fetch, ping, or reachability check against clintparker.com.** FR-011. The site is not
  added to the check list and is never checked — offline, the browser reports its own failure.
- **No new capability entry, and no new permission.** Already granted; see §1.
