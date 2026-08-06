# Contract: `Store` Mutation API (`src-tauri/src/store.rs`)

**Feature**: [../spec.md](../spec.md) | **Plan**: [../plan.md](../plan.md) |
**Covers**: FR-008, FR-012, FR-013, FR-014

The pure, temp-dir-tested layer. Two of the four mutations change signature; the on-disk format
and the `stage`/`save` write path inherited from `003-durability` do not change at all.

---

## `add` — the refusal becomes tellable

```rust
pub enum AddError {
    /// The list already holds this id. Nothing was applied — not in memory, not on disk.
    DuplicateId(String),
    /// The site is in the in-memory list; the save failed.
    Write(String),
}

pub fn add(&mut self, site: Site) -> Result<(), AddError>
```

**Was**: `Result<(), String>`, with both failures indistinguishable.

| # | Guarantee | Requirement |
|---|---|---|
| A1 | An id already in the list yields `DuplicateId`, decided **before** any mutation, so the in-memory list and the file still agree. | FR-012 |
| A2 | A save failure yields `Write` with the message the banner shows; the site **is** in the in-memory list. | FR-011 |
| A3 | Success appends and persists, exactly as today. | FR-007 |
| A4 | Ordering is unchanged: refusal is checked, then push, then save. | — |

The variants' doc comments are the contract. `003-durability` already wrote this distinction
down as a long prose comment on `add`, because there was no type to say it with; the comment
shrinks to the two doc lines above, and the prose about the caller's obligation moves to
[command-surface.md](./command-surface.md), which is where the obligation actually lives.

**Existing test impact**: `add_rejects_a_duplicate_id` and `a_failed_save_leaves_the_previous_file_intact`
assert `.is_err()`, which still compiles. Both should be tightened to match the variant, or the
whole point of the type is lost at exactly the place it is being proved.

---

## `replace` — read, decide, and write as one step

```rust
pub struct Replaced {
    pub site: Site,
    pub write: Result<(), String>,
}

pub fn replace(
    &mut self,
    id: &str,
    url: String,
    label: Option<String>,
    interval_secs: u64,
) -> Option<Replaced>
```

**New.** `update_site` currently does this across two separate lock acquisitions, so two
overlapping edits can each decide from the same stale snapshot and the later write discards the
earlier one's result.

| # | Guarantee | Requirement |
|---|---|---|
| R1 | The read of the current entry, the `method_override` decision, the in-place write, and the save all happen under one `&mut self` borrow. Nothing can be applied in between — not by discipline, but because there is no moment to interleave into. | FR-013 |
| R2 | Unchanged URL → the existing `method_override` is carried forward. | FR-014 |
| R3 | Changed URL → `method_override` becomes `None`, so HEAD support is re-learned against the new address. | FR-014, Constitution III |
| R4 | No entry with that id → `None`, and **nothing is written** (no save, no `sites.json` touch). | FR-014 |
| R5 | `Replaced.site` is the entry as it now stands; `Replaced.write` is the save result, so a failed save keeps today's behaviour (change stands in memory, banner fires). | FR-011 analogue |
| R6 | List order is preserved, as `update` already guarantees. | FR-015 |

**Inputs are pre-shaped by the caller.** `normalize_url`, `clamp_interval`, and `empty_to_none`
stay in `commands.rs` — they are input shaping, not list invariants. `replace` owns exactly one
rule: what happens to the learned request method. That rule moves here **verbatim** from
`commands.rs`; FR-014 requires the behaviour be identical, and moving it is the only change.

**Why `Option<Replaced>` and not a `Result`.** "There was nothing to edit" and "the edit
happened but did not persist" demand opposite responses from the caller — the first must report
that the site is gone and change nothing, the second must keep the row and warn. Folding both
into one `Result` is the same conflation `AddError` exists to undo one function above.

---

## `update` — unchanged, and kept

```rust
pub fn update(&mut self, site: Site) -> Result<(), String>   // unchanged
```

Still a blind whole-entry write with a save, still a no-op-then-save for an absent id. It is
**not** absorbed into `replace`, because `engine::Inner::persist_get_fallback` is a legitimate
blind write: it records that a site needs GET, and giving it the edit rules would mean it had
opinions about URLs and labels that it has no business having.

---

## `delete` — unchanged

```rust
pub fn delete(&mut self, id: &str) -> Result<(), String>   // unchanged
```

No refusal branch exists, so no typed error is added. `Err` means the same thing it always did:
the removal stands in memory, the save failed.

---

## Everything below the API

`stage`, `save`, `staging_path`, `load`, `list`, and `get` are **untouched**. In particular:

- The stage-to-sibling-then-`rename` publication from `003-durability` is inherited exactly.
  Both new call paths (`add`'s success branch, `replace`) go through the same private `save`,
  so the atomicity guarantee extends to them for free and no new write path exists to audit.
- `load`'s "never fails, corrupt file left on disk" semantics are unchanged (Constitution II).
- The symlink-at-the-path behaviour is **not** touched (FR-018). Undoing it would reopen the
  truncation window `003-durability` exists to close; it stays recorded in the roadmap as
  expected behaviour rather than being fixed.
