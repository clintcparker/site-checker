# Research: Concurrency & Robustness Hardening

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Date**: 2026-08-06

The spec left no `[NEEDS CLARIFICATION]` markers, so nothing here resolves an open
question from it. What is recorded instead are the ten decisions this plan makes that a
reader would otherwise have to reverse-engineer from the diff — including the two where
the obvious choice is the wrong one, and the one place the spec's own validation record
invited a challenge at plan time.

Facts about `std::sync::Mutex` below are from a probe run against this toolchain
(rustc 1.97.1), not from recall — see R1.

---

## R1. Recovering a poisoned lock: `into_inner()` **and** `clear_poison()`

**Decision**: recover with `PoisonError::into_inner()`, and call `Mutex::clear_poison()`
while still holding the recovered guard.

**Rationale**: `into_inner()` alone satisfies FR-001/FR-002/FR-003 — the app keeps
working — but leaves the mutex poisoned *forever*. Every subsequent lock would report
the same long-past fault, so under FR-004 the user would get a fresh "your list may be
stale" banner on every add, edit, delete, and background GET-fallback write for the rest
of the session. That is precisely the failure the spec's edge case rules out: "the app
does not accumulate a permanent degraded mode, and repeated warnings do not stack".
`clear_poison()` (stable since Rust 1.77; we are on 1.97.1) makes the poison a one-shot
signal — exactly one warning per fault, which is what FR-004 asks for.

Clearing *while holding the guard* rather than after dropping it is deliberate: both
`clear_poison(&self)` and the guard are shared borrows so it compiles either way, but
doing it under the guard means no other thread can observe the window between "poison
cleared" and "we are done", so two threads cannot both report the same fault.

**Verified, not assumed**. A standalone probe (`rustc --edition 2021`) confirmed all
four properties this design leans on:

| Property | Result |
|---|---|
| A panic while holding the guard poisons the mutex | confirmed (`lock()` returns `Err`) |
| `into_inner()` yields the data *as the panicking thread left it* | confirmed — a half-applied `push` was still there (this is FR-006) |
| `clear_poison()` compiles and runs while the recovered guard is alive | confirmed |
| A second recovery after a clear reports "not poisoned" | confirmed — no permanent degraded mode |

**Alternatives considered**:

- **`parking_lot::Mutex`** — no poisoning at all, so FR-001/2/3 come free. Rejected on
  two counts: it is a new runtime dependency for a single-user desktop tool, and more
  importantly it *destroys the signal*. FR-004 requires telling the user their saved list
  may be stale, and a lock that never poisons cannot tell you a fault happened. It solves
  the smaller half of the story by discarding the larger half.
- **`into_inner()` without `clear_poison()`** — see above; produces a permanent banner
  storm, contradicting the spec's own edge case.
- **`RwLock`** — orthogonal. It poisons too, and nothing here is read-heavy enough to
  care. Changing lock flavour would be scope the spec forbids (FR-016).
- **Catching the panic instead (`catch_unwind`)** — treats the symptom at the wrong
  layer, would have to wrap every critical section, and does nothing for a panic that
  originates outside our code.

---

## R2. Where recovery lives: a `SharedStore` type, not discipline at ten call sites

**Decision**: introduce `SharedStore` — a thin, cloneable wrapper over
`Arc<Mutex<Store>>` plus the `AppHandle` — whose single `lock()` method recovers and
warns. `AppState` and `Engine` hold a `SharedStore` instead of an `Arc<Mutex<Store>>`,
and neither can reach the raw mutex.

**Rationale**: the roadmap frames this item as "replace ten `.unwrap()`s", which invites
the mechanical fix: a helper function called correctly at each site. That leaves the
invariant as a property of caller discipline — the eleventh site added next year is a
fresh chance to write `.lock().unwrap()`, and nothing catches it. Wrapping the mutex
removes the option: with no accessor for the inner `Mutex`, a store lock *cannot* be
taken un-recovered, and it cannot forget to warn.

This is the same move `003-durability` made one layer down, and for the stated reason —
`Store::add` owns the duplicate-id refusal "so the invariant lives at the layer that owns
it rather than being a property of one caller's id generator". Six of the ten sites are
store locks, and all six become safe by construction.

**Alternatives considered**:

- **A free helper + an extension trait method, called at each site.** Simpler diff, no
  new type. Rejected: it leaves the invariant re-breakable, and it needs *two* method
  names (warning and silent) which is an invitation to pick the wrong one at the wrong
  site — the exact mistake FR-005 exists to prevent.
- **Putting the wrapper in `store.rs`.** Rejected: `store.rs` is the pure, temp-dir-tested
  layer and has no Tauri dependency today. Dragging `AppHandle` into it to satisfy FR-004
  would break Constitution IV's central split for the sake of file count.
- **Recovering inside `Store` itself** (i.e. `Store` owns its own lock). Rejected: `Store`
  is deliberately a plain `&mut self` type; its testability comes from not being shared.

---

## R3. Which recoveries warn, and which are silent

