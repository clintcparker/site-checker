# Phase 0 Research: Author Attribution in About

**Feature**: `20260813-175859-clint-parker-about` | **Date**: 2026-08-13

This run was unattended. Every question below was settled by reading the shipped
dependency sources and the repository's own code rather than by asking, and each entry
records the evidence so a reviewer can re-check the call rather than take it on trust.

---

## R-001: Which surface carries the About content

**Question**: Spec Open Decision #1 — enrich the macOS system-supplied About panel, or add a
small in-app About view? The spec is satisfied either way, and explicitly deferred the choice
to planning "because both satisfy every requirement above and the choice turns on
implementation constraints".

**Decision**: **An in-app About view.** The system panel is ruled out on evidence, not taste.

**Rationale**: FR-005 and FR-006 require an *activatable* link that hands the address to the
default browser. The macOS system About panel cannot carry one.

Tauri 2.11 renders `PredefinedMenuItem::about` through `muda` 0.19.3 (confirmed in
`src-tauri/Cargo.lock:2088`). Reading that crate's macOS implementation
(`muda-0.19.3/src/platform_impl/macos/mod.rs:1060-1113`), the `AboutMetadata` fields it
forwards to `orderFrontStandardAboutPanelWithOptions` are exactly: `name`, `version`,
`short_version`, `copyright`, `icon`, and `credits`. Two consequences decide this:

1. **`website` and `website_label` are never read on macOS.** They appear in `AboutMetadata`
   but the macOS branch does not reference them — they are GTK-only. There is no field that
   puts a URL on the panel.
