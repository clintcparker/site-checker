# Site Checker — v1 Design

**Date:** 2026-07-23
**Status:** Approved

## Purpose

A personal status dashboard for the websites and endpoints the user cares about
(`example.com`, `myapp.com`, internal tools). It answers exactly one question at
a glance: **is this thing up, and how long ago did we last confirm that?**

It is not a monitoring service. There is no alerting, no history, no SLA math,
no multi-user anything. It runs on one Mac, for one person.

A hard constraint runs through the whole design: **be a polite client.** Checks
must never look like abuse — no aggressive intervals, no cache-busting query
strings, no unusual request shapes that would trip a WAF or earn a 429.

## Decisions

Each of these was chosen explicitly during design. They are settled, not
defaults to be revisited during implementation.

| Decision | Choice | Rationale |
|---|---|---|
| App form | Standard window app (dock icon, resizable window) | Room for the full list as a table; no menu-bar extra to build |
| Stack | Tauri (Rust backend + web UI) | ~5MB bundle, OS webview, no bundled Chromium; suits an app that idles all day |
| "Green" means | HTTP status **200–399** | User wants "is my app working", not just "is the box alive" |
| Request method | **HEAD**, falling back to **GET** on 405/501 | Lightest possible traffic that still works on HEAD-hostile servers |
| Cache handling | Local HTTP cache **disabled**; no cache-busting params | Every check is a real network request. A CDN-cached response is fine — the requirement is that the request goes out, not that it reaches origin |
| Default interval | **60s**, configurable per URL | User's stated starting point |
| Interval floor | **10s** (lower values clamp up) | Guardrail against accidentally hammering an endpoint |
| Check timeout | **10s** | Beyond this, treat as Down |
| Launch at login | **Yes** | Should usually already be running |
| Window close | **Quits the app** | User's explicit choice; keeps lifecycle simple — no hidden background state |

## Architecture

Two units with one boundary between them:

- **Rust backend** owns networking, timers, classification, and persistence.
- **Web UI** owns rendering and input. It performs no networking of its own.

The UI sends commands via Tauri `invoke` and receives results via Tauri events.
This keeps all "is it up?" logic in one testable place and makes the UI a thin,
replaceable view.

```
┌─────────────── Web UI (webview) ───────────────┐
│  site table · add form · live "time since"     │
└──────┬──────────────────────────▲──────────────┘
   invoke(commands)          emit(site-status)
┌──────▼──────────────────────────┴──────────────┐
│                Rust backend                    │
│   ┌──────────────┐        ┌────────────────┐   │
│   │ Store (JSON) │◄──────►│ Checker engine │   │
│   └──────────────┘        └───────┬────────┘   │
└───────────────────────────────────┼────────────┘
                                    ▼
                            the wider internet
```

### Component 1 — Checker engine (Rust)

**Does:** runs one recurring check per site and reports the result.
**Used via:** `start(site)`, `stop(site_id)`, `reschedule(site)`; emits results.
**Depends on:** `reqwest`, `tokio`.

Per check:

1. Send `HEAD <url>` with a browser-like `User-Agent`, redirects followed (up to
   10 hops), 10s timeout, local cache disabled.
2. If the response status is `405` or `501`, immediately retry the same URL with
   `GET`, and persist `method_override = GET` on that site so future checks skip
   straight to GET.
3. Classify the final response (the one after redirects resolve):
   - status in `200..=399` → **Up**. In practice a followed redirect resolves to
     a 2xx; a *final* 3xx only appears when the 10-hop limit is hit, and per the
     rule above that still counts as Up.
   - any other status → **Down** (reason: `HTTP <code>`)
   - transport error (DNS, connect refused, TLS, timeout) → **Down**
     (reason: short error string)
4. Emit `site-status { id, state, checked_at, reason? }`.

**Jitter:** each site's first check is offset by a small random delay so that N
sites configured at 60s do not all fire on the same second. Subsequent checks
keep that offset.

