<!--
Sync Impact Report
==================
Version change: (template) → 1.0.0
Bump rationale: Initial ratification, drafted during spec-kit harness setup from the shipped v1
design (docs/superpowers/plans/2026-07-23-site-checker.md), the README, and the code as merged —
not from aspiration. Every principle below traces to something v1 actually does or deliberately
refuses to do. Review before relying on it for contested calls.

Deliberately NOT codified:
  Commit message convention — history is mixed (conventional-commit style dominates recently but
  is not universal); recording a MUST would invent a rule the history does not support.
  ROADMAP items — deferred work is tracked in docs/ROADMAP.md (untracked, local); the
  constitution constrains how features are built, not which ones are next.
-->

# Site Checker Constitution

## Core Principles

### I. One Mac, One Person
Site Checker is a personal status dashboard, not a monitoring service. It answers exactly one
question — "is this thing up, and how long ago did we last confirm that?" — for one person on one
Mac. Alerting, notifications, history, SLA math, multi-machine sync, and auth-gated checks are
out of scope by design, not by omission. A spec that adds any of these MUST say explicitly that
it is widening the product's scope and why; "it's easy to add" is not a reason.

### II. Results Are Ephemeral, Config Is Sacred
The only file this app owns is `~/Library/Application Support/com.clintparker.site-checker/sites.json`
— the user's list, snake_case keys, a bare JSON array, exactly the documented shape. Check
results are never written to disk: live status is in-memory only, every site starts Pending on
launch, and Pending is a UI-only state (the backend emits only `up`/`down`). Loading the store
never fails — a missing file is an empty list, a corrupt file is an empty list plus a visible
warning, and the corrupt file is left untouched on disk until the next write so it can be
recovered by hand.

### III. Be a Polite Client
Checks look like an ordinary browser and impose minimal load: HEAD by default, falling back to
GET only on `405`/`501`, with that discovery persisted per site as `method_override` so the
retry is never repeated. A browser-like User-Agent, no cache-busting query strings, no unusual
headers, no response caching layer, and a hard interval floor (`MIN_INTERVAL_SECS = 10`;
lower values clamp up). Any change to request behavior MUST be weighed against "would a WAF or
a rate limiter notice this?"

### IV. Testable Core, Thin Shell
Everything worth testing is a pure function with no Tauri or filesystem dependency: the URL/
interval validation and the HTTP classifier live in `model.rs` and `check.rs` and are tested by
plain `cargo test`; `store.rs` is tested against a temp dir. `engine.rs` (needs an `AppHandle`)
stays thin — scheduling only, no classification logic — which is what makes leaving it
unit-untested acceptable. The frontend is vanilla TypeScript, no framework; its testable logic
(relative-time formatting, rendering) is likewise separated from DOM wiring. New code follows
this split: logic that can be pure MUST be pure and tested; shells stay thin enough not to need
tests.

### V. The Rust/TS Contract Is snake_case, As-Is
`Site` and `StatusEvent` cross the Tauri boundary with snake_case field names and the frontend
reads them as-is (`interval_secs`, `checked_at`, `method_override`). Do not add
`#[serde(rename_all = "camelCase")]` or a translation layer: `Site` must serialize to snake_case
to match `sites.json`, and `StatusEvent` stays consistent with it. Command *arguments* follow
Tauri's convention (camelCase in JS, snake_case in Rust). Changing any persisted or event field
name is a breaking change to the user's data file and MUST be treated as such.

## Quality Gates

The merge bar demonstrated by v1 and maintained since:

- `cargo test` green (backend: model, store, HTTP classifier)
- `pnpm test` green (frontend logic)
- `cargo clippy -- -D warnings` clean
- No Critical or Important findings open from review; Minor findings either fixed or recorded in
  the roadmap rather than silently dropped.

## Development Workflow

Features flow through the spec-kit cycle (specify → plan → tasks → implement → ship), each in
its own worktree and feature branch, ending in a PR against `main`. Deferred and out-of-scope
findings are appended to the roadmap instead of expanding the feature mid-flight. The `specs/`
directory is the record of what was asked; the code comments record why the tricky parts are
the way they are — keep both honest.

## Governance

This constitution captures the project's actual practices; it supersedes habit but not the
user's explicit direction. Amendments happen in a PR that updates this file with a Sync Impact
Report (version bump, what changed, why). Versioning is semantic: MAJOR for removing or
redefining a principle, MINOR for adding or materially expanding one, PATCH for wording.
Compliance is checked at plan time (Constitution Check) and at review; a principle the code
contradicts must be amended or defended, never silently ignored.

**Version**: 1.0.0 | **Ratified**: 2026-08-05 | **Last Amended**: 2026-08-05
