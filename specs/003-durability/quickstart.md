# Quickstart: Validating Durability & Data Integrity

**Feature**: `specs/003-durability` | **Date**: 2026-08-06

How to prove this feature works. Everything of consequence is provable from `cargo test` —
that is the point of Constitution IV, and all three changes land in the pure layer. The manual
pass at the end exists only to confirm the one thing a unit test cannot see: that the UI is
genuinely unchanged.

---

## Prerequisites

Run from the feature worktree:

```bash
cd ~/src/site-checker--20260806-102818-durability-and-data
```

Toolchain already present and confirmed on this machine: `rustc 1.97.1`, `pnpm`, `tempfile`
in `[dev-dependencies]`. **No new dependency is added by this feature** — if `Cargo.toml`
gained one, something went wrong; see research R1.

---

## Baseline (captured 2026-08-06, before any change)

```bash
cd src-tauri && cargo test          # 29 passed; 0 failed
cd .. && pnpm test                  # 30 passed
cd src-tauri && cargo clippy -- -D warnings   # clean
```

The Rust baseline was re-run in this worktree and confirmed: **29 passed, 0 failed**. Any
number below 29 after this feature means an existing test was removed or broken — SC-007
allows exactly one kind of exception, and only one test qualifies for it (see US2 below).

---

## Validating User Story 1 — the list survives an interrupted save

**All of this is `cargo test`.** The seam that makes it testable is the split between the
staging step and the rename (research R4); the tests call the staging step directly to stop a
save at exactly the moment the guarantee is about.

```bash
cd src-tauri && cargo test store::
```

| Prove | Spec scenario | Assertion |
|---|---|---|
| Interrupted before publication | 1 | Stage a save on a store holding two sites, then `load()` the live path: **two** sites, `warning.is_none()` |
| The staged file is real but invisible | 1, FR-003 | `sites.json.tmp` exists next to `sites.json` and holds the three-site list; `load()` still returned two |
| Completed save publishes cleanly | 2 | After a full `save()`, `load()` returns three sites and no `.tmp` remains |
| No duplicated or partial content | 2 | The file parses as a bare array of exactly the expected sites |
| Orphans do not accumulate | Edge case, SC-005 | Stage twice (or three times) without renaming, then count files in the directory: `sites.json` + exactly one `.tmp` |
| Failed save preserves the previous file | 3, FR-004 | Point the store at a path whose parent is read-only (or is a directory — see below), assert `save()` is `Err`, then assert the pre-existing `sites.json` still loads with its original contents |
| Corrupt-file path unchanged | 4, FR-005 | `corrupt_file_yields_an_empty_list_a_warning_and_is_left_on_disk` passes **unmodified** |
| Parent directory still created | 5, FR-006 | `writes_create_the_parent_directory` passes **unmodified** |

Two verified facts to lean on when writing the failure test (research R5, confirmed by
experiment on this machine):

- A **directory** at the `sites.json` path makes the rename fail with `EISDIR` — a clean,
  deterministic failure, and the easiest way to write the "save fails" test without needing
  a full volume.
- A **symlink** at that path is *replaced* by the rename rather than followed. The target keeps
  its bytes. This is a real behavioural difference from `fs::write` and the spec's edge-case
  wording needs amending to say so — do not write a test asserting the old behaviour.

**Independent-test bar (spec US1)**: after this story alone, `cargo test`, `pnpm test`, and
`cargo clippy -- -D warnings` are all green and nothing from US2 or US3 has been touched.

---

## Validating User Story 2 — a typed scheme is stored in a consistent case

Pure function, no filesystem, no app launch.

```bash
cd src-tauri && cargo test model::
```

Assert the full table in [`contracts/normalize-url.md`](./contracts/normalize-url.md). The
rows that matter most:

| Input | Expected | Spec scenario |
|---|---|---|
| `HTTPS://example.com` | `https://example.com` | 1 |
| `HtTp://example.com/health` | `http://example.com/health` | 2 |
| `example.com` | `https://example.com` — **no trailing slash** | 3 |
| `example.com?next=HTTP://x.dev` | `https://example.com?next=HTTP://x.dev` — query verbatim | 4 |
| `FTP://example.com` | `Err` | 5 |
| `https://EXAMPLE.com` | `https://EXAMPLE.com` — host case kept | Assumptions |

Scenario 3 is the regression guard: if the trailing slash comes back, the fix took the
forbidden shortcut of returning `url::Url`'s serialization instead of the user's text.

**The one permitted test change under SC-007**: none of the existing `model::` tests encode
the old uppercase-passthrough behaviour, so none should need editing. If one does need
touching, that is a signal to re-read the change, not a licence to edit the assertion.

Scenario 6 — a stored uppercase URL is loaded as-is and normalized only on edit — is covered by
absence: `load()` is not modified and does not call `normalize_url`. Confirm by inspection,
not by a test.

---

## Validating User Story 3 — the store refuses two sites under one id

```bash
cd src-tauri && cargo test store::
```

| Prove | Spec scenario | Assertion |
|---|---|---|
| Duplicate is refused | 1 | Add `abc`, add a *different* site also with id `abc` → `Err`; list length is 1; the surviving site's fields are the **original** ones |
| The refusal wrote nothing | 2 | `load()` from the same path afterwards returns the pre-add state |
| A distinct id still works | 3 | Add `abc`, add `xyz` → both `Ok`, list length 2, order preserved |

Scenario 2 is the one that pins the ordering requirement: the check must run *before* the push
and before `save`, so a store whose file was never written proves it.

---

## Full gate — run after **each** story, not only at the end (SC-007)

```bash
cd src-tauri && cargo test && cargo clippy -- -D warnings
cd .. && pnpm test
```

Expected after all three stories: Rust **above 29** (new tests added, none removed), frontend
**30 unchanged** — this feature touches no frontend file, so a change in that number means
something out of scope was edited.

---

## Manual confirmation (once, at the end)

The only thing worth doing by hand, because it is the only claim the test suite cannot make:
**FR-011, that nothing about the app changed.**

```bash
pnpm tauri dev
```

1. Add a site with a normal URL. It appears, goes Pending, then resolves. Quit and relaunch —
   it is still there.
2. Add a site typed as `HTTPS://example.com`. It is listed as `https://example.com`.
3. Edit a site's interval, then delete a site. Both persist across a relaunch.
4. Check the app support directory: `sites.json` present, **no `sites.json.tmp` left behind**
   after a clean run.

```bash
ls -la ~/Library/Application\ Support/com.clintparker.site-checker/
```

> **Before running the app at all**, take an out-of-tree copy of your real `sites.json`. A
> stale dev server from an earlier run can hold the file, and losing the list to a durability
> feature would be a poor result.

---

## Definition of done for this plan's scope

- All three gates green after each story, not just at the end. — SC-007
- Rust test count strictly above 29; frontend still 30.
- No new entry in `Cargo.toml`.
- `docs/ROADMAP.md` section 1 emptied — all three items marked done. — SC-008
- Two spec/roadmap amendments carried out rather than dropped: the symlink edge case corrected
  (research R5), and the `add_site` refusal-vs-write-failure inconsistency recorded under
  roadmap section 2 (research R7).