**Decision**: a recovered **store** lock warns (FR-004). A recovered **task registry**
lock and a recovered **startup-warning** lock are silent (FR-005, FR-003).

**Rationale**: this is Constitution II applied directly. The site list is the one thing
the app owns on disk, so a fault that may have left it half-written is exactly the class
of non-fatal problem the existing banner was built for — the constitution's own words are
"a corrupt file is an empty list *plus a visible warning*". The task registry is the
opposite: which checks are currently running is ephemeral by design, rebuilt at every
launch, and every scheduling call replaces rather than accumulates, so a recovered
registry has no user-visible consequence to report. The startup-warning slot holds one
`Option<String>` that is `take()`n once; warning the user about a fault in the warning
channel would be noise about nothing.

**On the spec's open invitation**: the spec's validation record flagged "silent vs.
visible recovery" as the one judgment call with materially different outcomes, resolved
by assumption rather than by asking, and invited a challenge at plan time. Challenged and
**upheld**. Silence would be cheaper and would let the whole feature be backend-invisible,
but the user's file may genuinely not match what they last asked for, and this app's
established answer to "something non-fatal went wrong with your file" is a one-line
banner. Nothing about that reasoning changes at plan time.

**Alternative considered**: warn on *every* recovery, uniformly, and drop the FR-005
carve-out. Rejected — it would warn the user about a purely in-memory scheduling
structure they cannot see, do not own, and that is rebuilt on the next launch. A message
that names no consequence trains the user to ignore the banner that does.

---

## R4. A refused add needs a typed error, not a sentinel string

**Decision**: `Store::add` returns `Result<(), AddError>` where

```text
AddError::DuplicateId(String)  — nothing was applied, in memory or on disk
AddError::Write(String)        — the change is in memory; the save failed
```

`Store::update` and `Store::delete` keep `Result<(), String>` unchanged.

**Rationale**: FR-008 requires the shell to *distinguish* the two, and today both arrive
as `Err(String)`. The two variants carry opposite promises to the caller, and the doc
comment on each is the contract that `warn_on_write_failure` currently only implies —
`003-durability` already wrote that mismatch down in a long comment on `Store::add`
because there was no type to say it with. Now there is.

Leaving `update`/`delete` alone is deliberate rather than lazy: neither has a refusal
branch, so a shared `StoreError` enum would give both an unreachable variant and force
every caller to handle a case that cannot happen. The enum is narrow because the problem
is narrow.

**Alternatives considered**:

- **A shared `StoreError` for all three mutations.** Symmetric, and wrong for the reason
  above.
- **Pre-checking with a new `Store::contains(&id)` before calling `add`.** Rejected
  outright: that is the read-then-act shape User Story 3 exists to eliminate, reintroduced
  in the same feature that removes it.
- **Matching on the error string.** Rejected; it is not a contract, it is a coincidence.

---

## R5. Making the edit atomic: move read-decide-write into `Store`, not one guard in the shell

**Decision**: add `Store::replace(&mut self, id, url, label, interval_secs) ->
Option<Replaced>`, which reads the current entry, decides `method_override` from it,
writes the new entry, and saves — all under one `&mut self` borrow. `update_site` becomes
a caller of it. `Store::update` stays for `engine.rs`'s GET-fallback write.

**Rationale**: the cheap fix is to hold a single guard across the existing two locks in
`commands.rs` — three lines, no new API, and it satisfies FR-013 literally. It was
rejected for one concrete reason: **FR-017 and SC-006 require a test that fails against
today's code and passes after the change**, for each of the three behaviours. A fix living
in `commands.rs` needs a Tauri `State` and `AppHandle` to drive, so it cannot be reached
by `cargo test` — the pin would have to be dropped to a manual check, which FR-017 does
not permit. Moved into `Store`, the same behaviour is drivable by two threads sharing one
`Arc<Mutex<Store>>` in a plain unit test.

The design gain comes along for free and is the same one as R2: with `replace` taking
`&mut self`, Rust's borrow checker enforces the atomicity. There is no interleaving for a
future caller to reintroduce, because there is no longer a moment between the read and the
write for anything to interleave *into*.

FR-014 pins the decision rules themselves as unchanged — unchanged URL keeps the learned
method, changed URL drops it, unknown id reports "no longer exists". `replace` moves that
rule verbatim; it does not restate it.

**Shape note**: `replace` returns `Option<Replaced>` — `None` means no such site, which
the command maps to today's `"That site no longer exists"`. `Replaced` carries both the
resulting `Site` and the save result, because a failed *save* must keep today's behaviour
(the in-memory change stands, the banner fires, the row stays). Squashing both into one
`Result` would conflate "there was nothing to edit" with "the edit happened but did not
persist", which is the same conflation R4 is fixing one function over.

**Alternatives considered**:

- **One guard held across `get` + `update` in `commands.rs`.** See above — correct, but
  untestable, and leaves the rule in the shell.
- **`Store::modify(&mut self, id, f: impl FnOnce(&Site) -> Site)`.** Atomic and generic,
  but keeps the `method_override` rule in the untested shell, so it buys atomicity without
  buying the pin FR-017 requires.
