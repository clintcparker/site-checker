# Phase 1 Data Model: Durability & Data Integrity

**Feature**: `specs/003-durability` | **Date**: 2026-08-06

No field is added, removed, renamed, or retyped by this feature. What changes is the set of
*invariants* two existing entities are guaranteed to hold, plus one new transient artifact on
disk that is never part of the user's data. Constitution V (the snake_case contract) is
untouched by construction: nothing here alters a serialized name or the shape of the array.

---

## Entity: `Site`

`src-tauri/src/model.rs`. **Shape unchanged.**

| Field | Type | Serialized as | Change in this feature |
|---|---|---|---|
| `id` | `String` | `id` | none to the field; gains invariant **I1** |
| `url` | `String` | `url` | none to the field; gains invariant **I2** |
| `label` | `Option<String>` | `label`, omitted when `None` | none |
| `interval_secs` | `u64` | `interval_secs` | none |
| `method_override` | `Option<Method>` | `method_override`, `"GET"` when set | none |

### New invariants

- **I1 — `id` is unique within a store.** Enforced at the point of entry: `Store::add`
  refuses a site whose `id` is already present. This is a guarantee about the *collection*,
  not about the type — a `Site` value on its own carries no uniqueness claim, and nothing
  validates ids at load time (see Non-invariants).
- **I2 — `url`'s scheme is lowercase at the moment it is created or edited.** Enforced by
  `normalize_url`, which every write path already funnels through (`add_site` and
  `update_site` both call it before constructing the `Site`). Everything after the scheme —
  host case, path case, query string — is the user's text verbatim.

### Non-invariants (deliberate, per the spec's Assumptions)

- **Loading does not enforce I1 or I2.** A `sites.json` written before this feature may hold
  `HTTPS://example.com`, and it is loaded, listed, and checked exactly as it is. It becomes
  compliant only when the user next edits that site. There is no migration pass — adding one
  would mean writing to the store at startup, which is itself the durability risk this
  feature exists to reduce.
- **Host case is not normalized.** `https://EXAMPLE.com` keeps its host. Only the scheme is
  in scope.

---

## Entity: `Site list file`

`~/Library/Application Support/com.clintparker.site-checker/sites.json`.

**Format unchanged**: a bare JSON array of `Site`, pretty-printed, snake_case keys. **Location
unchanged. Load semantics unchanged.** The only thing that changes is *how the file is
replaced*.

### States as a reader sees them

A reader is `load()`, and it only ever opens the `sites.json` path. From its point of view the
file is in exactly one of these states — there is no partial state, which is the whole point
of FR-001:

| State | How `load()` behaves | Changed? |
|---|---|---|
| Absent | empty list, no warning | no |
| Complete previous contents | that list, no warning | no |
| Complete new contents | that list, no warning | no |
| Unreadable (permissions, I/O) | empty list + warning, file untouched | no |
| Unparseable (corrupt from before this feature) | empty list + warning, file left on disk | no |
| **Partially written** | — | **eliminated** |

The bottom row is the deliverable. Every other row is a behaviour that already exists and that
FR-005 requires be preserved byte-for-byte; the existing
`corrupt_file_yields_an_empty_list_a_warning_and_is_left_on_disk` test is its pin.

### Transitions during a save

```
              stage                          rename
  [ previous ] ─────► [ previous ]  ────────────────────► [ new ]
   sites.json          sites.json                          sites.json
                       + sites.json.tmp (new contents)      + no .tmp
```

Interruption at any point in the left arrow leaves the middle state: `sites.json` complete and
previous, plus one orphaned `.tmp`. Interruption during the right arrow is not observable —
`rename(2)` is atomic, so a reader sees either the left inode or the right one.

---

## Entity: `Staging artifact` *(new, transient, not user data)*

| Property | Value | Requirement |
|---|---|---|
| Path | `sites.json`'s own directory, name `sites.json.tmp` | FR-003 (same directory — a cross-filesystem rename is not atomic) |
| Lifetime | created by the staging step, consumed by the rename | — |
| Content | the complete new list, byte-identical to what `sites.json` will hold | — |
| Read by | nothing — `load()` opens only the `sites.json` path | FR-003 ("MUST NOT be readable as the site list") |
| Max count | **one**, structurally: the name is fixed, so the next save truncates and reuses any orphan rather than adding a second | FR-003 / SC-005 |
| Cleaned up on load? | **no** — see research R8 | FR-005 (load is unchanged) |

An orphan is the residue of an edit that was lost. It is left alone: it is bounded at one, it
costs a few hundred bytes, and it is a breadcrumb rather than a liability. Recovering from it
would be version history, which the spec puts out of scope.

---

## Behaviour contracts that the entities depend on

Detailed in [`contracts/`](./contracts/):

- [`store-write-path.md`](./contracts/store-write-path.md) — `add` / `update` / `delete` /
  staging / `save`, including what is guaranteed on failure.
- [`normalize-url.md`](./contracts/normalize-url.md) — the input→output table that pins I2.

The Tauri command surface (`list_sites`, `get_warning`, `add_site`, `update_site`,
`delete_site`, `get_autostart`, `set_autostart`) and the `site-status` / `store-warning`
events are **unchanged in name, arguments, return type, and payload**. That is FR-010 and
FR-011, and it is why this feature ships without touching the frontend.
