# Implementation Plan: Launch-at-login survives upgrades

**Branch**: `20260812-202608-autostart-launchagent-path` | **Date**: 2026-08-12 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/20260812-202608-autostart-launchagent-path/spec.md` ·
**Issue**: [#25](https://github.com/clintcparker/site-checker/issues/25)

## Summary

Site Checker records a version-pinned Homebrew Cellar path in its LaunchAgent, so launch-at-login
dies silently on the first `brew upgrade` while the checkbox still claims it is on. The cause is that
`tauri-plugin-autostart` builds its `AutoLaunch` from `current_exe().canonicalize()`, which resolves
Homebrew's stable `opt` symlink down to the keg that the next upgrade deletes — and the plugin
exposes no way to override that path.

The fix replaces the plugin with a direct `auto-launch` dependency and a new
`src-tauri/src/autostart.rs`, which derives the version-independent `…/opt/<formula>/…` path from the
running copy's own location, verifies it resolves to this same application, and registers it —
falling back to today's exact behaviour whenever no such path exists (hand-built copies, dev builds,
an unlinked keg). On every start it also reads any existing registration and rewrites a stale one in
place, never creating or deleting one. Finally it adds the missing removal step to the README and the
Homebrew formula's caveats. No UI change, no change to `sites.json`, no change to the Rust/TS
contract.

## Technical Context

**Language/Version**: Rust (stable ≥ 1.88, pinned by `rust-toolchain.toml`), edition 2021 ·
TypeScript ~5.6 (untouched by this feature)

**Primary Dependencies**: Tauri 2.11 · `auto-launch` 0.5 *(new direct dependency; already present
transitively)* · `tauri-plugin-autostart` 2.5 *(removed — see [research.md](./research.md) D1)*

**Storage**: None added. `sites.json` and the `autostart.initialized` marker are unchanged and are
not read or written by any code in this feature. The one file this feature writes is
`~/Library/LaunchAgents/Site Checker.plist`, owned by macOS.

**Testing**: `cargo test` (pure functions in the new module, plus a `tempfile`-backed test of the
path verification, following the pattern already used by `store.rs`) · `pnpm test` (unchanged;
frontend is not touched) · manual verification for the Homebrew upgrade path, per
[quickstart.md](./quickstart.md) §3–§4

**Target Platform**: macOS only. `.github/workflows/ci.yml` runs the Rust job on `macos-latest`
exclusively, so the new module can use the macOS `auto-launch` signatures directly.

**Project Type**: Desktop application (Tauri: Rust backend + vanilla TS frontend)

**Performance Goals**: The added startup cost is one `read_to_string` of a ~400-byte file and, only
when stale, one small write. It runs inline in `setup` — measured in microseconds, and deferring it
would race `set_autostart`.

**Constraints**: No failure introduced here may prevent startup or alter the site list (FR-008). No
user-visible change of any kind (spec assumption). The plist filename and `Label` must stay
`Site Checker`, or existing users' registrations are orphaned rather than repaired.

**Scale/Scope**: One new backend module (~120 lines plus tests), one edited `setup` block, one
`Cargo.toml` dependency swap, two documentation edits, one living-spec update.

**Unknowns**: none — all resolved in [research.md](./research.md).

## Constitution Check

*GATE: passed before Phase 0, re-checked after Phase 1 design. Constitution v1.0.0.*

| Principle | Assessment |
|---|---|
| **I. One Mac, One Person** | PASS. Fixes an existing single-user feature. Adds no alerting, history, sync, or service surface. Nothing about packaging or distribution is reopened — the formula edit is prose in `caveats` only. |
| **II. Results Are Ephemeral, Config Is Sacred** | PASS. `sites.json` is not read or written anywhere in this change; the store is loaded and the engine started *above* the touched block in `setup`. The LaunchAgent plist is macOS's file, not the app's config, and the change treats a file it cannot interpret exactly the way the store treats a corrupt `sites.json`: leave it untouched on disk. Departure worth naming: a corrupt store raises a visible warning, an uninterpretable plist deliberately does not (FR-007), because there is no action the user could take from it. |
| **III. Be a Polite Client** | PASS — not applicable. No HTTP behaviour changes. |
| **IV. Testable Core, Thin Shell** | PASS, and this shapes the design. Path derivation, plist parsing, and the repair decision are pure functions in `autostart.rs` under `cargo test`; the `AppHandle`-dependent shell is a handful of lines in `lib.rs` and two one-line command bodies. This is the same split as `model.rs`/`check.rs` versus `engine.rs`. |
| **V. The Rust/TS Contract Is snake_case, As-Is** | PASS. No persisted or event field is added, renamed, or serialised differently. `get_autostart` / `set_autostart` keep their names, arguments, and return types; `src/api.ts` is not edited. |
| **Quality Gates** | `cargo test`, `pnpm test`, and `cargo clippy -- -D warnings` all run in CI on `macos-latest` and must be green. |

**Post-Phase-1 re-check**: unchanged — PASS on all five. The Phase 1 design added no data, no
command, and no dependency beyond the swap already justified in research D1, which reduces the
backend's dependency count by one and removes an unused (and unpermitted) plugin command surface
rather than adding one.

**Complexity Tracking**: no violations to justify. The one structural decision worth flagging to
review — dropping `tauri-plugin-autostart` — is recorded as D1 in [research.md](./research.md) with
its four rejected alternatives, and is listed under Judgment Calls below.

## Project Structure

### Documentation (this feature)

```text
specs/20260812-202608-autostart-launchagent-path/
├── spec.md                          # Input (/speckit-specify)
├── plan.md                          # This file (/speckit-plan)
├── research.md                      # Phase 0 output
├── data-model.md                    # Phase 1 output
├── quickstart.md                    # Phase 1 output
├── contracts/
│   └── launch-agent-plist.md        # Phase 1 output
├── checklists/                      # From /speckit-specify
└── tasks.md                         # Phase 2 (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
src-tauri/
├── Cargo.toml                       # MODIFIED: -tauri-plugin-autostart, +auto-launch = "0.5"
├── capabilities/default.json        # UNCHANGED: core:default only; no plugin permission existed
└── src/
    ├── autostart.rs                 # NEW: path derivation, plist read, repair decision + tests
    ├── lib.rs                       # MODIFIED: drop .plugin(...), manage our AutoLaunch,
    │                                #           register with the desired path, repair on start
    ├── commands.rs                  # MODIFIED: get_autostart/set_autostart read our state
    │                                #           instead of the plugin's (signatures identical)
    ├── check.rs  engine.rs  lock.rs  model.rs  store.rs   # UNCHANGED
    └── main.rs                      # UNCHANGED

