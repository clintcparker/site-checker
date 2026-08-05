# Phase 1 Data Model — v1 Cleanup

## Persisted and cross-boundary data: unchanged

This feature adds, removes, and renames **no** data. Recorded explicitly because Principle
V makes any change here a breaking change to the user's file, and a reviewer should be able
to confirm at a glance that none happened.

| Entity | Where | Change |
|---|---|---|
| `Site` (`id`, `url`, `label`, `interval_secs`, `method_override`) | `model.rs`, `sites.json`, IPC | **None** — no field added, removed, renamed, or re-cased |
| `StatusEvent` (`id`, `state`, `checked_at`, `reason`) | `model.rs`, `site-status` event | **None** |
| `sites.json` | `~/Library/Application Support/com.clintparker.site-checker/` | **None** — same path, same bare-array shape, same snake_case keys |
| `autostart.initialized` marker | same directory | **None** — still written once on first run |
| IPC command surface (`list_sites`, `get_warning`, `add_site`, `update_site`, `delete_site`, `get_autostart`, `set_autostart`) | `commands.rs`, `api.ts` | **None** — no signature changes |

The only value that moves across the boundary differently is the *text* inside the existing
`String` warning payload. Its type, its transport (`get_warning` command and `store-warning`
event), and its nullability are all unchanged.

---

## Store load outcome — the state that gains a distinction

`store::load` returns `LoadOutcome { store, warning: Option<String> }`. It has three
outcomes today and three after the change; only the wording of two moves.

| # | Trigger | Sites | `warning` | Change |
|---|---|---|---|---|
| 1 | File absent (`ErrorKind::NotFound`) | empty | `None` | — |
| 2 | File present, cannot be opened (permissions, is-a-directory, other I/O) | empty | `Some(open message)` | **reworded** |
| 3 | File present and readable, contents not a valid site list | empty | `Some(damaged message)` | **reworded** |

Invariants that hold before and after (FR-008, Principle II):

- `load` never returns an error and never panics — every branch yields a usable `Store`.
- Outcomes 2 and 3 both start with an **empty** list; the app remains fully usable.
- In outcome 3 the file on disk is **not** touched. Nothing writes until the user's next
  save, so a damaged file stays recoverable by hand.
- Outcome 2 does not describe the file as damaged; outcome 3 does not describe it as
  unopenable. This is the new property, and it is what the added test pins.

Exact strings: [contracts/warning-messages.md](./contracts/warning-messages.md).

---

## Project identity values

Not runtime data, but the values this feature edits — and the ones FR-004 pins. "Shipped"
means the value appears in the built `.app`; "internal" means it appears only in source,
build output, and dependency records.

| Field | Location | Before | After | Visibility |
|---|---|---|---|---|
| Cargo package name | `src-tauri/Cargo.toml` `[package] name` | `tauri-app` | `site-checker` | internal |
| Cargo description | `src-tauri/Cargo.toml` | `A Tauri App` | describes Site Checker | internal |
| Cargo authors | `src-tauri/Cargo.toml` | `["you"]` | `["Clint Parker <me@clintparker.com>"]` | internal |
| Cargo lib name | `src-tauri/Cargo.toml` `[lib] name` | `tauri_app_lib` | `site_checker_lib` | internal |
| Lib call site | `src-tauri/src/main.rs:5` | `tauri_app_lib::run()` | `site_checker_lib::run()` | internal |
| npm package name | `package.json` `name` | `tauri-app` | `site-checker` | internal |
| **Main binary name** | `src-tauri/tauri.conf.json` `mainBinaryName` | *(absent — defaults to Cargo package name)* | `tauri-app` **(pinned)** | **shipped** |

The last row is the pin from research R1: without it, renaming the Cargo package silently
changes `CFBundleExecutable` and orphans the installed LaunchAgent.

### Pinned — must not change

| Value | Source | Must remain |
|---|---|---|
| `productName` | `tauri.conf.json` | `Site Checker` |
| `identifier` | `tauri.conf.json` | `com.clintparker.site-checker` |
| Window title | `tauri.conf.json` `app.windows[0].title` | `Site Checker` |
| `CFBundleExecutable` | derived (now pinned via `mainBinaryName`) | `tauri-app` |
| Config dir | derived from `identifier` | `…/com.clintparker.site-checker/` |

The last row is the reason the identifier is load-bearing rather than cosmetic: it is what
`app_config_dir()` resolves, so changing it would strand the user's existing `sites.json`.

---

## Deletions

| Path | Referenced by | Action |
|---|---|---|
| `src/assets/tauri.svg` | nothing (verified, R7) | delete |
| `src/assets/typescript.svg` | nothing | delete |
| `src/assets/vite.svg` | nothing | delete |
| `src/assets/` | — | remove once empty |
| `"opener:default"` | `src-tauri/capabilities/default.json` | remove |
| `tauri-plugin-opener = "2"` | `src-tauri/Cargo.toml` | remove |
| `"@tauri-apps/plugin-opener": "^2"` | `package.json` | remove |
| resolved opener records | `Cargo.lock`, `pnpm-lock.yaml` | regenerate |
