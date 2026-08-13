# Phase 0 Research: Clickable URLs Open in the Default Browser

Six questions had to be settled before the design could be written. None of them
was left as NEEDS CLARIFICATION.

---

## 1. How does the app hand a URL to the default browser?

**Decision**: A first-party Tauri command, `open_url`, that spawns
`/usr/bin/open` with the address and reports its exit status. No new crate, no
new npm package, no new capability permission.

**Rationale**:

- `tauri-plugin-opener` was already in this repository and was **deliberately
  removed** in `522367e` ("chore: remove the unregistered opener plugin"). That
  commit's own reasoning — "it still cost an unused capability grant and a
  compiled crate in the binary" — applies with equal force to putting it back
  for one call. Its removal deleted **368 lines from `Cargo.lock`**, including
  `zbus`/`async-broadcast` and the rest of a Linux DBus stack that a macOS-only
  app (Constitution I: "one person on one Mac") can never execute.
- The repository has an explicit precedent for exactly this trade. `Cargo.toml`
  documents choosing `auto-launch` directly over `tauri-plugin-autostart`
  because "the plugin was a thin wrapper over this same crate, so the dependency
  count is unchanged." Here the direct route is *cheaper* than that: it needs no
  crate at all, because `/usr/bin/open` ships with macOS.
- The plugin's security story does not apply. Its scope allowlist would have to
  read `{"url": "http://*"}, {"url": "https://*"}` — every host on the web —
  because the site list is whatever the user typed. The guard that actually
  matters for FR-007 is the scheme check, and that is ours to write either way.
- `/usr/bin/open` is spelled absolutely rather than as `open`, so the launched
  binary does not depend on the inherited `PATH`.

**Alternatives considered**:

| Option | Why rejected |
|---|---|
| `tauri-plugin-opener` (official, `openUrl()` in JS) | Reinstates a dependency this repo removed on stated grounds; drags a Linux DBus tree into a macOS-only bundle; its URL scope allowlist would be `*` and so guards nothing. |
| The `opener` crate directly | Cross-platform veneer with no cross-platform requirement. On macOS it does what `/usr/bin/open` does, for the price of a dependency. |
| `NSWorkspace.openURL` via `objc2` | The correct native call, and materially heavier: an Objective-C bridge crate and unsafe blocks, to replace a one-line process spawn in an app that is not latency-sensitive here (SC-003 allows two seconds). |
| `<a href target="_blank">` and let the webview handle it | WKWebView has no default external-link behaviour; `window.open` is a no-op or a new in-app webview. Does not reliably reach the default browser at all. See §2. |

**Failure reporting**: `/usr/bin/open` exits non-zero and writes to stderr when
LaunchServices can resolve no handler — which is precisely the "no default
browser is configured" edge case the spec lists. `Command::output()` gives both
the status and the stderr text, so FR-009's message has something true to say
rather than a generic apology.

---

## 2. What element makes the URL activatable without navigating the webview?

**Decision**: A `<button class="site-url" data-open-url="…">`, styled to read as
a link.

**Rationale**: FR-003 is unusually strong for a "don't navigate" requirement —
the spec spells out that "the dashboard offers no way back, so navigating it
away would strand the user in an unrecoverable state." An element with no `href`
cannot be followed. An `<a href>` can be, on at least three paths that a single
`click` handler with `preventDefault()` does not cover:

- **Middle-click** fires `auxclick`, not `click`. The delegated handler never sees it.
- **Cmd-click / Ctrl-click** on macOS asks for a new window; in a Tauri webview
  the outcome is platform-dependent and not something to bet an unrecoverable
  state on.
- **Any path where the handler did not run** — a JS error earlier in the module,
  a click landing before `mountUrlOpener` executes — degrades an anchor into a
  live navigation, and degrades a button into an inert one.

A `<button>` also arrives with everything FR-005 asks for at no cost: it is in
the tab order without a `tabindex`, it activates on both Enter and Space, and it
takes a native `:focus-visible` ring. And it matches what the codebase already
does — `render.ts`'s existing `button()` helper builds the Edit and Delete
controls the same way, dispatched by the same delegated-listener pattern.

**Alternatives considered**:

| Option | Why rejected |
|---|---|
| `<a href={url}>` + `preventDefault()` | Semantically the best fit and the usual web answer, but leaves the three navigation paths above open against a requirement that calls the failure unrecoverable. Recorded as an open decision in `plan.md` — this is the choice to revisit if "announced as a button" proves worse in practice than the residual risk. |
| `<a>` with no `href`, `role="link"`, `tabindex="0"` | Gets the "link" announcement back and removes the navigation risk, but hand-rolls keyboard activation (an `href`-less anchor does not activate on Enter) and the pointer cursor — reimplementing a `<button>` badly. |
| `<span>` + click handler | Fails FR-005 outright: not focusable, not keyboard-activatable, no affordance. |

**Cost accepted**: assistive technology announces "button" where "link" would be
more informative. Mitigated by the accessible name being the full URL itself,
which is what the user is choosing.

---

## 3. Will the open block the UI?

**Decision**: Declare the command `#[tauri::command(async)]`.

**Rationale**: The Tauri v2 documentation is explicit — "Commands without the
_async_ keyword are executed on the main thread unless defined with
`#[tauri::command(async)]`", and "Async commands are executed on a separate
async task using `async_runtime::spawn`." Waiting on `/usr/bin/open` to exit is
a blocking wait on a child process; on the main thread that is a visible stall
of the whole window, including the repaint timer.

`#[tauri::command(async)]` on a *synchronous* function is the right form here
rather than an `async fn`: the body is ordinary blocking I/O, `tokio`'s
`process` feature is not enabled in this project's dependency set, and enabling
it to gain `tokio::process` would be a dependency change to solve a problem that
`spawn` already solves.