- **Making `update` itself atomic by having it re-read.** Rejected: `engine.rs`'s
  GET-fallback path is a legitimate blind write and must not acquire the edit rules.

---

## R6. Provoking a poisoned lock from a test

**Decision**: spawn a thread that panics while holding the guard, `join()` it (discarding
the `Err`), then assert on recovery. Suppress the panic output with a scoped
`std::panic::set_hook` / `take_hook` pair.

**Rationale**: this is the whole mechanism, and it is deterministic — no sleeps, no
timing, no subprocess. The spec's final assumption ("fault injection is achievable in
tests", with a fallback if not) is discharged: it *is* achievable, and the probe in R1
runs exactly this shape successfully.

The panic hook is swapped only for the duration of the spawn+join. `cargo test` runs test
functions in parallel threads and the hook is global, so a concurrent test that panics
during that window would have its message swallowed — a real but small cost, paid only in
a failing run, and the alternative is `boom` in the output of every green run. If it turns
out `cargo test`'s per-thread output capture already suppresses it, drop the hook dance.

**Alternatives considered**:

- **Reaching for a poisoned lock without a real panic** — there is no supported way to
  poison a `Mutex` other than unwinding out of a critical section.
- **Killing a subprocess.** Non-deterministic, slow, and pointless when the in-process
  version is exact.

---

## R7. What is deliberately *not* covered by an automated test

**Decision**: the recovery **mechanism** is unit-tested; the recovery **banner** is
verified by hand, per [quickstart.md](./quickstart.md).

**Rationale**: `SharedStore::lock`'s warning half needs an `AppHandle` to emit through.
Tauri ships `tauri::test::mock_app()` behind its `test` feature, so this is *possible* —
it was rejected because it would add a dev-only feature flag and a mock-app harness to a
project whose Constitution IV explicitly accepts leaving the `AppHandle`-needing shell
unit-untested, in exchange for pinning three lines of `emit`. The line between them is the
same one the project already draws.

What that leaves under test: `lock::recover` (both the recovery and the one-shot flag),
`Store::add`'s two error variants, and `Store::replace` under genuine thread contention.
What it leaves to the eye: that the flag is wired to an `emit`, and that the banner reads
sensibly. Stated here rather than discovered at review.

**Consequence for SC-002** ("all ten shared-state accesses recover"): six are store locks
and are safe by construction (R2). The other three (see R9) are call-site discipline, so a
cheap source-text guard test — assert the crate's sources contain no `.lock().unwrap()` —
is proposed to pin them. It is a blunt instrument and will need updating if the string ever
appears legitimately in a comment; it is proposed anyway because it is the only thing that
actually catches the eleventh site.

---

## R8. No frontend file changes

**Decision**: zero files under `src/` change. The frontend suite must pass unmodified
(SC-008).

**Rationale**: each of the two permitted user-visible changes already has a surface.

- FR-004's recovery warning is a `store-warning` event, which `main.ts` already listens
  for and routes to `showBanner`. `showBanner` sets `textContent`, so repeated warnings
  *replace* rather than accumulate — the spec's "must not stack into an unreadable banner"
  edge case holds today with no change.
- FR-010's refusal message rides `add_site`'s existing `Err(String)` channel, which
  `form.ts` already catches and renders in `#site-error` — the same place a rejected URL
  lands. Because `hooks.onSaved` is only called on success, no row appears; because
  `engine.start` moves after the refusal check, no timer starts. FR-009 is satisfied by
  the backend alone.

Confirmed by reading `src/main.ts`, `src/form.ts`, and `src/api.ts` rather than assumed.

---

## R9. The lock-site count goes 10 → 9, and that is the fix, not a miss

**Decision**: record the arithmetic in the plan so a reviewer does not read the missing
site as an oversight.

Today's ten: `commands.rs` ×6 (`list_sites`, `get_warning`, `add_site`, the `get` inside
`update_site`, the `update` inside `update_site`, `delete_site`), `engine.rs` ×3
(`start`, `stop`, `persist_get_fallback`), `lib.rs` ×1 (the startup `list`).

After: `update_site`'s two collapse into one `SharedStore::lock()` around `Store::replace`
— that collapse *is* User Story 3. So nine remain: six store locks routed through
`SharedStore` (safe by construction), two task-registry locks and one startup-warning lock
using `lock::recover(..)` directly and discarding the flag (silent, per R3).

---

## R10. `docs/ROADMAP.md` must be drained in the primary checkout, not here

**Decision**: the roadmap edit that retires §1 is applied in
`/Users/clint/src/clintcparker/site-checker`, not in this worktree.

**Rationale**: `docs/` is in `.gitignore`, so an edit made here is invisible to the merge
and the section stays stale on `main` — which is exactly what happened to
`003-durability` (its own roadmap entry records the miss). Carrying the lesson forward is
cheap; rediscovering it is not.

Note also that §1's fourth item is **not** drained: FR-018 keeps the symlink-replacement
note recorded as expected behaviour. The retirement note should say the section closed
three of four items and why the fourth stays.