2. **`credits` is stripped of link attributes.** It is wrapped with
   `NSAttributedString::from_nsstring(&NSString::from_str(credits))` — a *plain* attributed
   string with no `NSLinkAttributeName` run. A URL placed in `credits` therefore renders as
   inert text. It would satisfy FR-004 but fail FR-005 ("presented as something activatable,
   not as bare unclickable text") and FR-006 outright.

The system route also costs *more* code, not less: `lib.rs` installs no menu today, so taking
it would mean building and registering a full macOS menu (the default menu disappears the
moment a custom one is set) purely to reach a panel that still could not hold the link.

**Alternatives considered**:

- *System About panel via `AboutMetadata`* — rejected above. Fails FR-005/FR-006 on macOS.
- *System panel for name/version + link elsewhere* — rejected: splits one About surface into
  two places, and SC-001 ("find out who made it in under 15 seconds") is served by one
  surface, not a scavenger hunt.
- *A second Tauri window for About* — rejected: a whole `WebviewWindow`, its own HTML entry
  point and lifecycle, for six lines of static text. Principle IV ("thin shell") points the
  other way, and a second window would need its own capability entry.

**Consequence for the spec's Open Decision #1**: resolved to the in-app route, which the spec
itself named as "the safer bet if the system panel turns out not to support an activatable
link". It does not.

---

## R-002: How the About view is presented inside the one window

**Decision**: A native `<dialog>` element in `index.html`, opened with `showModal()`.

**Rationale**: It is the platform's own modal primitive — Esc-to-dismiss, focus containment,
inertness of the page behind it, and the top-layer stacking all come from the browser engine
rather than from code this project would own and test. No dependency is added; the project has
no UI framework (Principle IV: "vanilla TypeScript, no framework") and this keeps it that way.

**Verified testable**: `happy-dom` (the configured vitest environment) implements
`HTMLDialogElement` — `node_modules/happy-dom/lib/nodes/html-dialog-element/HTMLDialogElement.js`
defines `showModal()` (sets the `open` attribute) and `close()` (removes it, fires `close`).
So `dialog.open` is directly assertable in vitest and this choice costs no test coverage. This
was checked before committing to it, because a `showModal()` that threw under happy-dom would
have forced a plain `hidden`-toggled `<div>` instead.

**Alternatives considered**:

- *A `hidden`-toggled `<div>` overlay* — rejected: re-implements Esc handling, focus trapping
  and stacking by hand, all of which are behaviour worth not owning.
- *An always-visible footer credit line, no dialog* — rejected: FR-001 asks for an About
  *surface* the user opens; a permanent footer line also competes for space at the 480×320
  minimum window size that FR-012 constrains.

---

## R-003: Where the version comes from

**Decision**: `getVersion()` from `@tauri-apps/api/app`, re-exported through `src/api.ts`.
**No new Rust command, and no capability edit.**

**Rationale**: Two candidate routes; the frontend one needs strictly less new surface.

- The repository's capability `src-tauri/capabilities/default.json` grants `core:default`,
  which includes `core:app:default`. That set is documented in the shipped crate
  (`tauri-2.11.5/permissions/app/autogenerated/reference.md`) as including **`allow-version`**
  and `allow-name`. So `getVersion()` is already permitted — nothing to add, nothing to widen.
- The alternative — a new `get_app_info` Tauri command over `app.package_info()` — would add a
  command, an `invoke_handler` entry, and a Rust surface for what is a read of a constant.
  Principle IV wants shells thin; this is the thinner one.

Routing it through `api.ts` rather than importing `@tauri-apps/api/app` directly in the About
module keeps the single-boundary convention every other call already follows, and it is what
makes the module mockable with the established `vi.mock("./api")` pattern
(`src/open.test.ts:7`).

**Version value in practice** (Edge Case "a development or unreleased build"):
`src-tauri/Cargo.toml` pins `version = "0.0.0"` as an inert sentinel — `.github/workflows/release.yml`
stamps the real version from the pushed tag at build time, and `tauri.conf.json` deliberately
carries no `version` key (CI enforces all three of these invariants at `ci.yml:90-102`). A local
or dev build therefore legitimately reports `0.0.0`. That is a legible value and is rendered
as-is; the requirement is that the surface not show a blank where the version belongs, and
`0.0.0` is not a blank. No special-casing, and nothing here may hand-edit those sentinels.

**Failure handling**: `getVersion()` is an `invoke` and can in principle reject. The About view
must still open and still show the attribution and the link if it does — the version is the
only part that degrades. Resolved as R-005.

---

## R-004: How the link opens, and how repeat activation is suppressed

**Decision**: Reuse `src/open.ts` unchanged — `mountUrlOpener` on the dialog, with the link
carrying `data-open-url="https://clintparker.com"`.

**Rationale**: FR-008 requires the About link to fall under "the app's existing
repeat-suppression window", and SC-004 fixes it at ten activations inside one second producing
exactly one navigation. `open.ts` already implements precisely that: `ACTIVATION_WINDOW_MS =
1000`, a per-URL `ActivationLedger`, and the deliberate rule that a *suppressed* activation does
not refresh the entry (`src/open.ts:46-51`). Writing a second, parallel rule for one link is
exactly what the spec's assumption "rather than to invent a second, differing rule" forbids.

`mountUrlOpener` is already written against a generic `HTMLElement` container with a delegated
listener matching `[data-open-url]` — it is not table-specific. Mounting a second instance on
the dialog needs no change to `open.ts` at all.

**One deliberate consequence, recorded**: each `mountUrlOpener` call closes over its *own*
`ledger` (`src/open.ts:63`). The table's ledger and the dialog's are separate. This is correct
here and not a defect: the ledger's stated purpose is absorbing a double-click on one control,
and no user double-clicks a table row and the About link as one gesture. Sharing a ledger would
also mean a site whose URL happens to be `https://clintparker.com` could suppress the About
link, which is worse. SC-004 is about ten activations of *the link*, and one ledger delivers it.

**Backend acceptance confirmed**: `openable_url` (`src-tauri/src/model.rs:138`) accepts
`https://clintparker.com` — non-empty, parses, `https` scheme, non-empty host — and returns it
byte-identical, so the address opened is exactly the one specified in FR-005. No backend change
is required for this feature at all.

**Alternatives considered**:

- *An `<a href>` with a target* — rejected. `render.ts:184-194` records why this codebase uses
  a `<button>` and not an anchor: a real anchor navigates the dashboard away from itself if the
  JS handler ever fails to run, which is unrecoverable. The same reasoning applies verbatim
  inside the dialog. The cost — assistive technology announcing "button" — is accepted here for
  the same reason it was accepted there.
- *Calling `openUrl()` directly from a bespoke click handler* — rejected: that is the second
  differing rule, and it would fail SC-004 unless the ledger logic were duplicated.

---

## R-005: Where a failure surfaces while the dialog is open

**Decision**: Reuse the existing banner via `showBanner`, and **close the dialog before the
message is shown** on an open failure.

**Rationale**: FR-009 requires a refusal to be *visible*. The banner lives at the top of the
page body (`index.html:9`), which a modal `<dialog>` covers — a message written to a banner
behind an open modal satisfies the letter of "shown" and fails the intent, and SC-005 puts a
one-second bound on the user seeing it. Closing first makes the message the thing the user is
looking at. `mountUrlOpener`'s `onError` hook is the seam: the dialog passes a callback that
closes itself and then calls `showBanner`.

The same applies to a rejected `getVersion()` (R-003), with one difference: that failure must
not close or block anything. The dialog opens regardless and shows the attribution and link;
the version line degrades to a plain "Version unavailable" rather than a blank, and no banner is
raised — a missing version stamp is not something the user can act on, and FR-001/FR-004/FR-005
are all still met. Recorded as a judgment call in plan.md.

---

## R-006: Discoverability of the About control

**Decision**: A text button in the existing `<footer>`, beside the "Launch at login" checkbox.

**Rationale**: FR-001 requires "an ordinary, discoverable control" and SC-001 sets 15 seconds
from launch using only visible controls. The footer is the app's existing home for
app-level (not site-level) affordances — it already holds the only other one — and it is on
screen at launch with no menu traversal. A menu item is ruled out by R-001: there is no custom
menu, and adding one to host a single item is disproportionate.

**Sizing note for FR-012**: the footer's `.setting` rule is `display: inline-flex`
(`src/styles.css:177`), so a second inline control sits beside the checkbox and wraps below it
when the window is at its 480px minimum (`tauri.conf.json` `minWidth: 480`). The dialog's own
content must be checked at 480×320 during QA rather than assumed.

---

## Resolved unknowns

Every "NEEDS CLARIFICATION" raised against the Technical Context is closed above:

| Unknown | Resolved by | Answer |
|---|---|---|
| Which About surface | R-001 | In-app `<dialog>`; system panel cannot hold an activatable link |
| Modal mechanism, and is it testable | R-002 | Native `<dialog>`; happy-dom implements `showModal()` |
| Version source and permission | R-003 | `getVersion()` via `api.ts`; `core:default` already grants it |
| Repeat-suppression mechanism | R-004 | Reuse `open.ts` `mountUrlOpener` unchanged |
| Where failures surface | R-005 | Existing banner, dialog closed first |
| How the user reaches About | R-006 | Footer button beside "Launch at login" |

No unknown remains open, and no NEEDS CLARIFICATION is carried into Phase 1.