**Note on the existing code**: `set_autostart` is a plain synchronous command
that writes a plist, so it does block the main thread today. That is not a
reason to copy it — it is a rare, user-initiated toggle, whereas this is a
control the user may hit repeatedly while a 1 s repaint timer is running.

**Alternatives considered**: `Command::spawn()` without waiting — non-blocking
and simple, but it succeeds as soon as the *binary* is found, so it cannot
distinguish "browser opening" from "no handler for http", and FR-009/SC-006
would have nothing to report.

---

## 4. Where does the http/https guard live?

**Decision**: In both places, deliberately, with the backend authoritative.

- `openable_url` in `model.rs` — the enforcement. Nothing reaches `/usr/bin/open`
  without passing it.
- `isOpenable` in `render.ts` — the presentation. Decides whether a row's URL is
  rendered as a `<button>` or as inert text.

**Rationale**: FR-007 is two requirements in one sentence. "MUST NOT open" is
enforcement and belongs where the privileged act happens. "MUST NOT be presented
as activatable" is a rendering decision taken on every repaint — up to once a
second per row — and an IPC round-trip per row per second to answer it would be
absurd, and would still be answering asynchronously a question the synchronous
render needs now.

**The duplication is acknowledged, not hidden.** The frontend living spec
already carries a `> **Known duplication.**` callout for the interval floor,
which is the same shape of problem: one rule, independently spelled in Rust and
in TypeScript, with nothing keeping them in step. This one is lower-risk than
the interval floor — the rule is "the scheme is `http` or `https`", it is fixed
by the HTTP requirement of the product itself, and a drift would make the UI
*offer* something the backend then refuses, which surfaces as FR-009's visible
message rather than as silence.

**Why not reuse `normalize_url`?** Different contract. `normalize_url` *repairs*:
it prepends `https://` to a scheme-less string, so a hand-edited
`sites.json` entry reading `/etc/passwd` would become `https:///etc/passwd`
rather than being refused as the edge case intends (it would then fail the host
check, but by accident, not by design). `openable_url` must never repair — it
validates a stored value and either returns it byte-identical (FR-006) or
refuses. Separate function, separate tests, adjacent in the file so the contrast
is visible.

---

## 5. What does "repeated rapid activations… SHOULD result in a single navigation" mean concretely?

**Decision**: Suppress a repeat activation of the *same* URL within **1000 ms**
of the previous accepted one. Per-URL, not global — two different sites clicked
in quick succession are two intentional opens.

**Rationale**: The spec reads the behaviour correctly as "a user expressing
impatience, not a request for many browser tabs." 1000 ms comfortably covers a
double-click (macOS's own double-click interval is 500 ms by default) and the
"nothing happened yet, click again" reflex, while staying short enough that
deliberately re-opening a page a second later works.

**Alternatives considered**:

| Option | Why rejected |
|---|---|
| Disable the control while the command is in flight | The window is effectively zero-width: `open` returns as soon as LaunchServices accepts the address, which is well before the browser is forward. It would guard almost nothing while adding a disabled state to reason about across repaints. |
| A global (not per-URL) cooldown | Punishes the legitimate case of opening two different sites in a row. |
| Do nothing — FR-012 is a SHOULD | Cheap to satisfy with a `Map<string, number>` and a pure predicate; leaving a stated requirement unmet to save four lines is a poor trade. |

**Testability**: expressed as a pure function taking the ledger, the URL, and
`now`, so the rule is tested with plain values and no timers.

---

## 6. How does the new element survive the 1 s reconciliation?

**Decision**: Follow the existing `renderRow`/`updateRow` contract exactly —
update in place, never recreate — with one explicitly scoped exception: the name
cell's children are rebuilt when a site gains or loses its label.

**Rationale**: The name cell's structure is positional. With no label, child 0
holds the URL. With a label, child 0 holds the label and child 1 holds the URL.
So toggling a label *moves which node is the URL*, changing that node's role and
element type. Threading that transition through `updateName` in place would mean
mutating a `<span>` into a `<button>` and back, which the DOM does not support —
it would be a replacement either way, just a less legible one.

Rebuilding on that transition is safe against FR-011 and against the frontend
living spec's repaint requirement, because both are about *repaints*: a status
event arriving, or the age counter ticking. Neither changes a label. A label
change is a user-initiated save that has already left the row, so no hover or
focus is in progress on the element being replaced.

**The repaint path itself touches nothing new.** `formatSince` ticking and a
status arriving do not read the URL, so the URL element's `textContent`,
`className`, and `data-open-url` are all written through the existing
change-guarded `setText`/`setClass` helpers and stay untouched between edits —
which is what keeps focus (US2 scenario 3) and hover (FR-004) intact.

**Scheme transitions**: a row can only go from inert to activatable, never back,
because the only runtime path that changes a stored URL is an edit, and
`update_site` runs `normalize_url`, which refuses a non-http/https scheme. An
inert entry can therefore only originate from a hand-edited file at load time.
The rebuild rule above covers the one-way transition without needing a special
case.

---

## Sources

- Tauri v2, *Calling Rust from the Frontend* — https://v2.tauri.app/develop/calling-rust/ (command threading; `#[tauri::command(async)]`)
- Tauri v2, *Opener plugin* — https://v2.tauri.app/plugin/opener/ (permissions, `opener:allow-open-url` scope, `openUrl()`)
- This repository: commit `522367e` (opener plugin removal and its stated cost), `src-tauri/Cargo.toml` (the `auto-launch`-over-plugin precedent), `.specify/memory/constitution.md` v1.0.0, `capabilities/frontend/spec.md`.
