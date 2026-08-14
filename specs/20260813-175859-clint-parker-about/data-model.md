# Phase 1 Data Model: Author Attribution in About

**Feature**: `20260813-175859-clint-parker-about` | **Date**: 2026-08-13

## Summary

**This feature introduces no entities.** The spec states it outright ("Key Entities: None"),
and FR-010 makes it a requirement rather than an observation: the feature must not read, write,
or migrate the user's saved site list, and must not introduce any new file the app owns.

That constraint is worth restating in model terms because it is the thing most likely to be
violated by accident:

- **No new persisted field.** `sites.json` keeps its documented shape — a bare JSON array of
  `Site` objects with snake_case keys (Constitution Principle II). Nothing in this feature
  touches `store.rs`, the store's on-disk format, or its load/save paths.
- **No new Tauri command, and no change to an existing one.** The `Site` and `StatusEvent`
  structs that cross the boundary are untouched, so the snake_case contract of Principle V is
  not engaged at all. See `contracts/about-surface.md`.
- **No new user-editable value.** There is no setting, no preference, and no state that
  survives the dialog closing.
- **No new file.** The About view reads a compiled-in version string and renders fixed text.

## Constants introduced

Not entities — fixed values compiled into the frontend bundle. Listed so the tasks step has one
place to look for the exact strings the acceptance scenarios assert on.

| Constant | Value | Requirement | Notes |
|---|---|---|---|
| Author name | `Clint Parker` | FR-004 | Exact spelling and capitalisation is a requirement. Not an email, handle, or username — the address in `Cargo.toml`'s `authors` field is deliberately not surfaced (spec Assumptions). |
| Attribution phrasing | `Created by Clint Parker` | FR-004 | Spec Open Decision #3 assumed this phrasing; retained. Any alternative must still spell the name exactly. |
| Author site address | `https://clintparker.com` | FR-005 | Secure scheme, apex domain, no path, no trailing slash. This exact string is what `data-open-url` carries and what `openable_url` returns byte-identical. |
| Link text | `clintparker.com` | FR-005 | Displayed without the scheme so the destination reads unambiguously as clintparker.com; the full address travels in `data-open-url`, per the attribute-not-`textContent` rule already established in `open.ts:70-73`. |
| App name | `Site Checker` | FR-002 | Matches `productName` in `tauri.conf.json`. Rendered as a literal — it is the window title and the bundle name, not a runtime variable. |

## Transient runtime values

Held in memory for the lifetime of the window, never persisted:

| Value | Type | Lifetime | Source |
|---|---|---|---|
| App version | `string` | Fetched when the About dialog is first opened | `getVersion()` via `src/api.ts` (FR-003). `0.0.0` in dev/local builds by design — see research R-003. |
| Version-fetch failure | degraded render | Same | Renders `Version unavailable` rather than a blank; does not block the dialog or raise a banner (research R-005). |
| Activation ledger | `Map<string, number>` | Lifetime of the mounted dialog opener | Owned by `mountUrlOpener` (`src/open.ts:63`), already existing. One `https://clintparker.com` → timestamp entry at most. Separate from the table's ledger by design (research R-004). |

## State transitions

The only state in this feature is whether the dialog is open. It is held by the DOM element's
own `open` property, not by a module-level variable:

```text
closed ──(About button activated)──▶ open ──(Close button / Esc / backdrop)──▶ closed
                                      │
                                      └──(link activated and OS refuses)──▶ closed + banner
```

Acceptance scenario US1-3 constrains the whole diagram from the outside: no transition here may
add, remove, re-schedule, or re-check a site (FR-007). The dialog neither reads nor writes
`sites`, `statuses`, or the scheduler — it is rendered from constants and one version string.

## Validation rules

None. There is no user input in this feature — no field to validate, no address to normalize.

The one address involved is a compile-time constant, and it is validated on the way out by the
existing backend guard `openable_url` (`src-tauri/src/model.rs:138`) like any other address the
app opens. Research R-004 confirmed it passes that guard and is returned byte-identical, so no
new validation code is warranted and none should be added.
