# Implementation Plan: Durability & Data Integrity

**Branch**: `20260806-102818-durability-and-data` | **Date**: 2026-08-06 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/003-durability/spec.md`

## Summary

`Store::save` rewrites `sites.json` in place with a plain `fs::write`, so a process that dies
mid-write leaves a truncated file and costs the user their whole list until they hand-repair
it. This plan closes that window by staging every save to a fixed-name sibling file and
publishing it with `std::fs::rename` — atomic at the VFS layer, so a reader sees either the
complete old contents or the complete new ones and never a mixture. Because `add`, `update`,
and `delete` already funnel through the one private `save`, that single change covers all
three mutations and no caller moves.

Two smaller integrity items ride along in the same layer: `normalize_url` lowercases the
leading scheme it returns (without switching to `url::Url`'s serialization, which would
re-introduce the trailing slash that function exists to avoid), and `Store::add` refuses an id
it already holds, before mutating anything.

Nothing the user sees changes. No dependency is added — this is `std::fs` throughout. All
three changes land in `model.rs` and `store.rs`, the already-tested pure layer, so the Tauri
shell and the entire frontend are untouched.

## Technical Context

**Language/Version**: Rust 1.97.1, edition 2021 (backend). Vanilla TypeScript, no framework
(frontend — not touched by this feature).

**Primary Dependencies**: tauri 2.11, serde / serde_json 1, url 2.5, uuid 1.24, reqwest 0.13,
tokio 1, tauri-plugin-autostart 2.5. **This feature adds none** — the atomic write is
`std::fs` only, and `tempfile` stays a dev-dependency (research R1).

**Storage**: one file — `~/Library/Application Support/com.clintparker.site-checker/sites.json`,
a bare pretty-printed JSON array of `Site` with snake_case keys. Shape, location, and load
semantics are frozen by FR-005 and FR-010.

**Testing**: `cargo test` (unit; `tempfile` for temp dirs, `httpmock` for the HTTP classifier),
`pnpm test` (vitest, frontend), `cargo clippy -- -D warnings`. Baseline re-confirmed in this
worktree on 2026-08-06: **Rust 29 passed / 0 failed**; frontend 30.

**Target Platform**: macOS desktop, Tauri 2 app bundle, APFS.

**Project Type**: desktop app — pure Rust core, thin Tauri command shell, vanilla TS frontend.

**Performance Goals**: none to speak of. Saves are user-initiated (one per add / edit /
delete), the file is a few KB, and the added rename plus a single `sync_all` on a file that
size is far below human perception. There is no throughput or latency budget to defend.

**Constraints**: no new runtime dependency; on-disk format and `load()` semantics unchanged
(FR-005, FR-010); no UI, event, or command-surface change (FR-011); at most one staging
artifact may exist at any time regardless of how many saves were interrupted (FR-003, SC-005);
the staging file must be a sibling of `sites.json` or the rename stops being atomic.

**Scale/Scope**: single user, single process, tens of sites. Two source files change
(`src-tauri/src/store.rs`, `src-tauri/src/model.rs`) plus `docs/ROADMAP.md`. Zero frontend
files.

**Unknowns**: none. The spec left no `[NEEDS CLARIFICATION]` markers, and the two places where
behaviour was genuinely uncertain — what `rename` does to a symlink and to a directory at the
destination — were settled by experiment rather than recall (research R5).

## Constitution Check

*GATE: evaluated before Phase 0, re-evaluated after Phase 1. Both passes recorded.*

| Principle | Verdict | Basis |
|---|---|---|
| **I. One Mac, One Person** | **PASS** | Nothing is added to the product. Every change hardens behaviour that already exists; no alerting, history, sync, or auth appears anywhere in this feature. |
| **II. Results Are Ephemeral, Config Is Sacred** | **PASS** — and this feature is that principle's most direct expression | The file's location, shape, and load semantics are untouched. The principle's own words — "a corrupt file is an empty list plus a visible warning, and the corrupt file is left untouched on disk" — are *preserved*, not replaced: research R8 explicitly declines to touch `load()`, and the existing `corrupt_file_yields_an_empty_list_a_warning_and_is_left_on_disk` test must pass unmodified as the pin. |
| **III. Be a Polite Client** | **PASS** | No request behaviour changes. One indirect effect: editing a URL's scheme case counts as a URL change, so `method_override` is cleared and HEAD support is re-learned — one extra request, for one site, once, under the existing rule. Well inside "would a WAF notice this?". |
| **IV. Testable Core, Thin Shell** | **PASS** | Both changed files are the pure layer: `model.rs` is plain functions, `store.rs` is tested against a temp dir. `commands.rs` and `engine.rs` are not modified (research R7 states the cost of that choice openly rather than hiding it). The staging/rename split exists so the crash-safety property is provable by a deterministic unit test instead of a flaky subprocess kill (research R4). |
| **V. The Rust/TS Contract Is snake_case, As-Is** | **PASS** | No serialized field name changes, no `rename_all` is added, the on-disk array shape is identical, and `to_string_pretty` stays. `Site` and `StatusEvent` are untouched. |
| **Quality Gates** | **PASS by construction** | SC-007 requires all three gates green after *each* story, not only at the end; the story split below is sequenced so each is independently shippable. |

**Post-Phase-1 re-evaluation**: unchanged, all PASS. The design artifacts introduced one new
on-disk entity (the staging artifact, [data-model.md](./data-model.md)) and one internal API
split (the staging step, [contracts/store-write-path.md](./contracts/store-write-path.md)).
Neither crosses the Tauri boundary, neither is serialized, and neither adds logic to the shell —
so no gate moves. The one behavioural divergence surfaced during design (rename replaces a
symlink rather than following it, research R5) is a spec-prose correction, not a constitution
question: nothing is destroyed and the user's data survives either way.

**Violations requiring justification**: none. Complexity Tracking below is therefore empty.

## Project Structure

### Documentation (this feature)

```text
specs/003-durability/
├── spec.md                       # Input (already written)
├── checklists/requirements.md    # Already written
├── plan.md                       # This file
├── research.md                   # Phase 0 — R1..R9, all mechanism decisions
├── data-model.md                 # Phase 1 — entities, invariants, file state transitions
├── contracts/
│   ├── store-write-path.md       # Phase 1 — Store API + failure + artifact contracts
│   └── normalize-url.md          # Phase 1 — input→output table
├── quickstart.md                 # Phase 1 — how to prove it works, per story
└── tasks.md                      # Phase 2 — NOT created by /speckit-plan
```

### Source Code (repository root)

```text
src-tauri/src/
├── model.rs      # CHANGED — has_leading_scheme → returns Option<usize>;
│                 #           normalize_url lowercases the scheme slice (US2)
├── store.rs      # CHANGED — save split into stage + rename (US1);
│                 #           add gains a duplicate-id refusal (US3)
├── commands.rs   # unchanged — warn_on_write_failure is the reporting channel as-is
├── engine.rs     # unchanged
├── check.rs      # unchanged
├── lib.rs        # unchanged
└── main.rs       # unchanged

