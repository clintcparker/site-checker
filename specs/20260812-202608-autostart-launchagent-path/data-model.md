# Phase 1 Data Model: Launch-at-login survives upgrades

**Feature**: `20260812-202608-autostart-launchagent-path` · **Spec**: [spec.md](./spec.md) · **Research**: [research.md](./research.md)

This feature adds **no persisted application data** and **no new fields to any type that crosses the
Rust/TS boundary**. `Site` and `StatusEvent` are untouched, `sites.json` is untouched, and the
`get_autostart` / `set_autostart` command signatures are unchanged. What follows models the two
things the spec names as entities — both of which live outside the app's own storage — plus the
small internal values used to reason about them.

---

## Entity: Launch-at-login registration

The record that tells macOS to open Site Checker at login. Owned by macOS, written by this app,
removed only by the user.

| Property | Value |
|---|---|
| Location | `~/Library/LaunchAgents/{app_name}.plist` — for this app, `~/Library/LaunchAgents/Site Checker.plist` |
| `app_name` | `app.package_info().name`, i.e. `productName` from `src-tauri/tauri.conf.json` = `Site Checker` |
| Format | XML plist, the fixed template written by `auto-launch` (see [contracts/launch-agent-plist.md](./contracts/launch-agent-plist.md)) |
| Fields that matter | `Label` (= `app_name`), `ProgramArguments[0]` (the recorded executable path), `RunAtLoad` (always `true`) |
| Existence | **is** the enabled/disabled state — the checkbox reads nothing but whether this file is present |
| Lifecycle | Created by `enable()` (first run, or the user ticking the box) · overwritten in place by `enable()` (repair) · deleted only by `disable()` (the user unticking the box) · never deleted by the app on its own (FR-012) |

### States

| State | Meaning | What the app does on start |
|---|---|---|
| Absent | The user has it off (or has never run the app) | First run only: create it. Otherwise: nothing (FR-006) |
| Present, path current | Correct | Nothing (FR-005, third scenario) |
| Present, path stale | Points at a version-scoped or otherwise superseded location | Overwrite with the current desired path, still enabled (FR-005) |
| Present, unreadable | Not in the shape this app writes | Nothing at all — no repair, no warning, normal startup (FR-007) |

### Transitions

```text
                 first run (marker absent)                user ticks box
        Absent ─────────────────────────────► Present ◄──────────────────── Absent
                                                 │
          ▲                                      │ start, recorded ≠ desired
          │                                      ▼
          └──────────── user unticks box ──── Present (rewritten, still enabled)
                          (disable)
```

There is deliberately **no** transition out of `Present` that the app initiates. Repair moves
`Present → Present`; only the user's untick reaches `Absent`.

### Invariants

- **I1** — The app never transitions `Absent → Present` except on first run (marker file missing).
  Repair is gated on the file already existing.
- **I2** — The app never transitions `Present → Absent`. `disable()` is reachable only from
  `set_autostart(false)`, i.e. a direct user action.
- **I3** — What `get_autostart` reports is unchanged by any repair, because repair preserves
  existence.
- **I4** — No failure in reading, deciding, or writing this file may propagate out of app startup or
  touch the site list.

---

## Entity: Install location

Where the running copy lives. Not stored anywhere; derived on each start.

| Value | Definition | Example |
|---|---|---|
| **Running path** | `std::env::current_exe()?.canonicalize()?` — every symlink resolved, so this is always the physical location | `/opt/homebrew/Cellar/site-checker/1.0.0/libexec/Site Checker.app/Contents/MacOS/site-checker` |
| **Stable path** | The running path with a `Cellar/<formula>/<version>/` segment replaced by `opt/<formula>/`. Exists only for a package-managed copy | `/opt/homebrew/opt/site-checker/libexec/Site Checker.app/Contents/MacOS/site-checker` |
| **Desired path** | Stable path if it exists *and* canonicalises back to the running path; otherwise the running path. This is what gets recorded | either of the above |

### Derivation rules

1. Split the running path into components. Find the **last** component equal to `Cellar`.
2. Require at least `<formula>` and `<version>` after it, and a non-empty remainder after those.
   Anything less → no stable path.
3. Stable path = `<everything before Cellar>` + `opt` + `<formula>` + `<remainder>`.
4. Accept the stable path only if it exists and `canonicalize(stable) == running path`. Otherwise
   the desired path is the running path.

Rule 4 is what makes rules 1–3 safe to state loosely: a coincidental `Cellar` component anywhere in
a user's own directory tree produces a path that either does not exist or is not this application,
and the fallback in FR-003/FR-004 catches it.

### Cases

| Running copy | Stable path found? | Desired path |
|---|---|---|
| Homebrew keg, `opt` linked | yes, verifies | stable (`opt/…`) |
| Homebrew keg, launched via the `/Applications` symlink | yes — `canonicalize` collapses to the keg first, so this is the same case | stable (`opt/…`) |
| Homebrew keg, `opt` symlink missing or dangling | derived but fails verification | running (unchanged behaviour) |
| Hand-built copy in `/Applications` | no `Cellar` component | running (unchanged behaviour) |
| `pnpm tauri dev` build under `target/debug` | no `Cellar` component | running (unchanged behaviour) |
| Path containing an unrelated `Cellar` directory | derived but fails verification | running (unchanged behaviour) |

---

## Internal values

Not persisted, not sent to the frontend; named here because the tests are written against them.

| Value | Type | Meaning |
|---|---|---|
| Recorded path | `Option<String>` | `ProgramArguments[0]` extracted from the registration. `None` = absent or uninterpretable — the two cases that mean "do nothing", so they collapse deliberately |
| Repair decision | `bool` | `true` only when a recorded path was successfully read **and** differs from the desired path |

Comparison of recorded vs desired is exact string equality. Both sides are absolute and both are
produced by the same derivation, so an equivalent-but-differently-spelled path (a pre-existing
`/private/var` vs `/var` registration, say) rewrites once and is stable from then on — the operation
is idempotent, so a spurious first rewrite costs one file write and nothing else.

---

## Unchanged by this feature

- `sites.json` — shape, keys, and location. Never read or written by any code added here.
- `Site`, `StatusEvent` — no new or renamed fields; the snake_case contract (Principle V) is not
  touched.
- `autostart.initialized` marker — same location, same meaning, same write-even-on-failure rule.
- The `get_autostart` / `set_autostart` commands — same names, arguments, and return types.
- Every frontend file. No UI change of any kind.
