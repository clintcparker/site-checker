# Implementation Plan: v1 Cleanup — Scaffold Leftovers & Message Clarity

**Branch**: `20260805-150909-cleanup-scaffold-leftovers` | **Date**: 2026-08-05 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/20260805-150909-cleanup-scaffold-leftovers/spec.md`

## Summary

Close all five items in section 1 of `docs/ROADMAP.md`: drop the unregistered opener
plugin (capability grant + both dependency declarations + both lockfiles), replace the
scaffold identity metadata, delete three unreferenced SVGs, reword the two near-identical
store warnings so they name distinct causes, and complete the `has_leading_scheme` doc
comment. Every change is either metadata, dead code, or user-facing wording — the app's
behavior is unchanged.

**The one non-obvious part is the rename.** `CFBundleExecutable` in the shipped bundle is
currently `tauri-app`, derived from the Cargo package name, and the *already-installed*
LaunchAgent at `~/Library/LaunchAgents/Site Checker.plist` hardcodes
`/Applications/Site Checker.app/Contents/MacOS/tauri-app`. A naive rename silently breaks
launch-at-login, and the `autostart.initialized` marker stops the app from repairing it.
The plan therefore renames the Cargo package *and* pins `"mainBinaryName": "tauri-app"` in
`tauri.conf.json`, exactly as the spec's edge case prescribes ("the shipped values must be
pinned explicitly so they stay put"). See [research.md](./research.md) R1 — this is the one
decision in the feature worth a second opinion.

## Technical Context

**Language/Version**: Rust 1.x (edition 2021, pinned via `rust-toolchain.toml`); TypeScript ~5.6

**Primary Dependencies**: Tauri 2.11, `tauri-plugin-autostart` 2.5, reqwest 0.13, tokio 1, serde/serde_json, url 2.5, uuid 1.24, rand 0.10; frontend `@tauri-apps/api` v2, Vite 6, vanilla TS (no framework)

**Storage**: `~/Library/Application Support/com.clintparker.site-checker/sites.json` — bare JSON array, snake_case keys. **Untouched by this feature.**

**Testing**: `cargo test` (backend: model, store, HTTP classifier — 29 tests), `pnpm test` / Vitest + happy-dom (frontend logic — 12 tests). Lint gate: `cargo clippy -- -D warnings`.

**Target Platform**: macOS desktop (single user, single machine)

**Project Type**: Desktop app — Tauri v2 shell, Rust backend, vanilla-TS frontend. Single project, existing layout.

**Performance Goals**: N/A — no runtime code path is touched.

**Constraints**: Observably identical app afterwards (FR-010). Shipped bundle name, identifier, window title, **and executable name** must not move (FR-004). Suite green and clippy clean after every item (FR-011).

**Scale/Scope**: 5 independent items, ~9 files. No new modules, no new dependencies, one new test.

## Constitution Check

*GATE: evaluated before Phase 0 and re-evaluated after Phase 1 design. Constitution v1.0.0.*

| Principle | Verdict | Reasoning |
|---|---|---|
| **I. One Mac, One Person** | ✅ PASS | Removes capability and code; adds no feature. Scope narrows, never widens. |
| **II. Results Are Ephemeral, Config Is Sacred** | ✅ PASS | `sites.json` shape, path, and key casing untouched. The principle's own guarantee — "a corrupt file is an empty list plus a visible warning… left untouched on disk until the next write" — is *preserved verbatim* by FR-007; only the warning's wording changes. Load still never fails (FR-008). |
| **III. Be a Polite Client** | ✅ PASS | No change to request method, headers, User-Agent, interval floor, or fallback logic. |
| **IV. Testable Core, Thin Shell** | ✅ PASS | The reworded messages live in `store::load`, already tested against a temp dir; the feature adds a test there rather than leaving the new distinction unpinned. `has_leading_scheme` is pure and already covered. No logic moves into the shell. |
| **V. Rust/TS Contract Is snake_case** | ✅ PASS | No `Site` or `StatusEvent` field is renamed, added, or re-cased. The warning crosses the boundary as a `String` payload whose *content* is not a contract field. |
| **Quality Gates** | ✅ PASS (planned) | `cargo test` + `pnpm test` + `cargo clippy -- -D warnings` are run per item, not just at the end (FR-011). Findings outside section 1 get appended to the roadmap, per the spec's Out of Scope. |

**Gate result: PASS — no violations, Complexity Tracking not required.**

**Post-Phase-1 re-check: PASS.** The design added one Cargo-package rename, one
`tauri.conf.json` key, one Rust test, and five text edits. Nothing introduced a new
abstraction, dependency, persisted field, or network behavior. The `mainBinaryName` pin is
the only addition that needs defending, and it defends Principle II's spirit rather than
straining it: it keeps an already-installed user's launch-at-login working across the
rename.

## Project Structure

### Documentation (this feature)

```text
specs/20260805-150909-cleanup-scaffold-leftovers/
├── spec.md              # Feature specification (already written)
├── checklists/
│   └── requirements.md  # Already written
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── build-identity.md
│   └── warning-messages.md
└── tasks.md             # Phase 2 — NOT created by /speckit-plan
```

### Source Code (repository root)

Existing layout; this feature edits the marked files and creates nothing new.

```text
package.json                          # ← name; drop @tauri-apps/plugin-opener   (US2, US3)
pnpm-lock.yaml                        # ← regenerate                             (US2)
index.html                            # (untouched — verified no <img> refs)