src/                                 # UNCHANGED — every frontend file, including api.ts and main.ts

install/homebrew/site-checker.rb     # MODIFIED: caveats gain the LaunchAgent removal step (FR-011)
README.md                            # MODIFIED: ### Uninstall gains the same step (FR-010)
capabilities/backend/spec.md         # MODIFIED: living spec for the launch-at-login capability
```

**Structure Decision**: The existing single-project Tauri layout is kept as-is — `src-tauri/src/`
for the backend, `src/` for the frontend, with `capabilities/` holding living specs (per
`living-specs.yml`, `src-tauri/src/**` maps to the `backend` capability). The one new file,
`src-tauri/src/autostart.rs`, sits alongside `check.rs` and `model.rs` as another pure-logic module
with its own `#[cfg(test)]` block, which is where this project already puts testable logic.

## Design

### The new module: `src-tauri/src/autostart.rs`

Four pure functions and two thin shells, in the split Principle IV asks for.

| Function | Kind | Responsibility |
|---|---|---|
| `stable_path(running: &Path) -> Option<PathBuf>` | pure | Rewrite `<prefix>/Cellar/<formula>/<version>/<rest>` → `<prefix>/opt/<formula>/<rest>` on the last `Cellar` component. `None` when there is no such segment. |
| `desired_path(running: &Path) -> PathBuf` | fs-reading, temp-dir testable | `stable_path`, accepted only if it exists **and** canonicalises to `running`; otherwise `running`. This is FR-001–FR-004 in one place. |
| `recorded_path(plist: &str) -> Option<String>` | pure | First `<string>` inside the `<array>` following `ProgramArguments`. `None` on anything that is not the shape we write. |
| `needs_repair(plist: Option<&str>, desired: &str) -> bool` | pure | `true` only when a path was read and differs. `None` (absent or uninterpretable) is `false`. FR-005–FR-007. |
| `manager(app) -> AutoLaunch` | shell | `AutoLaunchBuilder` with `app_name = package_info().name`, `use_launch_agent = true`, `app_path = desired_path(current_exe()?.canonicalize()?)`, `args = []`. |
| `repair_if_stale(&AutoLaunch, plist_path)` | shell | Read, decide, and on `true` call `enable()`. Every step swallows its error. |

### Wiring in `lib.rs`

The `setup` closure changes in three ways and in this order:

1. Build the `AutoLaunch` from `manager(app)` and `app.manage(...)` it, replacing
   `.plugin(tauri_plugin_autostart::init(...))` on the builder.
2. Leave the existing first-run marker block exactly as it is — same marker path, same
   write-even-on-failure rule, same warning text and same `get_or_insert_with` precedence over the
   store warning — but call `enable()` on our manager. On a genuine first run this already writes the
   corrected path, so step 3 finds nothing to do.
3. After the marker block and before `app.manage(AppState { … })`, call `repair_if_stale`. This is
   the only new startup behaviour, and it is unreachable-by-construction from the store: the site
   list was loaded and the engine started above, and nothing here touches either.

`commands.rs` keeps both command signatures and swaps `app.autolaunch()` for
`app.state::<AutoLaunch>()`; `set_autostart` still returns `is_enabled()` afterwards so the checkbox
can correct itself.

### Ordering guarantees

- First run: marker absent → `enable()` with the desired path → repair is a no-op.
- Upgraded install: marker present → repair reads the stale keg path, rewrites it to the `opt` path,
  file still present, checkbox unchanged.
- User opted out: no file → repair does nothing, marker prevents re-enabling.

### Documentation edits

Both removal surfaces gain the same line, placed next to the existing `/Applications` symlink step
because that is the other file the user must remove by hand:

```sh
rm ~/Library/LaunchAgents/"Site Checker.plist"
```

`install/homebrew/site-checker.rb` is a template rendered into `clintcparker/homebrew-tap` by the
release workflow, so editing it here is how the printed post-install notes change (FR-011).

## Risks and how they are handled

| Risk | Handling |
|---|---|
| A path containing an unrelated `Cellar` component produces a bogus registration | `desired_path` requires the rewritten path to exist *and* canonicalise back to the running executable. A coincidence fails both. |
| Repairing damages a registration the app did not write | `recorded_path` returns `None` for anything outside the exact template, and `needs_repair` maps `None` to "do nothing" (FR-007). |
| Repair re-enables a setting the user turned off | Repair is gated on the file already existing and never calls `enable()` when it is absent (FR-006, quickstart §2a). |
| Swapping the autostart implementation orphans existing users' registrations | Filename and `Label` both derive from `package_info().name`, which is kept. Same file, same shape — see the contract. |
| A failure in the new code blocks startup or loses the site list | Every step is error-swallowing; the store and engine are constructed above it and are never referenced by it (FR-008, quickstart §2d). |
| Running a dev build rewrites the developer's real login item | Accepted, and spec-mandated: the spec's "two copies exist" and "a development build is running" edge cases both say whichever copy the user runs is the one that registers itself. Listed as a judgment call below. |
| `auto-launch` changes its template in a future version | The version is pinned in `Cargo.lock` and the reader tests assert against the exact template, so a change fails `cargo test` rather than silently disabling repair. |

## Judgment Calls (unattended run — for the PR to surface)

No user was present; these were decided here and should be reviewed. They are in addition to the
three Open Decisions already recorded in [spec.md](./spec.md).

1. **`tauri-plugin-autostart` is removed, not worked around** (research D1). The plugin cannot be
   told which path to register, and nothing else in the repo uses it — `capabilities/default.json`
   never granted its JS commands, and `package.json` has no companion npm package. Reversible in one
   commit if review prefers a vendored fork or an upstream PR; the upstream `set_app_path` addition
   is worth doing separately either way.
2. **The plist is read with a small string scan rather than a `plist` crate** (research R3). The only
   files this must understand are ones this app wrote; anything else is meant to be left alone, and
   a strict reader gives that for free with no new dependency on the startup path. The accepted cost
   is that a hand-written or binary registration is never repaired.
3. **Repair runs inline in `setup`, unconditionally, including for development builds.** This follows
   the spec's edge cases literally. A `#[cfg(debug_assertions)]` guard would spare developers from
   having their real login item re-pointed at `target/debug`, at the cost of the debug and release
   paths no longer being the same code. Cheap to add if review wants it.
4. **Comparison of recorded versus desired path is exact string equality.** An
   equivalent-but-differently-spelled pre-existing path rewrites once and is then stable. The
   alternative — canonicalising both sides before comparing — would hide a genuinely dead path behind
   a failed `canonicalize` and skip the repair that is the whole point.

## Phase Status

- [x] Phase 0 — research complete, no NEEDS CLARIFICATION outstanding → [research.md](./research.md)
- [x] Phase 1 — design complete → [data-model.md](./data-model.md),
      [contracts/launch-agent-plist.md](./contracts/launch-agent-plist.md), [quickstart.md](./quickstart.md)
- [x] Constitution Check — passed pre-research and re-checked post-design
- [x] Phase 2 — `tasks.md` complete → [tasks.md](./tasks.md) (34 tasks, 3 user stories)
