# Implementation Plan: Concurrency & Robustness Hardening

**Branch**: `20260806-120325-section-1-docs` | **Date**: 2026-08-06 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/20260806-120353-concurrency-hardening/spec.md`

## Summary

`docs/ROADMAP.md` §1 lists four items; three are latent correctness holes in the layer
beneath the UI and are closed here. None is reachable from the shipped window today — the
point is that the core stops relying on the window to avoid states it permits.

**Fault recovery (US1).** Ten `Mutex::lock().unwrap()` calls mean one panic inside any
critical section poisons that lock and cascades a panic into every later command — the
app looks alive but accepts nothing until relaunch. The fix is not ten call-site edits: it
is a `SharedStore` wrapper over `Arc<Mutex<Store>>` with no accessor for the raw mutex, so
a store lock *cannot* be taken un-recovered. Recovery is `PoisonError::into_inner()` (which
preserves whatever the interrupted operation left behind, FR-006) plus `Mutex::clear_poison()`
(which makes the poison one-shot, so the user gets one banner per fault rather than one per
subsequent action). The two task-registry locks and the startup-warning lock use the same
`lock::recover` helper directly and discard its flag — recovering them is required, warning
about them is not.

**Refused add (US2).** `Store::add` gained a duplicate-id refusal in `003-durability`, but
`add_site` funnels every `Store::add` error into `warn_on_write_failure` and returns `Ok`
regardless, so a refusal surfaces as "could not be saved" while a row appears and a timer
starts for a site in no list. A two-variant `AddError` lets the shell tell the two apart and
branch: refusal returns `Err` (no row, no timer, no banner), write failure keeps today's
behaviour exactly.

**Atomic edit (US3).** `update_site` takes the store lock twice — once to read
`method_override`, once to write — so two overlapping edits can each decide from the same
stale snapshot. The read-decide-write moves into `Store::replace`, where a single `&mut self`
borrow makes the interleaving impossible by construction rather than by call-site discipline.
That collapse is why the lock-site count goes 10 → 9 (research R9).

All three changes are Rust. **Zero frontend files change** (research R8): both permitted
user-visible messages already have a surface. No dependency is added. Nothing about
`sites.json` moves.

## Technical Context

**Language/Version**: Rust 1.97.1, edition 2021 (backend — the only layer this feature
touches). Vanilla TypeScript, no framework (frontend — unmodified).

**Primary Dependencies**: tauri 2.11, serde / serde_json 1, url 2.5, uuid 1.24, reqwest 0.13,
tokio 1, rand 0.10, tauri-plugin-autostart 2.5. **This feature adds none, runtime or dev** —
poison recovery is `std::sync` only, and `tauri`'s `test` feature was considered and declined
(research R7).

**Storage**: one file — `~/Library/Application Support/com.clintparker.site-checker/sites.json`,
a bare pretty-printed JSON array of `Site` with snake_case keys. Untouched: no shape, field,
location, or write-path change (FR-015). `003-durability`'s stage-then-rename save is inherited
as-is.

**Testing**: `cargo test` (unit; `tempfile` for temp dirs, `httpmock` for the HTTP classifier),
`pnpm test` (vitest), `cargo clippy -- -D warnings`. **Baseline re-confirmed in this worktree
on 2026-08-06: Rust 42 passed / 0 failed; frontend 30 passed / 0 failed** (after a `pnpm install`
— the worktree had no `node_modules`).

**Target Platform**: macOS desktop, Tauri 2 app bundle.

**Project Type**: desktop app — pure Rust core, thin Tauri command shell, vanilla TS frontend.

**Performance Goals**: none. Lock hold times are unchanged (the store guard is still taken and
released synchronously, never across an `.await`); `clear_poison` runs only on the recovery
path, which in a healthy session never runs at all.

**Constraints**: no new dependency; no change to the on-disk shape or location (FR-015); no new
user-facing capability, and only two permitted user-visible changes — the recovery warning
(FR-004) and the refusal message (FR-010) (FR-016); `002-robustness`'s submit/delete guards and
interval ceiling and `003-durability`'s atomic save must all survive unweakened; the
symlink-replacement behaviour must **not** be changed (FR-018); every one of the three
behaviours needs a test that fails against today's code (FR-017).

**Scale/Scope**: single user, single process, tens of sites. Four Rust files change
(`lib.rs`, `commands.rs`, `engine.rs`, `store.rs`), one is added (`lock.rs`), plus
`docs/ROADMAP.md` — **edited in the primary checkout, not this worktree**, because `docs/` is
gitignored and `003-durability` lost its roadmap edit exactly that way (research R10). Zero
frontend files.

**Unknowns**: none. The spec left no `[NEEDS CLARIFICATION]` markers. The one place behaviour
was genuinely uncertain — what `into_inner()` and `clear_poison()` actually do to a
half-applied change, and whether a cleared poison stays cleared — was settled by a probe run
against this toolchain rather than from recall (research R1).

## Constitution Check

*GATE: evaluated before Phase 0, re-evaluated after Phase 1. Both passes recorded.*

| Principle | Verdict | Basis |
|---|---|---|
| **I. One Mac, One Person** | **PASS** | FR-016 forbids new capability and the design adds none. No alerting, history, sync, or auth appears anywhere. The two permitted messages both reuse surfaces the app already has. |
| **II. Results Are Ephemeral, Config Is Sacred** | **PASS** — and the FR-004/FR-005 split *is* this principle | The site list is the one thing the app owns, so a fault that may have left it half-written earns the existing banner ("a corrupt file is an empty list **plus a visible warning**"). The check registry is ephemeral by design and rebuilt every launch, so its recovery is silent. `sites.json` itself is untouched. |
| **III. Be a Polite Client** | **PASS** | No request behaviour changes at all. `check.rs` is not modified; the HEAD→GET discovery and its persistence are carried through `Store::replace` verbatim (FR-014), so no site is re-probed that would not be re-probed today. |
| **IV. Testable Core, Thin Shell** | **PASS**, and the design is chosen *for* this | Every behaviour that FR-017 must pin lands in the `cargo test`-drivable layer: `Store::replace` and `AddError` in `store.rs`, `recover` in the new `lock.rs` as a Tauri-free generic function. The alternative for US3 — one guard held in `commands.rs` — was rejected precisely because it would have been untestable (research R5). What stays untested is named openly: three lines of `emit` wiring, per research R7. |
| **V. The Rust/TS Contract Is snake_case, As-Is** | **PASS** | No serialized field name changes and no `rename_all` is added. `Site` and `StatusEvent` are untouched. `AddError`, `Replaced`, and `SharedStore` are internal Rust types that never cross the IPC boundary — the commands map them to the existing `Result<Site, String>` / `Result<(), String>` shapes, so the frontend contract is byte-identical (see [contracts/command-surface.md](./contracts/command-surface.md)). |
| **Quality Gates** | **PASS by construction** | The story split below is sequenced so each story leaves all three gates green independently. SC-007 requires the bar at the end; this plan requires it after each story. |

**Post-Phase-1 re-evaluation**: unchanged, all PASS. Phase 1 introduced one new module
(`lock.rs`), one new shell type (`SharedStore`), and two new internal error/result types
(`AddError`, `Replaced`) — documented in [data-model.md](./data-model.md) and the three
contracts. None is serialized, none crosses the Tauri boundary, and none adds logic to the
shell: `SharedStore` holds a lock and an `AppHandle` and nothing else. Principle IV is
*strengthened* rather than strained, because `Store::replace` moves the `method_override`
decision rule out of the untested `commands.rs` and into the temp-dir-tested `store.rs`.

## Project Structure

### Documentation (this feature)

```text
specs/20260806-120353-concurrency-hardening/
├── spec.md                       # Input
├── plan.md                       # This file
├── research.md                   # Phase 0 — ten decisions, incl. the probe results
├── data-model.md                 # Phase 1 — entities and the poison state machine
├── quickstart.md                 # Phase 1 — how to prove it, incl. the manual banner check
├── contracts/
│   ├── lock-recovery.md          #   lock::recover + SharedStore::lock
│   ├── store-mutation-api.md     #   Store::add / replace / update / delete
│   └── command-surface.md        #   the Tauri commands the frontend sees
├── checklists/
│   └── requirements.md           # Written by /speckit-specify
└── tasks.md                      # Phase 2 — NOT created by /speckit-plan
```

### Source Code (repository root)

```text
src-tauri/src/
├── lock.rs        # NEW — lock::recover<T> (pure, tested) + SharedStore (thin, Tauri-aware)
├── lib.rs         # CHANGED — builds a SharedStore in setup(); no raw Arc<Mutex<Store>>
├── commands.rs    # CHANGED — AppState.store: SharedStore; add_site branches on AddError;
│                  #           update_site delegates to Store::replace
├── engine.rs      # CHANGED — Inner.store: SharedStore; tasks locks use lock::recover
├── store.rs       # CHANGED — Store::add -> Result<(), AddError>; new Store::replace
├── model.rs       # unchanged
├── check.rs       # unchanged
└── main.rs        # unchanged

