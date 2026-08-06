# Data Model: Concurrency & Robustness Hardening

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

Nothing persisted changes. `sites.json` keeps its location, its bare-array shape, its
snake_case keys, and its stage-then-rename write path (FR-015). This document therefore
describes **in-memory** structures only — the ones the feature adds, the ones it re-homes,
and the one state machine (lock poisoning) that the whole of User Story 1 turns on.

---

## Unchanged entities

Listed so their absence from the rest of the document is a statement rather than an omission.

| Entity | Where | Change |
|---|---|---|
| `Site` | `model.rs` | **None.** Same five fields, same snake_case serialization, same shape on disk and over IPC (Constitution V, FR-015). |
| `StatusEvent` | `model.rs` | **None.** |
| `Method` | `model.rs` | **None.** |
| `Store` (the list itself) | `store.rs` | Its *data* is unchanged — still a `PathBuf` and a `Vec<Site>`. Only its mutation API grows; see below. |
| `LoadOutcome` | `store.rs` | **None.** Load semantics are untouched. |
| `StoreWarning` (the emitted payload) | moves `commands.rs` → `lock.rs` | Same `{ message: String }` on the wire; only its home moves, so all banner traffic funnels through one place. |

---

## New: `SharedStore`

The site list as the rest of the app is allowed to see it.

| Field | Type | Notes |
|---|---|---|
| `inner` | `Arc<Mutex<Store>>` | **Private, with no accessor.** This is the entity's whole point. |
| `app` | `AppHandle` | Needed only to raise the FR-004 banner. |

**Cloneable** — `lib.rs` builds one and hands clones to `Engine` and `AppState`; both fields
are cheap to clone.

**Invariant it enforces**: a store lock cannot be taken un-recovered, and cannot forget to
warn. Because `inner` is private and there is no getter, `.lock().unwrap()` on the site list
is not merely discouraged — it is unwritable outside `lock.rs`. This replaces six of the ten
call sites with a structural guarantee (research R2, R9).

**Relationships**: replaces the `Arc<Mutex<Store>>` field in both `commands::AppState` and
`engine::Inner`. Neither type gains anything else.

---

## New: `AddError`

Two failure modes that today collapse into one `Err(String)`, separated because they carry
opposite promises (FR-008).

| Variant | Payload | What is true afterwards |
|---|---|---|
| `DuplicateId` | `String` (diagnostic) | **Nothing was applied.** The in-memory list and the file are both exactly as they were. |
| `Write` | `String` (the message shown in the banner) | **The change is in memory.** The list holds the new site; the save failed. |

**Produced by**: `Store::add` only. `Store::update` and `Store::delete` keep
`Result<(), String>` — neither has a refusal branch, so a shared error type would give both an
unreachable variant (research R4).

**Consumed by**: `commands::add_site`, which branches — `DuplicateId` returns `Err` to the
frontend before any timer starts (FR-009, FR-012); `Write` takes today's path unchanged, banner
and all (FR-011).

**Never serialized.** It does not cross the IPC boundary; `add_site` maps it to the existing
`Result<Site, String>`.

---

## New: `Replaced`

What `Store::replace` hands back on success.

| Field | Type | Notes |
|---|---|---|
| `site` | `Site` | The entry as it now stands, after the `method_override` rule was applied. The command returns this to the frontend and reschedules from it. |
| `write` | `Result<(), String>` | Whether the save that followed succeeded. `Err` keeps today's behaviour: the in-memory change stands and the banner fires. |

`Store::replace` returns `Option<Replaced>`; `None` means no site with that id, which the
command maps to today's `"That site no longer exists"`. The two are kept apart rather than
folded into one `Result` because "there was nothing to edit" and "the edit happened but did
not persist" demand opposite responses — the same conflation `AddError` exists to undo one
function over (research R5).

**Never serialized.**

---

## State: lock poisoning

The state machine User Story 1 is about. It belongs to every `Mutex` in the app, but only the
site list's transitions are user-visible.

```text
        ┌──────────┐   panic while holding the guard   ┌──────────┐
        │  Healthy │ ────────────────────────────────► │ Poisoned │
        └──────────┘                                   └──────────┘
             ▲                                               │
             │        lock::recover — into_inner()           │
             └───────────  + clear_poison()  ────────────────┘
                        (reports recovered = true)
```

| Transition | Trigger | Effect on the guarded data | User-visible |
|---|---|---|---|
| Healthy → Poisoned | a panic unwinds out of a critical section | whatever the panicking thread had already applied stays applied | nothing yet |
| Poisoned → Healthy | the next `lock::recover` on that mutex | **preserved verbatim** — `into_inner()` returns the data as left, nothing is reset or discarded (FR-006) | site list: one banner (FR-004). Check registry and startup warning: silent (FR-005, FR-003) |
| Healthy → Healthy | every ordinary lock | none | none (FR-007) |

**One warning per fault, not one per action.** Without `clear_poison`, the mutex would stay
poisoned for the life of the process and every later lock would re-report the same long-past
fault — a banner on every add, edit, delete, and background GET-fallback write. Clearing makes
the poison a one-shot signal, which is what the spec's "the app does not accumulate a permanent
degraded mode" edge case requires. Verified by probe (research R1).

**Recovery does not restart anything.** No site is re-scheduled, no list is reloaded, no check
is cancelled as a side effect of recovering (FR-006). Recovery is exactly: take the data as it
is, clear the flag, carry on.

---

## Which locks warn

Three mutexes, three answers — the FR-004 / FR-005 / FR-003 split, in one place.

| Mutex | Guards | Sites | On recovery |
|---|---|---|---|
| `SharedStore.inner` | the user's site list | 6 (`list_sites`, `add_site`, `update_site`, `delete_site`, `engine::persist_get_fallback`, `lib.rs` startup) | **Warns.** It is the one thing the app owns on disk, and the saved copy may not match what the user last asked for (FR-004). |
| `engine::Inner.tasks` | which sites have a running check | 2 (`start`, `stop`) | **Silent.** Ephemeral by design, rebuilt every launch, and every scheduling call replaces rather than accumulates (FR-005, Constitution II). |
| `commands::AppState.warning` | the one-shot startup message | 1 (`get_warning`) | **Silent.** Warning the user about a fault in the warning channel reports nothing they can act on (FR-003). |

Nine sites after the change, not ten: `update_site`'s two collapse into one, which *is* User
Story 3 (research R9).
