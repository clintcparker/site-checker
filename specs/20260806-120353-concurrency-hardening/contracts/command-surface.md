# Contract: Tauri Command Surface

**Feature**: [../spec.md](../spec.md) | **Plan**: [../plan.md](../plan.md) |
**Covers**: FR-009, FR-010, FR-011, FR-015, FR-016

The app's only external interface: seven `#[tauri::command]`s and two events, consumed by
`src/api.ts`. This is the contract the frontend is written against, so it is the one place
where "no user-facing change" (FR-016) has to be demonstrated rather than asserted.

**Bottom line: the wire format does not change at all.** No command is added, removed, or
renamed. No argument or return type changes. No event is added or renamed. `AddError`,
`Replaced`, and `SharedStore` are internal Rust types that never leave the backend — the
commands map them to the `Result<Site, String>` / `Result<(), String>` shapes that exist today.
`src/api.ts` compiles unchanged, and so does the rest of `src/`.

One command gains a new *reason* to return `Err`. That is the only behavioural delta.

---

## Commands

| Command | Args (camelCase in JS) | Returns | Change |
|---|---|---|---|
| `list_sites` | — | `Site[]` | none |
| `get_warning` | — | `string \| null` | none |
| `add_site` | `url`, `label`, `intervalSecs` | `Result<Site, String>` | **new `Err` case** — see below |
| `update_site` | `id`, `url`, `label`, `intervalSecs` | `Result<Site, String>` | none observable |
| `delete_site` | `id` | `Result<(), String>` | none |
| `get_autostart` | — | `Result<bool, String>` | none |
| `set_autostart` | `enabled` | `Result<bool, String>` | none |

## Events

| Event | Payload | Change |
|---|---|---|
| `site-status` | `StatusEvent` | none |
| `store-warning` | `{ message: string }` | **new occasion** — also raised when a poisoned site-list lock is recovered (FR-004). Same name, same shape, same handler. |

---

## `add_site` — the one behavioural change

### Before

Every `Store::add` failure went through `warn_on_write_failure` and the command returned
`Ok(site)` regardless. A refusal therefore produced: a "could not be saved" banner, a row in the
table, and a running timer — for a site held in no list, which vanished at the next launch.

### After

```text
normalize_url(url)?                         // unchanged — a bad URL is still Err, nothing persisted
build Site with a fresh v4 UUID             // unchanged

match store.lock().add(site.clone()) {
    Err(AddError::DuplicateId(_)) => return Err("<refusal message>"),   // ← new branch
    Err(AddError::Write(message)) => warn_on_write_failure(message),    // today's path
    Ok(())                        => {}
}

engine.start(site.clone())                  // now unreachable on a refusal
Ok(site)
```

| # | Guarantee | Requirement |
|---|---|---|
| C1 | On a refusal the command returns `Err` **before** `engine.start`, so no timer is created and no `site-status` is ever emitted for that site. | FR-009 |
| C2 | On a refusal no `store-warning` is emitted — one message, not two contradictory ones. | spec edge case |
| C3 | The refusal message says the site was **not added**, in terms that do not imply the change is being held un-saved. | FR-010 |
| C4 | On a write failure every one of today's behaviours is preserved: `Ok(site)` returns, the row appears, the timer starts, the banner reports the save failure. | FR-011 |
| C5 | On success, nothing changes. | FR-007 |

### Where the message lands, and why nothing in `src/` moves

`form.ts` already wraps `addSite` in `try/catch` and renders the caught string in `#site-error`
— the same inline slot a rejected URL uses. On the `Err` path `hooks.onSaved` is never called,
so no row is added; `resetToAddMode` is not called either, so the user's input survives for them
to look at. FR-009 and FR-010 are satisfied by the backend alone.

**Message wording** is left to implementation, with the constraint from FR-010 and one drafted
candidate:

> `That site was not added — the list already has an entry with the same identity. Nothing was changed.`

It must not use the word "saved", which is what makes the current message a lie.

### Reachability, stated plainly

`add_site` mints a fresh v4 UUID on every call, so the shipped window cannot reach the refusal
branch. It is fixed anyway, for the reason `003-durability` gave when it created the branch: the
invariant belongs to the layer that owns the list, not to one caller's id generator, and an
importer or restore path reopens it immediately.

---

## `update_site` — same wire contract, different insides

Rewritten as a caller of `Store::replace` (see [store-mutation-api.md](./store-mutation-api.md)).
Externally identical:

| # | Guarantee | Requirement |
|---|---|---|
| U1 | A bad URL still returns `Err` from `normalize_url` before anything else happens. | FR-007 |
| U2 | An unknown id still returns `Err("That site no longer exists")` and writes nothing. | FR-014 |
| U3 | The returned `Site` is the same value the old code would have returned, including `method_override`. | FR-014 |
| U4 | The site is still rescheduled, and only that site. | FR-007 |
| U5 | A failed save still returns `Ok(site)` with a banner. | FR-011 |

**Implementation note that belongs here because it is a correctness trap, not a style point**:
the store guard must be bound to a named variable in an explicit scope and dropped before
`engine.reschedule`, rather than left as a temporary in a `let ... else`. Holding the store lock
across the scheduling call is not a deadlock today, but it is a lock-ordering hazard nobody
should have to reason about later.

---

## What the frontend must **not** need

Recorded so a reviewer can check it by reading the diff's file list rather than the diff:

- No new command → no new `api.ts` wrapper.
- No new event → no new listener in `main.ts`.
- No new banner mechanism → `showBanner` unchanged. It assigns `textContent`, so repeated
  warnings replace rather than stack, which is the spec's "must not stack into an unreadable
  banner" edge case already satisfied today.
- No `Site` field change → no `render.ts` change, no `Site` interface change (Constitution V).

**Any diff touching `src/` is a scope violation under FR-016 and should be challenged at review.**
The 30 frontend tests must pass unmodified (SC-008).