src/                                  # frontend (vanilla TS)
├── api.ts  form.ts  main.ts  render.ts  time.ts  styles.css
├── render.test.ts  time.test.ts
└── assets/                           # ← delete tauri.svg typescript.svg vite.svg,
                                      #   then the now-empty directory            (US4)

src-tauri/
├── Cargo.toml                        # ← name/description/authors/[lib] name;
│                                     #   drop tauri-plugin-opener               (US2, US3)
├── Cargo.lock                        # ← regenerate                             (US2, US3)
├── tauri.conf.json                   # ← add "mainBinaryName": "tauri-app"      (US3)
├── capabilities/default.json         # ← drop "opener:default"                  (US2)
└── src/
    ├── main.rs                       # ← tauri_app_lib → site_checker_lib       (US3)
    ├── lib.rs                        # (untouched — opener was never registered)
    ├── model.rs                      # ← has_leading_scheme doc comment         (US5)
    ├── store.rs                      # ← two warning strings + one new test     (US1)
    ├── check.rs  commands.rs  engine.rs
```

**Structure Decision**: Keep the existing single-project Tauri layout unchanged. This
feature is deliberately non-structural — creating directories or moving files would itself
violate FR-010's "observably identical" bar and make the clean-build comparison harder to
trust. The only structural change is a *deletion* (`src/assets/`).

## Implementation Sequence

Items are independent (spec Assumption: "may land in any order"), but two ordering
constraints are real and one is a convenience:

1. **US2 (opener) — the capability entry and the Cargo dependency must move together.**
   `tauri-build` resolves `"opener:default"` against installed plugins at compile time;
   removing `tauri-plugin-opener` from `Cargo.toml` while `capabilities/default.json`
   still grants `opener:default` fails the build. Removing the capability first (or both
   at once) is safe. Treat them as one atomic edit.
2. **US3 (rename) — `tauri.conf.json`'s `mainBinaryName` pin lands in the same commit as
   the `Cargo.toml` package rename.** A commit with the rename but no pin is a commit that
   silently breaks launch-at-login if anyone builds from it.
3. **Convenience: do US2 before US3.** Both regenerate `Cargo.lock`; doing the removal
   first keeps the two lockfile diffs readable instead of interleaved.

US1 (`store.rs`), US4 (assets), and US5 (doc comment) touch nothing the others touch and
can land in any position.

Per FR-011, the gate (`cargo test` + `pnpm test` + `cargo clippy -- -D warnings`) runs
after **each** item, not once at the end. The expensive whole-product checks — clean
rebuild and fresh `pnpm install` — run once at the end; see
[quickstart.md](./quickstart.md).

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Rename changes `CFBundleExecutable` and breaks the installed LaunchAgent | Pin `"mainBinaryName": "tauri-app"`; verify with `plutil -p` against the pre-change values recorded in [contracts/build-identity.md](./contracts/build-identity.md) |
| Stale build state masks an unresolved `tauri_app_lib` reference | `cargo clean` before the verification build (spec edge case; SC-005) |
| Stale `node_modules` masks a still-present opener dependency | `rm -rf node_modules && pnpm install` before verification (spec edge case) |
| `pnpm tauri build` hangs on the DMG's `osascript` step (roadmap §5) | Build with `--bundles app` — produces the `.app` needed for identity verification, skips the DMG re-layout entirely |
| Reworded messages drift apart from each other later | Pin the distinction in a test, not just in review (SC-006 requires test count to not drop) |

## Complexity Tracking

*Not required — Constitution Check passed with no violations.*
