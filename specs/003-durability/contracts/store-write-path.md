# Contract: `Store` write path

**Module**: `src-tauri/src/store.rs` | **Feature**: `specs/003-durability`

This is an internal Rust contract, not a wire format. It is written down because `Store` is the
only thing in this app that touches the user's file, and after this feature it makes a
guarantee it did not make before.

**The Tauri command surface is unchanged** — no command signature, argument name, return type,
or event payload moves. Consumers of the store see the same API; what changes is what survives
a crash.

---

## Guarantee

> At every instant, the `sites.json` path holds either its complete previous contents or the
> complete new contents. There is no instant at which it holds a partial or interleaved
> mixture, and no instant at which it does not exist. — FR-001

This holds for a process that dies at any point, including inside `save`. It is scoped to
process death, not to media failure or power loss (spec Assumptions, research R3).

---

## Public API

### `pub fn load(path: PathBuf) -> LoadOutcome`

**Unchanged. Must remain unchanged.** — FR-005

| Input | `store.list()` | `warning` | File on disk |
|---|---|---|---|
| path absent | `[]` | `None` | — |
| valid JSON array | parsed sites, order preserved | `None` | untouched |
| unreadable (I/O, permissions) | `[]` | `Some(…)` | untouched |
| not valid JSON | `[]` | `Some(…)` | **left exactly as found** |

`load` never returns an error and never writes. It opens exactly the path it was handed — it
does not enumerate the directory, so a sibling staging artifact is invisible to it.

Pinned by the existing tests `missing_file_yields_an_empty_list_and_no_warning` and
`corrupt_file_yields_an_empty_list_a_warning_and_is_left_on_disk`, both of which must pass
**unmodified**.

---

### `pub fn add(&mut self, site: Site) -> Result<(), String>`

**Changed**: gains a duplicate-id refusal.

| Precondition | Result | In-memory list | File on disk |
|---|---|---|---|
| no site with `site.id` present | `Ok(())` if the write succeeds | site appended at the end | rewritten atomically |
| **a site with `site.id` already present** | **`Err(_)`** | **unchanged** | **unchanged — no write is attempted** |
| id is new, but the write fails | `Err(_)` | site appended (kept) | previous contents, complete |

The refusal happens **before** the push and therefore before `save`. Ordering is the contract:
FR-009 requires that a refused add leave both the list and the file untouched, and only a
check that precedes the mutation delivers that.

Note the asymmetry in the last two rows, which is existing behaviour and is intentional: a
*write* failure keeps the user's change in memory (FR-004 — the edit is not silently reverted),
while a *refusal* means there was never a valid change to keep.

---

### `pub fn update(&mut self, site: Site) -> Result<(), String>`

**Unchanged in behaviour**, now atomic by virtue of `save`.

Replaces the site with a matching id, preserving list position. A site that is not present
remains a no-op that still saves and still returns `Ok`. (That no-op branch is one of the
roadmap's §4 coverage gaps; closing it is not in this feature's scope.)

---

### `pub fn delete(&mut self, id: &str) -> Result<(), String>`

**Unchanged in behaviour**, now atomic by virtue of `save`. Removes every site with the given
id — in practice at most one, now guaranteed at most one by invariant I1.

---

## Private write path

Both steps are private to `Store`. `store.rs`'s own `mod tests` is a child module and reaches
them directly; nothing outside the module can.

### staging step — `fn …(&self) -> Result<PathBuf, String>`

1. Create `self.path`'s parent directory if absent. — FR-006
2. Serialize `self.sites` with `serde_json::to_string_pretty`. — FR-010, unchanged
3. Write those bytes to `sites.json.tmp` **in `self.path`'s own directory**, truncating any
   orphan already there. — FR-003
4. `sync_all()` the file. — research R3
5. Return the staging path.

**Post-state on success**: `sites.json` is untouched and still holds the previous contents;
`sites.json.tmp` holds the complete new contents. This is the state an interrupted save leaves
behind, and it is the state the atomicity test asserts against.

**On failure at any step**: `Err(_)`, and `sites.json` is untouched. The rename never runs.

### `fn save(&self) -> Result<(), String>`

The staging step, then `std::fs::rename(staged, &self.path)`.

**On success**: `sites.json` holds the new contents; no staging artifact remains (the rename
consumed it).

**On rename failure**: `Err(_)`, `sites.json` holds its complete previous contents, and the
staged file remains as the single permitted orphan.

---

## Failure contract — FR-004

Every `Err` from this path satisfies all three of:

1. The previous `sites.json` is complete and loadable. No caller has to hand-repair anything.
2. The in-memory list still reflects the user's change (for write failures — see the `add`
   note above for how a refusal differs).
3. The error string reaches the UI through the existing `store-warning` banner, unchanged:
   `commands.rs::warn_on_write_failure` is not modified.

Message shape stays `Could not …: {e}`, one line, naming `sites.json`. — research R9

---

## Artifact contract — FR-003 / SC-005

- The staging artifact lives in `sites.json`'s directory. Never `std::env::temp_dir()`; a
  cross-filesystem rename is not atomic and generally fails outright.
- Its name is fixed, so repeated interrupted saves cannot produce more than one. The count
  does not grow with the number of interruptions.
- It is never treated as the site list, by any code path.

---

## What this contract does **not** do

- No cross-process locking. One process owns the file. — spec Assumptions
- No backup, no version history, no recovery from an orphaned staging file. — spec Out of Scope
- No cleanup of orphans at load time. — research R8
- No parent-directory `fsync`; durability is scoped to process death. — research R3
- No change to the on-disk array shape or to any serialized field name. — FR-010,
  Constitution V
