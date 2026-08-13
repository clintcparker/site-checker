# Phase 1 Data Model: Clickable URLs Open in the Default Browser

## Persisted entities

**None added, none changed.**

`Site` (`src-tauri/src/model.rs`) is read by this feature and gains no field.
`sites.json` keeps its documented shape — a bare JSON array of snake_case
objects — and this feature never writes it. `StatusEvent` is untouched, so the
Rust/TS wire contract (Constitution V) is unchanged in both directions.

Per Constitution II, there is nothing here for the "config is sacred" rule to
protect and nothing new for the "results are ephemeral" rule to keep off disk:
the feature persists no state of any kind. No click history, no "last visited",
no per-site open count.

## Entities read

### `Site` — read-only

| Field | Read by this feature | Used for |
|---|---|---|
| `id` | yes | Row keying only; unchanged from today's use. |
| `url` | yes | The address displayed, guarded, and opened. Used **verbatim** — FR-006. |
| `label` | yes | Decides which slot in the name cell holds the URL (FR-008); never itself activatable. |
| `interval_secs` | no | — |
| `method_override` | no | — |

The feature must not mutate any of these (FR-010).

## Transient state

Two pieces of state exist, both in frontend memory only, both dying with the
window.

### Activation ledger

The record of when each URL was last successfully handed off, used to satisfy
FR-012.

| Property | Value |
|---|---|
| Shape | `Map<string, number>` — URL → epoch milliseconds of the last accepted activation |
| Lifetime | Module-scoped in `src/open.ts`; created at mount, never persisted, never synced |
| Keyed by | The URL string, **not** the site id — the rule is about the address being opened, so two sites pointing at the same URL share a window and one site whose URL was edited starts a fresh one |
| Growth | Bounded by the number of distinct URLs the user activates in one session. On the order of the site count; no eviction needed at this app's scale (Constitution I) |

**Validation rule** (`shouldOpen`, pure):

> An activation of `url` at time `now` is accepted if the ledger holds no entry
> for `url`, or if `now - ledger.get(url) >= ACTIVATION_WINDOW_MS`. On
> acceptance the ledger entry is set to `now`. A *rejected* activation does not
> update the entry — otherwise a user drumming on the control would extend the
> suppression indefinitely.

`ACTIVATION_WINDOW_MS = 1000`. Rationale in [research.md](./research.md) §5.

### Row URL activatability

Not stored anywhere — derived on each render from the site's URL.

**Derivation rule** (`isOpenable`, pure):

> A URL is activatable iff, after trimming, it begins with a case-insensitive
> `http://` or `https://` **and** parses as a URL with a non-empty host.

The host clause is what makes FR-007's second sentence hold. A prefix test alone
admits `https://`, `http://`, `https://[bad` and `https://exa mple.com` — all
four render as controls and are all refused by `openable_url`, which is exactly
the "presented as activatable but never opened" case FR-007 forbids (QA,
TC-105/TC-213). Asking the platform's `URL` parser for a host is the same pair
of questions the backend asks `url::Url`, and it stays synchronous and pure, so
the once-a-second repaint is unaffected.

Consequences:

- Activatable → rendered as a `<button class="site-url" data-open-url="{url}">`.
- Not activatable → rendered as inert text in a `<span>`, shown but not styled
  or announced as something that can be opened (FR-007, second sentence). Not
  hidden, not flagged as invalid, not repaired — spec Open Decision 3.

This is the frontend half of a rule deliberately spelled in two places; the
backend's `openable_url` is authoritative. See [research.md](./research.md) §4.

## State transitions

The URL element has no state machine of its own. Its element *type* changes only
on two user-initiated events, never on a repaint:

| Event | Transition | Handling |
|---|---|---|
| Repaint (status arrives, age ticks) | none | Element identity preserved — FR-011. The repaint path does not read `site.url`. |
| Edit changes the URL | inert `<span>` → `<button>` (one-way; `normalize_url` makes the reverse unreachable) | Node replaced within the name cell |
| Edit adds or removes a label | URL moves between the primary and secondary slot | Name cell's children rebuilt — [research.md](./research.md) §6 |
| Site deleted | row removed | Existing `renderTable` removal path; an open already dispatched is not cancelled and must not resurrect the row (spec Edge Cases) |

## Boundary types

`src/api.ts` gains one function on the single typed backend boundary the
frontend living spec mandates ("All backend access goes through one typed
boundary"):

```ts
export function openUrl(url: string): Promise<void>;
```

It resolves on success and rejects with the backend's bare `String` on failure —
the same shape `addSite`/`deleteSite` already produce, which is why `open.ts`
can reuse the `String(message)` idiom `form.ts` uses.

The argument name `url` is a single lowercase word, so Tauri's camelCase →
snake_case *argument* conversion is a no-op on it. No new instance of the
asymmetry Constitution V warns about.

Full contract: [contracts/open-url-command.md](./contracts/open-url-command.md).
