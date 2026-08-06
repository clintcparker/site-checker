# Contract: Lock Recovery (`src-tauri/src/lock.rs`)

**Feature**: [../spec.md](../spec.md) | **Plan**: [../plan.md](../plan.md) |
**Covers**: FR-001, FR-002, FR-003, FR-004, FR-005, FR-006, FR-007

An internal Rust contract — nothing here crosses the IPC boundary. It is written down because
the module deliberately sits on both sides of the project's central split: `recover` is a
Tauri-free generic that `cargo test` drives directly, and `SharedStore` is the thin shell that
needs an `AppHandle`.

---

## `pub fn recover<T>(mutex: &Mutex<T>) -> (MutexGuard<'_, T>, bool)`

Lock a mutex, surviving a prior poisoning. The `bool` is `true` when **this call** was the one
that found and cleared a poison.

### Guarantees

| # | Guarantee | Requirement |
|---|---|---|
| L1 | Never panics and never returns an error, whatever the mutex's state. | FR-001, FR-002, FR-003 |
| L2 | On an unpoisoned mutex, returns the guard and `false` — indistinguishable from `lock().unwrap()`. | FR-007 |
| L3 | On a poisoned mutex, returns a guard over the data **exactly as the panicking thread left it**. Nothing is reset, replaced, or discarded. | FR-006 |
| L4 | Clears the poison, so a subsequent `recover` on the same mutex returns `false` — one report per fault, not one per access. | spec edge case "no permanent degraded mode" |
| L5 | The clear happens while the returned guard is still held, so two threads cannot both report the same fault. | — |
| L6 | Blocks exactly as `Mutex::lock` does; no change to contention behaviour or hold time. | FR-007 |

### Implementation shape

```rust
match mutex.lock() {
    Ok(guard) => (guard, false),
    Err(poisoned) => {
        mutex.clear_poison();      // under the guard, per L5
        (poisoned.into_inner(), true)
    }
}
```

`Mutex::clear_poison` is stable since Rust 1.77; this project is on 1.97.1. All six guarantees
were confirmed by probe against that toolchain before this contract was written
(research R1) — they are not inferred from documentation.

### Callers that discard the flag

Two of the app's three mutexes recover silently, and both do so by writing
`lock::recover(..).0` with a comment naming the reason:

- `engine::Inner.tasks` — which checks are running is ephemeral by design, rebuilt at every
  launch, and every scheduling call replaces rather than accumulates (FR-005, Constitution II).
- `commands::AppState.warning` — a one-shot `Option<String>`; a warning about the warning
  channel names no consequence (FR-003).

Discarding the flag is a decision, not an oversight. A reviewer seeing `.0` should find the
comment; if there is no comment, that is the finding.

---

## `pub struct SharedStore`

The site list as the rest of the app is allowed to see it: `Arc<Mutex<Store>>` plus an
`AppHandle`, both private, no accessor for either.

### `SharedStore::new(app: AppHandle, store: Store) -> Self`

Takes ownership of the loaded `Store`. Called once, from `lib.rs`'s `setup()`.

### `SharedStore::lock(&self) -> MutexGuard<'_, Store>`

| # | Guarantee | Requirement |
|---|---|---|
| S1 | Recovers per `recover` above — never panics on a poisoned lock. | FR-001 |
| S2 | When and only when the recovery flag is set, emits one `store-warning` event telling the user their saved list may not reflect their most recent change. | FR-004 |
| S3 | In the absence of a fault, emits nothing and behaves identically to today's `lock().unwrap()`. | FR-007 |
| S4 | The guard is a plain `MutexGuard<Store>` — every existing `store.list()`, `.get()`, `.add()`, `.update()`, `.delete()` call site reads the same as before. | — |

**The structural guarantee**: `inner` is private and has no getter, so `.lock().unwrap()` on
the site list is not merely discouraged, it is unwritable outside this module. That is what
makes FR-001 hold for the eleventh call site as well as the current ten (research R2).

**Startup caveat**: `lib.rs` calls `lock()` during `setup()`, before the window's JS has
registered its `store-warning` listener, so a warning emitted there would be dropped — Tauri
events have no replay. This is inert rather than a bug: the store is constructed a few lines
earlier and cannot be poisoned yet. Worth a comment at that call site so a future reader does
not mistake it for a missing case.

### `SharedStore::warn_on_write_failure(&self, result: Result<(), String>)`

Moved from `commands.rs` unchanged in behaviour: an `Err` emits `store-warning` with the
message, an `Ok` does nothing. It lives here so **every** banner in the app is raised from one
place — the recovery warning and the write-failure warning are the same surface, as FR-004
requires ("using the existing warning banner rather than a new mechanism").

Its long-standing doc comment ("the in-memory change stands") becomes true again as a result of
this feature: the one branch that violated it — a refused add — no longer reaches it. See
[store-mutation-api.md](./store-mutation-api.md).

### `StoreWarning`

The emitted payload, `{ message: String }`, moves here from `commands.rs`. Same event name
(`store-warning`), same field, same JSON. The frontend's `onStoreWarning` needs no change.

---

## Out of scope for this contract

`SharedStore::lock`'s emit side is **not** unit-tested. Driving it needs an `AppHandle`, which
needs `tauri`'s `test` feature and a mock-app harness — declined in research R7 in favour of a
manual check, on the same line Constitution IV already draws around the `AppHandle`-needing
shell. What that leaves verified by machine is `recover` itself; what it leaves verified by eye
is that the flag is wired to an `emit`, per [../quickstart.md](../quickstart.md).