src/               # unchanged in full — api.ts, main.ts, form.ts, render.ts, time.ts,
                   # and all four test files

docs/ROADMAP.md    # CHANGED — §1 drained (3 of 4 items), EDITED IN THE PRIMARY CHECKOUT
```

**Structure Decision**: the existing layout is kept; the only addition is `src-tauri/src/lock.rs`.
It exists as its own module rather than living in `store.rs` or `commands.rs` because it has to
sit on *both* sides of the project's central split: `recover<T>` is a generic, Tauri-free
function that `cargo test` drives directly, while `SharedStore` needs an `AppHandle` to raise the
FR-004 banner. Putting either half in `store.rs` would drag Tauri into the pure temp-dir-tested
layer; putting them in `commands.rs` would make `engine.rs` depend on `commands.rs`, which
already depends on `engine.rs`. A separate module gives a clean `commands → lock` and
`engine → lock` graph with no cycle.

## Implementation Sequence

Ordered so each story is independently shippable with all three gates green, and so the
riskiest structural change lands first with the smaller ones on top of it.

**Story 1 — fault recovery (P1, FR-001…FR-007).** Add `lock.rs` with `recover<T>` and its unit
tests (poisoned → recovered, half-applied state preserved, second recovery reports clean). Add
`SharedStore` and move the `store-warning` emit into it so all banner traffic funnels through
one place. Rewrite `lib.rs`'s `setup()` to build the `SharedStore`, thread it into `Engine` and
`AppState`, and switch the remaining task-registry and startup-warning locks to
`lock::recover(..).0`. At the end of this story no `.lock().unwrap()` remains in the crate;
add the source-text guard test that pins that (research R7).

**Story 2 — refused add (P2, FR-008…FR-012).** Introduce `AddError` in `store.rs`, adapt the
existing `add_rejects_a_duplicate_id` test to the new type and add one asserting the variant is
`DuplicateId` (not `Write`). Branch `add_site` on it: refusal returns `Err` before
`engine.start`; write failure keeps today's path exactly.

**Story 3 — atomic edit (P3, FR-013, FR-014).** Add `Store::replace` and `Replaced`, moving the
`method_override` rule out of `commands.rs` verbatim. Pin it with a two-thread contention test
over one `Arc<Mutex<Store>>` plus single-threaded tests for each FR-014 rule. Rewrite
`update_site` as a caller. Beware the guard's temporary lifetime here: bind the `SharedStore`
guard to a named variable in its own scope rather than relying on a `let ... else` temporary,
so the lock is provably released before `reschedule`.

**Close-out.** Drain §1 of `docs/ROADMAP.md` **in the primary checkout** — three items retired,
the fourth kept with FR-018's reason attached. Re-run both suites and clippy.

## Complexity Tracking

> Constitution Check passed on both evaluations with no violations. No justification required;
> the table is retained empty per the template.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| — | — | — |
