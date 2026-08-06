# Quickstart: Validating Concurrency & Robustness Hardening

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

How to prove this feature works, and — just as important — how to prove it changed nothing
else. Two of the three stories are fully machine-verifiable; the third has one seam that is
checked by eye, named openly in step 5 rather than left to be discovered at review.

## Prerequisites

Run everything from this worktree:

```sh
cd /Users/clint/src/clintcparker/site-checker--20260806-120325-section-1-docs
```

The Rust toolchain is already in place (rustc 1.97.1 — `Mutex::clear_poison` needs ≥ 1.77).
The frontend needs one setup step, because a fresh worktree has no `node_modules`:

```sh
pnpm install
```

## Baseline (confirmed 2026-08-06, before any change)

```sh
cd src-tauri && cargo test          # 42 passed, 0 failed
cd .. && pnpm test                  # 30 passed, 0 failed
cd src-tauri && cargo clippy -- -D warnings   # clean
```

Both counts were re-run in this worktree, not carried over from `main`. Every one of these
tests must still pass **unmodified** at the end (SC-008) — with the single documented exception
in step 3.

---

## 1. Prove each story fails first (FR-017, SC-006)

The spec requires a test per behaviour that fails against today's code. Write the test, watch it
fail, then write the fix. If a new test passes before the fix lands, it is pinning something
other than what it claims.

| Story | Test asserts | Fails today because |
|---|---|---|
| US1 | `lock::recover` on a poisoned mutex returns a guard, reports `true`, and preserves the half-applied change | `lock.rs` does not exist; every site is `lock().unwrap()`, which panics |
| US2 | `Store::add` on a duplicate id yields `AddError::DuplicateId`, not `AddError::Write` | `add` returns `Result<(), String>`; the variants do not exist |
| US3 | two threads editing one site through `Store::replace` leave the second edit applied on top of the first's result | `replace` does not exist; the equivalent path in `commands.rs` reads and writes under separate locks |

---

## 2. Story 1 — fault recovery

```sh
cd src-tauri && cargo test lock::
```

**Expected**: the recovery tests pass. What they cover:

- **Recovers.** A thread panics holding the guard, is joined, and the next `recover` returns a
  usable guard instead of panicking (FR-001).
- **Preserves.** The data is exactly as the panicking thread left it — a half-applied change is
  still there, nothing is reset (FR-006). This is the guarantee that makes "continue, not reset"
  real rather than aspirational.
- **One-shot.** A second `recover` reports `false`. This is the pin for the spec's "the app does
  not accumulate a permanent degraded mode" edge case; without `clear_poison` it fails.

Then the structural half:

```sh
grep -rn 'lock()\.unwrap()' src-tauri/src/    # expect: no matches
```

Zero matches is SC-002. Six of the nine remaining lock sites are safe by construction —
`SharedStore.inner` is private with no accessor, so a store lock cannot be written un-recovered.
The other three (two `tasks`, one `warning`) are call-site discipline and should be pinned by the
source-text guard test proposed in research R7. Each should carry a comment saying *why* it
discards the recovery flag; a bare `.0` with no comment is a review finding.

---

## 3. Story 2 — a refused add leaves no ghost

```sh
cd src-tauri && cargo test store::
```

**Expected**: `add_rejects_a_duplicate_id` passes, now asserting the `DuplicateId` variant
rather than bare `.is_err()`.

**The one documented exception to "unmodified"**: `add_rejects_a_duplicate_id` and
`a_failed_save_leaves_the_previous_file_intact` are tightened from `.is_err()` to a variant
match. They would still compile untouched — which is the problem. A test that cannot tell the
two failures apart is not pinning the thing this story exists to fix.

**Manual check** (the shell branch has no unit test — `add_site` needs a Tauri `State`):

1. `pnpm tauri dev`, add a site, confirm the row appears and starts checking. Nothing about
   the happy path may change (FR-007).
2. Reading `commands.rs`, confirm by eye that the `DuplicateId` arm returns `Err` **above**
   `engine.start(...)` — that ordering is FR-009, and it is invisible to any test here.
3. Confirm the refusal message does not contain the word "saved" (FR-010).

---

## 4. Story 3 — two edits cannot discard each other

```sh
cd src-tauri && cargo test store::replace
```

**Expected**: all `replace` tests pass, including the two-thread contention test over one
`Arc<Mutex<Store>>`. The FR-014 rules each get their own single-threaded test — unchanged URL
keeps `method_override`, changed URL drops it, unknown id returns `None` and writes nothing.

The contention test is the story. It should fail against a deliberately re-split
read-then-write implementation, so if it passes both ways it is testing the wrong thing.

---

## 5. The seam that is checked by eye, not by machine

`SharedStore::lock`'s warning half needs an `AppHandle` to emit through, and `tauri`'s mock-app
test harness was declined (research R7) rather than added for three lines of `emit`. So:

```sh
pnpm tauri dev
```

- Confirm the app launches, the site list loads, and checks start — the `SharedStore` rewrite
  touched every store access, so this is the smoke test for all of them.
- Add, edit, and delete a site. All three must behave exactly as before (FR-007).
- Read `SharedStore::lock` and confirm the recovery flag is wired to a `store-warning` emit, and
  that the message says the saved list may not reflect the most recent change (FR-004).
- Read `engine.rs`'s two `tasks` locks and `get_warning`'s `warning` lock and confirm none of
  them emits (FR-005, FR-003).

Provoking a real poison in the running app would mean shipping a panic, which this feature is
not going to do. The mechanism is proved in step 2; the wiring is proved here.

---

## 6. Full merge bar (SC-007)

```sh
cd src-tauri && cargo test                    # 42 + the new tests, 0 failed
cd .. && pnpm test                            # 30 passed, unmodified (SC-008)
cd src-tauri && cargo clippy -- -D warnings   # clean
```

Nothing disabled, nothing skipped, no `#[ignore]`. **The frontend count must be exactly 30 and
no file under `src/` may appear in the diff** — any frontend change is a scope violation under
FR-016 (see [contracts/command-surface.md](./contracts/command-surface.md)).

---

## 7. Close-out: drain the roadmap in the *primary* checkout

```sh
cd /Users/clint/src/clintcparker/site-checker    # NOT this worktree
```

Edit `docs/ROADMAP.md` §1 there. `docs/` is gitignored, so an edit made in the worktree never
reaches `main` — that is exactly how `003-durability` lost its roadmap edit, as its own entry
records (research R10).

Retire three of the four items. **Keep the fourth** — the symlink-at-the-path note — with
FR-018's reason attached: it is expected behaviour inherent to the atomic-save guarantee, and
undoing it would reopen the truncation window `003-durability` exists to close.