src/              # unchanged in full — no frontend file is touched by this feature
index.html        # unchanged

docs/ROADMAP.md   # CHANGED — section 1 emptied (SC-008); two deferrals appended (R5, R7)
```

**Structure Decision**: no structural change. This is a brownfield hardening feature and it
deliberately edits two existing files in place. Both live in the pure core the constitution
reserves for testable logic, and every new test goes into the `mod tests` already at the bottom
of each of those files — same location, same style, same `a_site` helper.

## Implementation Sequence

Three stories, in spec priority order. Each is independently shippable and each ends with all
three gates green (SC-007). Ordering is by value, not by dependency — none of the three depends
on another, so US1 goes first because it is the only one that protects real user data.

**US1 — atomic saves (P1)** · `store.rs`
Split `save` into a private staging step (`create_dir_all`, serialize, write
`sites.json.tmp` in the same directory, `sync_all`, return the path) and `save` itself
(staging step, then `fs::rename`). Add tests for: interrupted-before-publication, completed
save, no orphan after success, at-most-one orphan after repeated staging, and a failed save
leaving the previous file intact. Existing store tests pass unmodified.

**US2 — lowercase scheme (P2)** · `model.rs`
`has_leading_scheme` returns `Option<usize>`; `normalize_url` builds `candidate` from the
lowercased scheme slice plus the remainder verbatim. Add the table from
[contracts/normalize-url.md](./contracts/normalize-url.md), including the no-trailing-slash
regression guard. No existing `model::` test should need editing.

**US3 — duplicate-id refusal (P3)** · `store.rs`
`Store::add` returns `Err` before the push when the id is present. Three tests: refused,
nothing written, distinct id still fine.

**Wrap-up** · `docs/ROADMAP.md`
Empty section 1 (SC-008), and append the two items this plan deliberately deferred rather than
dropped: the symlink edge-case correction (R5) and the `add_site` refusal-vs-write-failure
inconsistency (R7).

## Complexity Tracking

> No Constitution Check violations. Nothing to justify.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| *(none)*  | —          | —                                    |

## Notes carried into implementation

- **`docs/` is gitignored in this repo.** The roadmap edit required by SC-008 will not appear
  in `git status` and will not be committed. It still has to be made — it is the record that
  section 1 was drained — but do not expect it in the PR diff.
- **Living specs are enabled** (`living-specs.yml`, capability `backend` matching
  `src-tauri/src/**`) with nothing registered yet. Both changed files fall under that
  capability, so a `/speckit-companion-living-*` pass may want to run after implementation.
  Out of scope for this plan.
- **Back up the real `sites.json` out of tree** before any manual verification run. See
  [quickstart.md](./quickstart.md).