### Component 2 — Store (Rust)

**Does:** persists the user's list of sites.
**Used via:** `load()`, `add(site)`, `update(site)`, `delete(id)`, `list()`.
**Depends on:** `serde_json`, the Tauri app-config dir.

File: `~/Library/Application Support/com.clintparker.site-checker/sites.json`

```jsonc
[
  {
    "id": "uuid",
    "url": "https://example.com",
    "label": "Marketing site",   // optional
    "interval_secs": 60,
    "method_override": null       // or "GET" once HEAD is known to fail
  }
]
```

Loaded once at launch; rewritten on every add/update/delete.

**Check results are never written to disk.** Live status lives in memory only
and is empty again on next launch (every site starts Pending). This is a
deliberate consequence of "no history in v1".

### Component 3 — Web UI

**Does:** renders the site list and accepts input.
**Depends on:** Tauri `invoke` + event APIs only.

A single table, one row per site:

```
URL / label            Status        Last checked
example.com            🟢 Up          5s ago
myapp.com              🔴 Down        60s ago
api.foo.dev            ⚪ Pending      —
```

- **Status dot:** green (Up), red (Down), grey (Pending — not yet checked this
  session). Hovering a red dot shows the failure reason.
- **Last checked:** a relative time that ticks live in the UI (`5s ago` →
  `59s ago` → `3m ago`) on a local 1s timer. It counts from the last *completed*
  check and does not require backend chatter.
- **Add form:** URL (required), label (optional), interval in seconds (defaults
  to 60).
- **Per row:** edit and delete.
- **Launch at login:** a single checkbox in the window footer, **on by default**,
  registered on first run. Unchecking removes the macOS login item. This is the
  app's only global setting.

### Data flow

**User adds a site:** UI validates non-empty → `invoke("add_site", …)` → Store
writes JSON → engine starts a timer for it → first check fires after jitter →
`site-status` event → row turns green or red.

**Steady state:** engine timer fires → HTTP check → `site-status` event → UI
updates dot and resets that row's "time since" counter to 0.

**User edits an interval:** `invoke("update_site", …)` → Store rewrites →
engine reschedules that one site. Other sites are untouched.

## Error handling

| Condition | Behavior |
|---|---|
| Malformed / unparseable URL | Rejected at add-time with an inline form error; nothing is persisted |
| Missing scheme (`example.com`) | Normalized to `https://` before saving |
| DNS / connect / TLS / timeout failure | Row shows Down with a short reason on hover. Not an app error — this is the product working |
| `sites.json` missing | Treated as an empty list; created on first add |
| `sites.json` corrupt | Non-fatal banner in the UI, app starts with an empty list, the bad file is left untouched (not overwritten) so it can be recovered by hand |
| Store write failure | Non-fatal banner; the in-memory change stands so the session keeps working |

## Testing

Rust unit tests, driven against a local `httpmock` server:

- **Classifier:** 200/301/399 → Up; 400/404/429/500 → Down; each transport
  error → Down with a reason.
- **HEAD→GET fallback:** a server that 405s on HEAD and 200s on GET yields Up,
  and sets `method_override = GET`.
- **Interval clamping:** an interval below 10 is stored as 10.
- **URL normalization:** `example.com` → `https://example.com`.
- **Store round-trip:** add → load → update → delete against a temp dir;
  corrupt-file handling returns an empty list rather than panicking.

No UI end-to-end tests in v1.

## Out of scope for v1

Deferred deliberately, each an easy later addition:

- Desktop notifications on up→down / down→up transitions
- Status history, sparklines, uptime percentages
- A manual "check now" button
- A menu-bar icon
- Continuing to run after the window is closed
- Per-URL expected-status configuration
- Auth headers / private endpoints

## Setup prerequisite

Rust is **not** currently installed on this machine (Node 26, pnpm 10, and
Xcode Command Line Tools are present). Tauri requires it. First implementation
step is installing Rust via `rustup` — approved by the user during design.
