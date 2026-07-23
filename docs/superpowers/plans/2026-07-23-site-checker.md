# Site Checker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a macOS desktop app that shows, at a glance, whether each of the user's websites is up and how long ago that was last confirmed.

**Architecture:** A Tauri v2 app. All networking, scheduling, classification, and persistence live in a Rust backend; the webview UI is a thin view that calls Rust via `invoke` and receives results via Tauri events. The Rust side is split so that the two pieces worth testing — URL/interval validation and the HTTP classifier — are pure functions with no Tauri or filesystem dependency.

**Tech Stack:** Rust (stable ≥ 1.88), Tauri 2.11, reqwest 0.13, tokio 1, serde/serde_json, uuid, url, rand; TypeScript + Vite (vanilla, no UI framework), vitest for one pure formatter.

---

## Global Constraints

These apply to every task. Values are copied verbatim from the spec.

- **"Up" means HTTP status `200..=399`.** Anything else is Down.
- **Request method is HEAD**, falling back to **GET** on `405` or `501` only. The fallback is persisted per site as `method_override = "GET"`.
- **Check timeout: 10 seconds.** Redirects followed, **max 10 hops**.
- **Default interval: 60 seconds. Interval floor: 10 seconds** — lower values clamp up to 10.
- **Be a polite client.** No cache-busting query strings. No custom/unusual headers beyond a browser-like `User-Agent`. No interval below the floor.
- **No local HTTP cache.** (reqwest has no response cache — this constraint requires no code, only that nobody adds one.)
- **Check results are never written to disk.** Live status is in-memory only; every site starts Pending on launch.
- **Persistence file:** `~/Library/Application Support/com.clintparker.site-checker/sites.json`, snake_case keys, exactly the shape in the spec.
- **App identifier:** `com.clintparker.site-checker`.
- **Closing the window quits the app.**
- **Launch at login is on by default,** registered on first run.
- **No history, no alerting, no notifications, no "check now" button, no menu-bar icon, no auth headers, no per-URL expected status.** These are explicitly out of scope for v1.

### Naming convention gotcha (applies to Tasks 6–9)

Tauri v2 converts **command arguments** from camelCase (JS) to snake_case (Rust) automatically, but does **not** convert **serialized struct fields**. So:

- Calling a command: `invoke("add_site", { url, label, intervalSecs })` → `fn add_site(url: String, label: Option<String>, interval_secs: u64)`.
- Reading a payload: `Site` and `StatusEvent` fields arrive in JS as `interval_secs`, `method_override`, `checked_at` — **snake_case**. Do not add `#[serde(rename_all = "camelCase")]`; `Site` must serialize to snake_case to match the spec's `sites.json` format, and `StatusEvent` stays consistent with it.

---

## Toolchain situation (read before Task 1)

The spec says "Rust is not currently installed." That was very nearly right, but the actual state on this machine is more specific, and the fix is different from a plain `rustup` install:

- `~/.tool-versions` pins `rust 1.77.0`, and `~/.config/fish/config.fish:66-69` puts `~/.asdf/installs/rust/1.77.0/bin` on `PATH`.
- That directory holds **rustup shims**, which resolve toolchains from `RUSTUP_HOME` (default `~/.rustup`).
- `~/.rustup/settings.toml` exists but contains no `default_toolchain`, and `~/.rustup/toolchains/` is empty.
- **Result:** `rustc --version` and `cargo --version` both fail with `rustup could not choose a version of rustc to run`. Rust is effectively unusable right now.
- That same directory also holds **real, working binaries** — `trunk`, `cargo-nextest`, `cargo-llvm-cov`. **Do not delete this directory or remove it from `PATH`;** doing so would break `trunk`.

The fix in Task 1 is therefore additive, not destructive: install rustup in its standard location and put `~/.cargo/bin` **ahead** of the asdf directory on `PATH`. `trunk` keeps working; `cargo`/`rustc` start working.

**Version floors matter here.** The dependency with the highest MSRV is `httpmock` at **1.88.0** (`reqwest` needs 1.85, `uuid` needs 1.85, `tauri` needs 1.77.2). Installing current stable satisfies all of them. The pinned 1.77.0 does not — it cannot build this project.

---

## File Structure

```
site-checker/
├── rust-toolchain.toml          # pins the project to stable; immunity from the asdf pin
├── package.json                 # pnpm scripts, @tauri-apps/cli, vite, vitest
├── vite.config.ts
├── tsconfig.json
├── index.html                   # single window markup shell
├── src/                         # frontend — no networking of its own
│   ├── main.ts                  # bootstrap: load sites, subscribe to events, wire the form
│   ├── api.ts                   # the ONLY file that touches Tauri invoke/listen; types live here
│   ├── time.ts                  # formatSince() — pure, unit-tested
│   ├── time.test.ts
│   ├── render.ts                # builds the table DOM from a row model
│   └── styles.css
└── src-tauri/
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── capabilities/default.json
    └── src/
        ├── main.rs              # thin binary entry
        ├── lib.rs               # Tauri builder wiring, setup, window-close-quits, AppState
        ├── model.rs             # Site, Method, CheckState, StatusEvent, normalize_url, clamp_interval
        ├── store.rs             # sites.json load/save/add/update/delete
        ├── check.rs             # pure classifier + one HTTP check (no Tauri, no filesystem)
        ├── engine.rs            # one tokio task per site: jitter, loop, emit
        └── commands.rs          # #[tauri::command] handlers
```

**Why this split:** `check.rs` and `model.rs` hold everything the spec asks to be tested, and neither depends on Tauri or the filesystem — so their tests are plain `cargo test`. `engine.rs` needs an `AppHandle` and is therefore not unit-tested; keeping it thin (scheduling only, no classification logic) is what makes that acceptable. `store.rs` is tested against a temp dir.

---

## Task 1: Toolchain repair and project scaffold

**Files:**
- Modify: `~/.config/fish/config.fish` (add one block; do **not** remove the existing rust block at lines 66-69)
- Create: `rust-toolchain.toml`
- Create: everything under `src-tauri/` and `src/` (via scaffolder)
- Create: `.gitignore`

**Interfaces:**
- Consumes: nothing.
- Produces: a working `cargo`, a `site-checker` Tauri app that builds and opens a window, and a git repo with one commit.

- [ ] **Step 1: Confirm the broken state**

```bash
rustc --version; cargo --version
```

Expected: both fail with `error: rustup could not choose a version of rustc to run`. If instead they print a version ≥ 1.88.0, skip to Step 4.

- [ ] **Step 2: Install rustup in its standard location**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
```

Expected: ends with `Rust is installed now. Great!`. This writes `~/.cargo/` and populates `~/.rustup/toolchains/stable-aarch64-apple-darwin/`.

- [ ] **Step 3: Put `~/.cargo/bin` ahead of the asdf rust directory on PATH**

Add this block to `~/.config/fish/config.fish` immediately **after** the existing `# Trunk (Rust WASM bundler)` block that ends at line 69. Placement matters: `fish_add_path` prepends, so the block that runs last ends up first on `PATH`.

```fish
    # Rustup toolchain (must come after the trunk block so it wins on PATH)
    if test -d ~/.cargo/bin
        fish_add_path ~/.cargo/bin
    end
```

Then verify in a **new** shell:

```bash
rustc --version && cargo --version && which trunk
```

Expected: `rustc 1.9x.x` (must be ≥ 1.88.0), a matching cargo, and `trunk` still resolving to `/Users/clint/.asdf/installs/rust/1.77.0/bin/trunk`. If `trunk` is not found, the ordering is wrong — fix it before continuing.

- [ ] **Step 4: Initialize the git repo**

The project directory currently contains only `docs/`. It is not yet a git repository.

```bash
cd /Users/clint/src/clintcparker/site-checker
git init
git add docs
git commit -m "docs: add v1 design spec and implementation plan"
```

- [ ] **Step 5: Scaffold the Tauri app**

Run from the project root. The scaffolder wants an empty-ish directory; `docs/` and `.git/` do not bother it.

```bash
pnpm create tauri-app@latest . --template vanilla-ts --manager pnpm --identifier com.clintparker.site-checker
```

If the CLI prompts instead of accepting flags, answer: project name `site-checker`, frontend language **TypeScript**, package manager **pnpm**, UI template **Vanilla**, and **not** a mobile app.

```bash
pnpm install
```

pnpm 10 blocks postinstall scripts by default. If install warns about ignored build scripts, run `pnpm approve-builds` and approve `esbuild` and `@tauri-apps/cli`.

Accept whatever versions the scaffolder pins for Vite and TypeScript. The Rust dependency versions in Task 2 onward are the ones that matter.

- [ ] **Step 6: Pin the project's Rust toolchain**

Create `rust-toolchain.toml` at the project root. This makes the build independent of the machine's asdf pin.

```toml
[toolchain]
channel = "stable"
```

- [ ] **Step 7: Configure the window and identifier**

Replace `src-tauri/tauri.conf.json` with:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Site Checker",
  "version": "0.1.0",
  "identifier": "com.clintparker.site-checker",
  "build": {
    "beforeDevCommand": "pnpm dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "pnpm build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "Site Checker",
        "width": 720,
        "height": 480,
        "minWidth": 480,
        "minHeight": 320,
        "resizable": true
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": ["app", "dmg"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns"
    ]
  }
}
```

Leave `src-tauri/capabilities/default.json` exactly as scaffolded — its `core:default` permission set is what allows `invoke` and event listening. Autostart is driven entirely from Rust in Task 9, so no extra permissions are needed.

- [ ] **Step 8: Verify the app builds and runs**

```bash
pnpm tauri dev
```

Expected: Rust compiles (first build takes several minutes), and a 720×480 window titled "Site Checker" opens showing the scaffold's placeholder page. Quit it with Cmd-Q.

```bash
cd src-tauri && cargo test
```

Expected: `running 0 tests ... ok`. This confirms the test harness works before any tests exist.

- [ ] **Step 9: Commit**

```bash
cd /Users/clint/src/clintcparker/site-checker
git add -A
git commit -m "chore: scaffold Tauri v2 app with vanilla-ts frontend"
```

---

## Task 2: Domain model, URL normalization, interval clamping

**Files:**
- Create: `src-tauri/src/model.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod model;`)
- Modify: `src-tauri/Cargo.toml`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub struct Site { pub id: String, pub url: String, pub label: Option<String>, pub interval_secs: u64, pub method_override: Option<Method> }`
  - `pub enum Method { Get }` (serializes as the string `"GET"`)
  - `pub enum CheckState { Up, Down }` (serializes as `"up"` / `"down"`)
  - `pub struct StatusEvent { pub id: String, pub state: CheckState, pub checked_at: u64, pub reason: Option<String> }`
  - `pub fn normalize_url(input: &str) -> Result<String, String>`
  - `pub fn clamp_interval(secs: u64) -> u64`
  - `pub const MIN_INTERVAL_SECS: u64 = 10;`
  - `pub const DEFAULT_INTERVAL_SECS: u64 = 60;`

- [ ] **Step 1: Add dependencies**

In `src-tauri/Cargo.toml`, set the `[dependencies]` section to include these alongside whatever the scaffolder already put there (`tauri`, `serde`, `serde_json`):

```toml
[dependencies]
tauri = { version = "2.11", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
url = "2.5"
uuid = { version = "1.24", features = ["v4"] }

[dev-dependencies]
tempfile = "3"
```

```bash
cd src-tauri && cargo build
```

Expected: compiles clean.

- [ ] **Step 2: Write the failing tests**

Create `src-tauri/src/model.rs` containing only the test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_https_scheme_when_missing() {
        assert_eq!(normalize_url("example.com").unwrap(), "https://example.com");
    }

    #[test]
    fn preserves_an_explicit_scheme() {
        assert_eq!(normalize_url("http://example.com").unwrap(), "http://example.com");
        assert_eq!(
            normalize_url("https://api.foo.dev/health").unwrap(),
            "https://api.foo.dev/health"
        );
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(normalize_url("  example.com  ").unwrap(), "https://example.com");
    }

    #[test]
    fn rejects_empty_input() {
        assert!(normalize_url("   ").is_err());
    }

    #[test]
    fn rejects_unparseable_input() {
        assert!(normalize_url("http://").is_err());
        assert!(normalize_url("not a url at all").is_err());
    }

    #[test]
    fn rejects_non_http_schemes() {
        assert!(normalize_url("ftp://example.com").is_err());
        assert!(normalize_url("file:///etc/hosts").is_err());
    }

    #[test]
    fn clamps_intervals_below_the_floor() {
        assert_eq!(clamp_interval(0), 10);
        assert_eq!(clamp_interval(9), 10);
    }

    #[test]
    fn leaves_intervals_at_or_above_the_floor_alone() {
        assert_eq!(clamp_interval(10), 10);
        assert_eq!(clamp_interval(60), 60);
        assert_eq!(clamp_interval(3600), 3600);
    }

    #[test]
    fn method_override_serializes_as_uppercase_get() {
        let json = serde_json::to_string(&Method::Get).unwrap();
        assert_eq!(json, "\"GET\"");
    }

    #[test]
    fn site_omits_absent_optional_fields() {
        let site = Site {
            id: "abc".into(),
            url: "https://example.com".into(),
            label: None,
            interval_secs: 60,
            method_override: None,
        };
        let json = serde_json::to_string(&site).unwrap();
        assert_eq!(
            json,
            r#"{"id":"abc","url":"https://example.com","interval_secs":60,"method_override":null}"#
        );
    }

    #[test]
    fn check_state_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&CheckState::Up).unwrap(), "\"up\"");
        assert_eq!(serde_json::to_string(&CheckState::Down).unwrap(), "\"down\"");
    }
}
```

Add `mod model;` to the top of `src-tauri/src/lib.rs`.

- [ ] **Step 3: Run the tests to verify they fail**

```bash
cd src-tauri && cargo test model
```

Expected: compile errors — `cannot find function normalize_url in this scope`, `cannot find struct Site`, etc.

- [ ] **Step 4: Write the implementation**

Prepend this to `src-tauri/src/model.rs`, above the test module:

```rust
use serde::{Deserialize, Serialize};

/// Lowest interval we will ever schedule. Guardrail against hammering an endpoint.
pub const MIN_INTERVAL_SECS: u64 = 10;
/// What a new site gets when the user does not say otherwise.
pub const DEFAULT_INTERVAL_SECS: u64 = 60;

/// The only method we ever persist. HEAD is the default and needs no override;
/// this is written only once a server has told us HEAD is unwelcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Method {
    Get,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Site {
    pub id: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub interval_secs: u64,
    #[serde(default)]
    pub method_override: Option<Method>,
}

/// Emitted state. There is deliberately no `Pending` — that is a UI-only state
/// meaning "no event received yet this session".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckState {
    Up,
    Down,
}

/// Payload of the `site-status` event. Field names stay snake_case to match
/// `Site` and `sites.json`; the frontend reads them as-is.
#[derive(Debug, Clone, Serialize)]
pub struct StatusEvent {
    pub id: String,
    pub state: CheckState,
    /// Epoch milliseconds, taken when the check completed.
    pub checked_at: u64,
    pub reason: Option<String>,
}

/// Validate user input and add a scheme if one is missing.
///
/// Returns the user's own text (trimmed, scheme-prefixed) rather than the
/// re-serialized `Url`, so `example.com` yields `https://example.com` and not
/// `https://example.com/`.
pub fn normalize_url(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Enter a URL".to_string());
    }

    let candidate = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };

    let parsed = url::Url::parse(&candidate).map_err(|_| "Not a valid URL".to_string())?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Only http and https URLs are supported".to_string());
    }
    if parsed.host_str().is_none_or(|h| h.is_empty()) {
        return Err("URL is missing a host".to_string());
    }

    Ok(candidate)
}

/// Raise anything below the floor up to it. Never lowers a value.
pub fn clamp_interval(secs: u64) -> u64 {
    secs.max(MIN_INTERVAL_SECS)
}
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cd src-tauri && cargo test model
```

Expected: `test result: ok. 11 passed; 0 failed`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/model.rs src-tauri/src/lib.rs src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat: add site model with URL normalization and interval clamping"
```

---

## Task 3: JSON store

**Files:**
- Create: `src-tauri/src/store.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod store;`)

**Interfaces:**
- Consumes: `model::Site` from Task 2.
- Produces:
  - `pub struct Store { path: PathBuf, sites: Vec<Site> }`
  - `pub struct LoadOutcome { pub store: Store, pub warning: Option<String> }`
  - `pub fn load(path: PathBuf) -> LoadOutcome` — never fails, never panics
  - `impl Store`: `pub fn list(&self) -> Vec<Site>`, `pub fn add(&mut self, site: Site) -> Result<(), String>`, `pub fn update(&mut self, site: Site) -> Result<(), String>`, `pub fn delete(&mut self, id: &str) -> Result<(), String>`, `pub fn get(&self, id: &str) -> Option<Site>`
  - The `Result` in `add`/`update`/`delete` reports a **write** failure only. The in-memory change is applied regardless — the caller turns the error into a banner and keeps working.

- [ ] **Step 1: Write the failing tests**

Create `src-tauri/src/store.rs` containing only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Method;

    fn a_site(id: &str) -> Site {
        Site {
            id: id.to_string(),
            url: format!("https://{id}.example.com"),
            label: None,
            interval_secs: 60,
            method_override: None,
        }
    }

    #[test]
    fn missing_file_yields_an_empty_list_and_no_warning() {
        let dir = tempfile::tempdir().unwrap();
        let outcome = load(dir.path().join("sites.json"));
        assert!(outcome.store.list().is_empty());
        assert!(outcome.warning.is_none());
    }

    #[test]
    fn add_then_reload_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");

        let mut store = load(path.clone()).store;
        store.add(a_site("one")).unwrap();
        store.add(a_site("two")).unwrap();

        let reloaded = load(path).store;
        assert_eq!(reloaded.list().len(), 2);
        assert_eq!(reloaded.list()[0].url, "https://one.example.com");
    }

    #[test]
    fn update_replaces_the_matching_site_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");

        let mut store = load(path.clone()).store;
        store.add(a_site("one")).unwrap();
        store.add(a_site("two")).unwrap();

        let mut edited = a_site("one");
        edited.interval_secs = 300;
        edited.method_override = Some(Method::Get);
        store.update(edited).unwrap();

        let reloaded = load(path).store;
        let got = reloaded.get("one").unwrap();
        assert_eq!(got.interval_secs, 300);
        assert_eq!(got.method_override, Some(Method::Get));
        assert_eq!(reloaded.list().len(), 2);
        assert_eq!(reloaded.list()[0].id, "one", "order is preserved");
    }

    #[test]
    fn delete_removes_only_the_named_site() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");

        let mut store = load(path.clone()).store;
        store.add(a_site("one")).unwrap();
        store.add(a_site("two")).unwrap();
        store.delete("one").unwrap();

        let reloaded = load(path).store;
        assert_eq!(reloaded.list().len(), 1);
        assert_eq!(reloaded.list()[0].id, "two");
    }

    #[test]
    fn corrupt_file_yields_an_empty_list_a_warning_and_is_left_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");
        std::fs::write(&path, "{ this is not json").unwrap();

        let outcome = load(path.clone());
        assert!(outcome.store.list().is_empty());
        assert!(outcome.warning.is_some());

        let still_there = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            still_there, "{ this is not json",
            "the bad file must not be overwritten so it can be recovered by hand"
        );
    }

    #[test]
    fn writes_create_the_parent_directory() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("deeper").join("sites.json");

        let mut store = load(path.clone()).store;
        store.add(a_site("one")).unwrap();

        assert!(path.exists());
    }
}
```

Add `mod store;` to `src-tauri/src/lib.rs`.

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cd src-tauri && cargo test store
```

Expected: compile errors — `cannot find function load in this scope`.

- [ ] **Step 3: Write the implementation**

Prepend to `src-tauri/src/store.rs`:

```rust
use std::path::PathBuf;

use crate::model::Site;

pub struct Store {
    path: PathBuf,
    sites: Vec<Site>,
}

pub struct LoadOutcome {
    pub store: Store,
    /// Set when the file existed but could not be read or parsed. The caller
    /// surfaces this as a non-fatal banner.
    pub warning: Option<String>,
}

/// Read `sites.json`. Never fails: a missing file is an empty list, and an
/// unreadable or corrupt file is an empty list plus a warning. In the corrupt
/// case the file on disk is deliberately left untouched — the next write will
/// overwrite it, but nothing before that does, so it can be recovered by hand.
pub fn load(path: PathBuf) -> LoadOutcome {
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return LoadOutcome {
                store: Store { path, sites: Vec::new() },
                warning: None,
            };
        }
        Err(e) => {
            return LoadOutcome {
                store: Store { path, sites: Vec::new() },
                warning: Some(format!("Could not read sites.json ({e}). Starting empty.")),
            };
        }
    };

    match serde_json::from_str::<Vec<Site>>(&raw) {
        Ok(sites) => LoadOutcome {
            store: Store { path, sites },
            warning: None,
        },
        Err(e) => LoadOutcome {
            store: Store { path, sites: Vec::new() },
            warning: Some(format!(
                "sites.json could not be read ({e}). Starting with an empty list; \
                 the existing file has been left alone."
            )),
        },
    }
}

impl Store {
    pub fn list(&self) -> Vec<Site> {
        self.sites.clone()
    }

    pub fn get(&self, id: &str) -> Option<Site> {
        self.sites.iter().find(|s| s.id == id).cloned()
    }

    pub fn add(&mut self, site: Site) -> Result<(), String> {
        self.sites.push(site);
        self.save()
    }

    /// Replace the site with a matching id, preserving list order. A site that
    /// is not present is a no-op, still followed by a save.
    pub fn update(&mut self, site: Site) -> Result<(), String> {
        if let Some(slot) = self.sites.iter_mut().find(|s| s.id == site.id) {
            *slot = site;
        }
        self.save()
    }

    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        self.sites.retain(|s| s.id != id);
        self.save()
    }

    fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(&self.sites)
            .map_err(|e| format!("Could not serialize sites: {e}"))?;
        std::fs::write(&self.path, json)
            .map_err(|e| format!("Could not write sites.json: {e}"))
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd src-tauri && cargo test store
```

Expected: `test result: ok. 6 passed; 0 failed`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/store.rs src-tauri/src/lib.rs
git commit -m "feat: add JSON store with corrupt-file recovery"
```

---

## Task 4: HTTP check and classifier

This is the core of the product. Everything here is pure or takes an explicit URL — no Tauri, no filesystem.

**Files:**
- Create: `src-tauri/src/check.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod check;`)
- Modify: `src-tauri/Cargo.toml`

**Interfaces:**
- Consumes: `model::{CheckState, Method}` from Task 2.
- Produces:
  - `pub struct CheckOutcome { pub state: CheckState, pub reason: Option<String>, pub used_get_fallback: bool }`
  - `pub fn build_client() -> reqwest::Client`
  - `pub fn classify_status(status: u16) -> CheckState`
  - `pub async fn check_url(client: &reqwest::Client, url: &str, method_override: Option<Method>) -> CheckOutcome`
  - `pub const USER_AGENT: &str`
  - `used_get_fallback` is `true` only when this call discovered the fallback (HEAD returned 405/501 and GET was then used). It is `false` when the site already had `method_override = Some(Method::Get)` — there is nothing new to persist in that case.

- [ ] **Step 1: Add dependencies**

Add to `src-tauri/Cargo.toml`:

```toml
[dependencies]
reqwest = { version = "0.13", default-features = false, features = ["default-tls", "http2", "system-proxy", "charset"] }
tokio = { version = "1", features = ["time", "rt", "macros"] }

[dev-dependencies]
httpmock = "0.8"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Note: reqwest 0.13's `default-tls` feature resolves to **rustls**, so there is no OpenSSL or native-tls build dependency to satisfy.

```bash
cd src-tauri && cargo build
```

Expected: compiles clean.

- [ ] **Step 2: Write the failing tests**

Create `src-tauri/src/check.rs` containing only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::{Method::GET, Method::HEAD, MockServer};

    #[test]
    fn classifies_2xx_and_3xx_as_up() {
        assert_eq!(classify_status(200), CheckState::Up);
        assert_eq!(classify_status(204), CheckState::Up);
        assert_eq!(classify_status(301), CheckState::Up);
        assert_eq!(classify_status(399), CheckState::Up);
    }

    #[test]
    fn classifies_everything_else_as_down() {
        assert_eq!(classify_status(400), CheckState::Down);
        assert_eq!(classify_status(404), CheckState::Down);
        assert_eq!(classify_status(429), CheckState::Down);
        assert_eq!(classify_status(500), CheckState::Down);
        assert_eq!(classify_status(503), CheckState::Down);
        assert_eq!(classify_status(199), CheckState::Down);
    }

    #[tokio::test]
    async fn a_200_on_head_is_up() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(HEAD).path("/");
                then.status(200);
            })
            .await;

        let outcome = check_url(&build_client(), &server.url("/"), None).await;
        assert_eq!(outcome.state, CheckState::Up);
        assert_eq!(outcome.reason, None);
        assert!(!outcome.used_get_fallback);
    }

    #[tokio::test]
    async fn a_404_is_down_with_the_status_as_the_reason() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(HEAD).path("/");
                then.status(404);
            })
            .await;

        let outcome = check_url(&build_client(), &server.url("/"), None).await;
        assert_eq!(outcome.state, CheckState::Down);
        assert_eq!(outcome.reason.as_deref(), Some("HTTP 404"));
    }

    #[tokio::test]
    async fn a_followed_redirect_resolves_to_the_final_status() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(HEAD).path("/old");
                then.status(301).header("location", "/new");
            })
            .await;
        server
            .mock_async(|when, then| {
                when.method(HEAD).path("/new");
                then.status(200);
            })
            .await;

        let outcome = check_url(&build_client(), &server.url("/old"), None).await;
        assert_eq!(outcome.state, CheckState::Up);
    }

    #[tokio::test]
    async fn head_405_falls_back_to_get_and_reports_the_fallback() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(HEAD).path("/");
                then.status(405);
            })
            .await;
        let get_mock = server
            .mock_async(|when, then| {
                when.method(GET).path("/");
                then.status(200);
            })
            .await;

        let outcome = check_url(&build_client(), &server.url("/"), None).await;
        assert_eq!(outcome.state, CheckState::Up);
        assert!(
            outcome.used_get_fallback,
            "the caller needs this to persist method_override = GET"
        );
        get_mock.assert_async().await;
    }

    #[tokio::test]
    async fn head_501_also_falls_back_to_get() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(HEAD).path("/");
                then.status(501);
            })
            .await;
        server
            .mock_async(|when, then| {
                when.method(GET).path("/");
                then.status(200);
            })
            .await;

        let outcome = check_url(&build_client(), &server.url("/"), None).await;
        assert_eq!(outcome.state, CheckState::Up);
        assert!(outcome.used_get_fallback);
    }

    #[tokio::test]
    async fn a_known_get_only_site_skips_head_entirely() {
        let server = MockServer::start_async().await;
        let head_mock = server
            .mock_async(|when, then| {
                when.method(HEAD).path("/");
                then.status(405);
            })
            .await;
        server
            .mock_async(|when, then| {
                when.method(GET).path("/");
                then.status(200);
            })
            .await;

        let outcome = check_url(&build_client(), &server.url("/"), Some(Method::Get)).await;
        assert_eq!(outcome.state, CheckState::Up);
        assert!(
            !outcome.used_get_fallback,
            "already persisted; nothing new to write"
        );
        head_mock.assert_hits_async(0).await;
    }

    #[tokio::test]
    async fn a_get_fallback_that_also_fails_is_down() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(HEAD).path("/");
                then.status(405);
            })
            .await;
        server
            .mock_async(|when, then| {
                when.method(GET).path("/");
                then.status(500);
            })
            .await;

        let outcome = check_url(&build_client(), &server.url("/"), None).await;
        assert_eq!(outcome.state, CheckState::Down);
        assert_eq!(outcome.reason.as_deref(), Some("HTTP 500"));
    }

    #[tokio::test]
    async fn a_connection_failure_is_down_with_a_short_reason() {
        // Port 1 on loopback: nothing listens there, so this refuses immediately.
        let outcome = check_url(&build_client(), "http://127.0.0.1:1/", None).await;
        assert_eq!(outcome.state, CheckState::Down);
        let reason = outcome.reason.expect("a transport failure must explain itself");
        assert!(!reason.is_empty());
        assert!(
            reason.len() <= 80,
            "reason is shown in a hover tooltip, keep it short: {reason}"
        );
    }

    #[tokio::test]
    async fn an_unresolvable_host_is_down() {
        let outcome = check_url(
            &build_client(),
            "https://this-host-does-not-exist.invalid/",
            None,
        )
        .await;
        assert_eq!(outcome.state, CheckState::Down);
        assert!(outcome.reason.is_some());
    }
}
```

Add `mod check;` to `src-tauri/src/lib.rs`.

- [ ] **Step 3: Run the tests to verify they fail**

```bash
cd src-tauri && cargo test check
```

Expected: compile errors — `cannot find function build_client in this scope`.

- [ ] **Step 4: Write the implementation**

Prepend to `src-tauri/src/check.rs`:

```rust
use crate::model::{CheckState, Method};

/// A stock Safari string. The spec's "be a polite client" constraint means
/// looking like an ordinary browser rather than announcing an unknown tool that
/// a WAF might rate-limit or block.
pub const USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 \
     (KHTML, like Gecko) Version/17.0 Safari/605.1.15";

const TIMEOUT_SECS: u64 = 10;
const MAX_REDIRECTS: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOutcome {
    pub state: CheckState,
    /// Short, tooltip-sized explanation. `None` when the site is Up.
    pub reason: Option<String>,
    /// True only when *this* check discovered that HEAD is unsupported. The
    /// caller persists `method_override = GET` when it sees this.
    pub used_get_fallback: bool,
}

/// One client is shared by every site. reqwest keeps no response cache, so the
/// spec's "local HTTP cache disabled" requirement is satisfied by construction —
/// do not add a caching layer.
pub fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
        .build()
        .expect("the HTTP client has no fallible configuration")
}

/// "Is my app working", not "is the box alive": 200-399 is Up.
///
/// A *final* 3xx only appears when the redirect limit is hit; per the spec that
/// still counts as Up.
pub fn classify_status(status: u16) -> CheckState {
    if (200..=399).contains(&status) {
        CheckState::Up
    } else {
        CheckState::Down
    }
}

/// Run one check. Sends HEAD unless the site is already known to need GET.
/// On a 405 or 501 from HEAD, retries once with GET against the same URL.
pub async fn check_url(
    client: &reqwest::Client,
    url: &str,
    method_override: Option<Method>,
) -> CheckOutcome {
    if method_override == Some(Method::Get) {
        return match client.get(url).send().await {
            Ok(response) => outcome_from_status(response.status().as_u16(), false),
            Err(e) => transport_failure(&e, false),
        };
    }

    let head_status = match client.head(url).send().await {
        Ok(response) => response.status().as_u16(),
        Err(e) => return transport_failure(&e, false),
    };

    if head_status != 405 && head_status != 501 {
        return outcome_from_status(head_status, false);
    }

    // This server is HEAD-hostile. Retry with GET and tell the caller to
    // remember it so future checks go straight to GET.
    match client.get(url).send().await {
        Ok(response) => outcome_from_status(response.status().as_u16(), true),
        Err(e) => transport_failure(&e, true),
    }
}

fn outcome_from_status(status: u16, used_get_fallback: bool) -> CheckOutcome {
    let state = classify_status(status);
    CheckOutcome {
        reason: match state {
            CheckState::Up => None,
            CheckState::Down => Some(format!("HTTP {status}")),
        },
        state,
        used_get_fallback,
    }
}

/// reqwest's `Display` chains every source, which is far too long for a hover
/// tooltip. Collapse to a category instead.
fn transport_failure(error: &reqwest::Error, used_get_fallback: bool) -> CheckOutcome {
    let reason = if error.is_timeout() {
        "Timed out after 10s".to_string()
    } else if error.is_connect() {
        "Could not connect".to_string()
    } else if error.is_redirect() {
        "Too many redirects".to_string()
    } else if error.is_body() || error.is_decode() {
        "Bad response".to_string()
    } else {
        "Request failed".to_string()
    };

    CheckOutcome {
        state: CheckState::Down,
        reason: Some(reason),
        used_get_fallback,
    }
}
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cd src-tauri && cargo test check
```

Expected: `test result: ok. 11 passed; 0 failed`. The DNS test needs network access; `.invalid` is a reserved TLD that never resolves, so it fails fast rather than hanging.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/check.rs src-tauri/src/lib.rs src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat: add HTTP checker with HEAD-to-GET fallback and status classifier"
```

---

## Task 5: Scheduling engine

Deliberately thin: scheduling only, no classification logic. It needs an `AppHandle` to emit, so it is not unit-tested — which is acceptable precisely because all the judgment lives in Task 4.

**Files:**
- Create: `src-tauri/src/engine.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod engine;`)
- Modify: `src-tauri/Cargo.toml`

**Interfaces:**
- Consumes: `check::{build_client, check_url}`, `model::{Method, Site, StatusEvent, CheckState}`, `store::Store`.
- Produces:
  - `pub struct Engine` — a handle around an `Arc<Inner>`; cheap to hold, all methods take `&self`
  - `pub fn new(app: tauri::AppHandle, store: std::sync::Arc<std::sync::Mutex<Store>>) -> Engine`
  - `pub fn start(&self, site: Site)`
  - `pub fn stop(&self, id: &str)`
  - `pub fn reschedule(&self, site: Site)` — stop then start
  - `pub fn start_all(&self, sites: Vec<Site>)`
  - Emits the `site-status` event with a `StatusEvent` payload.

**Test coverage note.** Task 4 proves the checker reports `used_get_fallback`, and Task 3 proves the store round-trips `method_override = Some(Method::Get)`. The seam between them — `persist_get_fallback` below — is the one piece of the spec's HEAD→GET test that no automated test covers, because writing it would require a live `AppHandle`. It is verified by hand in Task 10 instead. Keep this function trivial so that stays a fair trade.

- [ ] **Step 1: Add the rand dependency**

Add to `src-tauri/Cargo.toml` under `[dependencies]`:

```toml
rand = "0.10"
```

- [ ] **Step 2: Write the implementation**

There is no test step in this task — see the note above. Create `src-tauri/src/engine.rs`:

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};

use crate::check::{build_client, check_url};
use crate::model::{Method, Site, StatusEvent};
use crate::store::Store;

/// Upper bound on the startup offset. Keeps N sites on a shared interval from
/// all firing on the same second without delaying the first result much.
const MAX_JITTER_SECS: u64 = 10;

/// A cheap handle. The spawned per-site tasks need shared ownership of the
/// engine's guts, so those live behind an `Arc` and every public method takes
/// a plain `&self`.
pub struct Engine {
    inner: Arc<Inner>,
}

struct Inner {
    app: AppHandle,
    client: reqwest::Client,
    store: Arc<Mutex<Store>>,
    tasks: Mutex<HashMap<String, tauri::async_runtime::JoinHandle<()>>>,
}

impl Engine {
    pub fn new(app: AppHandle, store: Arc<Mutex<Store>>) -> Self {
        Self {
            inner: Arc::new(Inner {
                app,
                client: build_client(),
                store,
                tasks: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub fn start_all(&self, sites: Vec<Site>) {
        for site in sites {
            self.start(site);
        }
    }

    /// Spawn the recurring check for one site. Replaces any task already
    /// running for that id.
    pub fn start(&self, site: Site) {
        self.stop(&site.id);

        let inner = Arc::clone(&self.inner);
        let id = site.id.clone();
        let handle = tauri::async_runtime::spawn(async move {
            inner.run_site(site).await;
        });

        self.inner.tasks.lock().unwrap().insert(id, handle);
    }

    /// Aborts the task. A check already in flight may still emit one last
    /// event; the UI ignores events for sites it no longer has.
    pub fn stop(&self, id: &str) {
        if let Some(handle) = self.inner.tasks.lock().unwrap().remove(id) {
            handle.abort();
        }
    }

    pub fn reschedule(&self, site: Site) {
        self.start(site);
    }
}

impl Inner {
    async fn run_site(&self, site: Site) {
        // The offset is applied once, before the first check. Because the loop
        // sleeps a full interval after each check, every subsequent check keeps
        // the same offset.
        let jitter_ceiling_ms = site.interval_secs.min(MAX_JITTER_SECS) * 1000;
        let jitter_ms = rand::random_range(0..=jitter_ceiling_ms);
        tokio::time::sleep(Duration::from_millis(jitter_ms)).await;

        let interval = Duration::from_secs(site.interval_secs);
        let mut method_override = site.method_override;

        loop {
            let outcome = check_url(&self.client, &site.url, method_override).await;

            if outcome.used_get_fallback {
                method_override = Some(Method::Get);
                self.persist_get_fallback(&site.id);
            }

            let event = StatusEvent {
                id: site.id.clone(),
                state: outcome.state,
                checked_at: now_millis(),
                reason: outcome.reason,
            };
            let _ = self.app.emit("site-status", event);

            tokio::time::sleep(interval).await;
        }
    }

    /// Record that this site needs GET so future launches skip the HEAD probe.
    /// The lock is taken and released synchronously — never held across an
    /// `.await`, which would make this task non-`Send`.
    fn persist_get_fallback(&self, id: &str) {
        let mut store = self.store.lock().unwrap();
        if let Some(mut site) = store.get(id) {
            site.method_override = Some(Method::Get);
            // A write failure here is not worth a banner: the in-memory value
            // holds for this session and the next check re-discovers it.
            let _ = store.update(site);
        }
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
```

- [ ] **Step 3: Verify it compiles**

Add `mod engine;` to `src-tauri/src/lib.rs`, then:

```bash
cd src-tauri && cargo build
```

Expected: compiles clean. If you get `future cannot be sent between threads safely`, a `MutexGuard` is being held across an `.await` — check `persist_get_fallback`.

- [ ] **Step 4: Run the full test suite to confirm nothing regressed**

```bash
cd src-tauri && cargo test
```

Expected: all 28 tests from Tasks 2-4 still pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/engine.rs src-tauri/src/lib.rs src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat: add per-site scheduling engine with startup jitter"
```

---

## Task 6: Tauri commands and app wiring

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs` (full rewrite)
- Modify: `src-tauri/src/main.rs`

**Interfaces:**
- Consumes: everything from Tasks 2-5.
- Produces the command surface the frontend uses in Tasks 7-9:
  - `list_sites() -> Vec<Site>`
  - `add_site(url: String, label: Option<String>, interval_secs: u64) -> Result<Site, String>`
  - `update_site(id: String, url: String, label: Option<String>, interval_secs: u64) -> Result<Site, String>`
  - `delete_site(id: String) -> Result<(), String>`
  - `get_warning() -> Option<String>`
- Produces the events: `site-status` (payload `StatusEvent`) and `store-warning` (payload `{ message: String }`).
- Produces `pub struct AppState { pub store: Arc<Mutex<Store>>, pub engine: Engine, pub warning: Mutex<Option<String>> }`.

- [ ] **Step 1: Write the commands**

Create `src-tauri/src/commands.rs`:

```rust
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{Emitter, State};

use crate::engine::Engine;
use crate::model::{clamp_interval, normalize_url, Site};
use crate::store::Store;

pub struct AppState {
    pub store: Arc<Mutex<Store>>,
    pub engine: Engine,
    /// Set at startup when sites.json could not be read. Read once by the UI.
    pub warning: Mutex<Option<String>>,
}

#[derive(Clone, Serialize)]
struct StoreWarning {
    message: String,
}

/// A write failure must not lose the user's edit. The in-memory change stands
/// and the UI shows a banner.
fn warn_on_write_failure(app: &tauri::AppHandle, result: Result<(), String>) {
    if let Err(message) = result {
        let _ = app.emit("store-warning", StoreWarning { message });
    }
}

fn empty_to_none(label: Option<String>) -> Option<String> {
    label
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
}

#[tauri::command]
pub fn list_sites(state: State<'_, AppState>) -> Vec<Site> {
    state.store.lock().unwrap().list()
}

#[tauri::command]
pub fn get_warning(state: State<'_, AppState>) -> Option<String> {
    state.warning.lock().unwrap().take()
}

#[tauri::command]
pub fn add_site(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    url: String,
    label: Option<String>,
    interval_secs: u64,
) -> Result<Site, String> {
    // A bad URL is rejected outright and nothing is persisted.
    let url = normalize_url(&url)?;

    let site = Site {
        id: uuid::Uuid::new_v4().to_string(),
        url,
        label: empty_to_none(label),
        interval_secs: clamp_interval(interval_secs),
        method_override: None,
    };

    let write = state.store.lock().unwrap().add(site.clone());
    warn_on_write_failure(&app, write);

    state.engine.start(site.clone());
    Ok(site)
}

#[tauri::command]
pub fn update_site(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
    url: String,
    label: Option<String>,
    interval_secs: u64,
) -> Result<Site, String> {
    let url = normalize_url(&url)?;

    let existing = state
        .store
        .lock()
        .unwrap()
        .get(&id)
        .ok_or_else(|| "That site no longer exists".to_string())?;

    // A changed URL invalidates what we learned about HEAD support.
    let method_override = if existing.url == url {
        existing.method_override
    } else {
        None
    };

    let site = Site {
        id,
        url,
        label: empty_to_none(label),
        interval_secs: clamp_interval(interval_secs),
        method_override,
    };

    let write = state.store.lock().unwrap().update(site.clone());
    warn_on_write_failure(&app, write);

    // Only this site's timer restarts; every other site is untouched.
    state.engine.reschedule(site.clone());
    Ok(site)
}

#[tauri::command]
pub fn delete_site(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.engine.stop(&id);
    let write = state.store.lock().unwrap().delete(&id);
    warn_on_write_failure(&app, write);
    Ok(())
}
```

- [ ] **Step 2: Wire up the app**

Replace `src-tauri/src/lib.rs` entirely:

```rust
mod check;
mod commands;
mod engine;
mod model;
mod store;

use std::sync::{Arc, Mutex};

use tauri::Manager;

use commands::AppState;
use engine::Engine;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // ~/Library/Application Support/com.clintparker.site-checker/
            let path = app.path().app_config_dir()?.join("sites.json");
            let loaded = store::load(path);

            let store = Arc::new(Mutex::new(loaded.store));
            let engine = Engine::new(app.handle().clone(), Arc::clone(&store));

            let sites = store.lock().unwrap().list();
            engine.start_all(sites);

            app.manage(AppState {
                store,
                engine,
                warning: Mutex::new(loaded.warning),
            });

            Ok(())
        })
        // Closing the window quits the app. Without this, macOS keeps the
        // process alive with no window, which the spec explicitly rules out.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                window.app_handle().exit(0);
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_sites,
            commands::get_warning,
            commands::add_site,
            commands::update_site,
            commands::delete_site,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Confirm `src-tauri/src/main.rs` matches the scaffold's shape (adjust the crate name if the scaffolder chose a different one — check `[lib] name` in `Cargo.toml`):

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    site_checker_lib::run()
}
```

- [ ] **Step 3: Verify it compiles and the tests still pass**

```bash
cd src-tauri && cargo test
```

Expected: all tests pass, no warnings about unused items.

- [ ] **Step 4: Smoke-test the command surface**

```bash
pnpm tauri dev
```

In the webview devtools console (right-click → Inspect Element):

```js
const { invoke } = window.__TAURI__.core;
await invoke("list_sites");                                        // → []
await invoke("add_site", { url: "example.com", intervalSecs: 60 }) // → a Site
await invoke("list_sites");                                        // → [that Site]
await invoke("add_site", { url: "ftp://nope", intervalSecs: 60 })  // → rejects
```

If `window.__TAURI__` is undefined, set `"withGlobalTauri": true` under `app` in `tauri.conf.json` **for this step only**, and remove it afterwards — the frontend imports the API properly from Task 7 on.

Then confirm persistence:

```bash
cat ~/Library/Application\ Support/com.clintparker.site-checker/sites.json
```

Expected: a JSON array with one site, snake_case keys, `"url": "https://example.com"` (scheme added by normalization).

Also confirm the window-close behavior: close the window with the red button and check that the process is gone (`pgrep -f site-checker` returns nothing).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src
git commit -m "feat: expose site CRUD commands and wire up the Tauri app"
```

---

## Task 7: Site table with live relative time

**Files:**
- Create: `src/api.ts`, `src/time.ts`, `src/time.test.ts`, `src/render.ts`
- Modify: `src/main.ts`, `src/styles.css`, `index.html`
- Modify: `package.json`, create `vitest.config.ts`

**Interfaces:**
- Consumes: the commands and `site-status` event from Task 6.
- Produces:
  - `api.ts`: types `Site`, `StatusEvent`, `RowState`; functions `listSites`, `addSite`, `updateSite`, `deleteSite`, `getWarning`, `onSiteStatus`, `onStoreWarning`
  - `time.ts`: `formatSince(checkedAt: number | null, now: number): string`
  - `render.ts`: `renderTable(tbody: HTMLElement, rows: Row[], now: number): void` and `type Row = { site: Site; status: StatusEvent | null }`

- [ ] **Step 1: Add vitest**

The spec rules out UI end-to-end tests, and this plan honors that. `formatSince` is a different animal — a pure function with a table of rules and no DOM — so it gets a real test.

```bash
pnpm add -D vitest
```

Add to the `scripts` block in `package.json`:

```json
"test": "vitest run"
```

Create `vitest.config.ts` at the project root:

```ts
// Import from "vitest/config", not "vite" — only vitest's defineConfig knows
// about the `test` key.
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
  },
});
```

- [ ] **Step 2: Write the failing test**

Create `src/time.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { formatSince } from "./time";

const NOW = 1_700_000_000_000;

describe("formatSince", () => {
  it("shows an em dash when there is no check yet", () => {
    expect(formatSince(null, NOW)).toBe("—");
  });

  it("shows seconds under a minute", () => {
    expect(formatSince(NOW, NOW)).toBe("0s ago");
    expect(formatSince(NOW - 5_000, NOW)).toBe("5s ago");
    expect(formatSince(NOW - 59_000, NOW)).toBe("59s ago");
  });

  it("shows whole minutes from one minute up", () => {
    expect(formatSince(NOW - 60_000, NOW)).toBe("1m ago");
    expect(formatSince(NOW - 119_000, NOW)).toBe("1m ago");
    expect(formatSince(NOW - 180_000, NOW)).toBe("3m ago");
    expect(formatSince(NOW - 59 * 60_000, NOW)).toBe("59m ago");
  });

  it("shows whole hours from one hour up", () => {
    expect(formatSince(NOW - 3_600_000, NOW)).toBe("1h ago");
    expect(formatSince(NOW - 7_200_000, NOW)).toBe("2h ago");
  });

  it("never shows a negative age when the clock jitters", () => {
    expect(formatSince(NOW + 500, NOW)).toBe("0s ago");
  });
});
```

- [ ] **Step 3: Run the test to verify it fails**

```bash
pnpm test
```

Expected: `Failed to resolve import "./time"`.

- [ ] **Step 4: Write the formatter**

Create `src/time.ts`:

```ts
/**
 * Age of the last completed check, rendered for the "Last checked" column.
 * Both arguments are epoch milliseconds; `now` is passed in rather than read
 * so this stays pure and testable.
 */
export function formatSince(checkedAt: number | null, now: number): string {
  if (checkedAt === null) return "—";

  const seconds = Math.max(0, Math.floor((now - checkedAt) / 1000));
  if (seconds < 60) return `${seconds}s ago`;

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;

  return `${Math.floor(minutes / 60)}h ago`;
}
```

- [ ] **Step 5: Run the test to verify it passes**

```bash
pnpm test
```

Expected: `5 passed`.

- [ ] **Step 6: Write the API layer**

Create `src/api.ts`. This is the only frontend file that touches Tauri.

```ts
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Mirrors the Rust `Site`. Field names are snake_case — Tauri does not
 *  rename serialized struct fields, only command arguments. */
export interface Site {
  id: string;
  url: string;
  label?: string;
  interval_secs: number;
  method_override: "GET" | null;
}

/** Mirrors the Rust `StatusEvent`. */
export interface StatusEvent {
  id: string;
  state: "up" | "down";
  checked_at: number;
  reason: string | null;
}

export function listSites(): Promise<Site[]> {
  return invoke("list_sites");
}

export function getWarning(): Promise<string | null> {
  return invoke("get_warning");
}

// Command arguments ARE camelCase-converted by Tauri: intervalSecs → interval_secs.
export function addSite(
  url: string,
  label: string | null,
  intervalSecs: number,
): Promise<Site> {
  return invoke("add_site", { url, label, intervalSecs });
}

export function updateSite(
  id: string,
  url: string,
  label: string | null,
  intervalSecs: number,
): Promise<Site> {
  return invoke("update_site", { id, url, label, intervalSecs });
}

export function deleteSite(id: string): Promise<void> {
  return invoke("delete_site", { id });
}

export function onSiteStatus(
  handler: (event: StatusEvent) => void,
): Promise<UnlistenFn> {
  return listen<StatusEvent>("site-status", (e) => handler(e.payload));
}

export function onStoreWarning(
  handler: (message: string) => void,
): Promise<UnlistenFn> {
  return listen<{ message: string }>("store-warning", (e) =>
    handler(e.payload.message),
  );
}
```

- [ ] **Step 7: Write the table renderer**

Create `src/render.ts`:

```ts
import type { Site, StatusEvent } from "./api";
import { formatSince } from "./time";

export interface Row {
  site: Site;
  /** null until the first check of this session completes — the Pending state. */
  status: StatusEvent | null;
}

const DOT: Record<"up" | "down" | "pending", string> = {
  up: "🟢",
  down: "🔴",
  pending: "⚪",
};

const LABEL: Record<"up" | "down" | "pending", string> = {
  up: "Up",
  down: "Down",
  pending: "Pending",
};

export function renderTable(tbody: HTMLElement, rows: Row[], now: number): void {
  tbody.replaceChildren(...rows.map((row) => renderRow(row, now)));
}

function renderRow(row: Row, now: number): HTMLTableRowElement {
  const state = row.status?.state ?? "pending";

  const tr = document.createElement("tr");
  tr.dataset.id = row.site.id;

  const name = document.createElement("td");
  name.className = "site";
  name.append(text("span", "site-primary", row.site.label ?? row.site.url));
  if (row.site.label) {
    name.append(text("span", "site-secondary", row.site.url));
  }

  const status = document.createElement("td");
  status.className = "status";
  const dot = text("span", `dot dot-${state}`, DOT[state]);
  // The failure reason lives in a tooltip, per the spec.
  if (row.status?.reason) dot.title = row.status.reason;
  status.append(dot, text("span", "status-label", LABEL[state]));

  const since = document.createElement("td");
  since.className = "since";
  since.textContent = formatSince(row.status?.checked_at ?? null, now);

  const actions = document.createElement("td");
  actions.className = "actions";
  actions.append(
    button("edit", "Edit", row.site.id),
    button("delete", "Delete", row.site.id),
  );

  tr.append(name, status, since, actions);
  return tr;
}

function text(tag: string, className: string, content: string): HTMLElement {
  const el = document.createElement(tag);
  el.className = className;
  el.textContent = content;
  return el;
}

function button(action: string, label: string, id: string): HTMLButtonElement {
  const el = document.createElement("button");
  el.className = `row-action row-action-${action}`;
  el.dataset.action = action;
  el.dataset.id = id;
  el.textContent = label;
  return el;
}
```

- [ ] **Step 8: Write the markup shell**

Replace `index.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Site Checker</title>
    <link rel="stylesheet" href="/src/styles.css" />
  </head>
  <body>
    <div id="banner" class="banner" hidden></div>

    <table id="sites">
      <thead>
        <tr>
          <th>URL / label</th>
          <th>Status</th>
          <th>Last checked</th>
          <th></th>
        </tr>
      </thead>
      <tbody id="rows"></tbody>
    </table>

    <p id="empty" class="empty" hidden>No sites yet. Add one below.</p>

    <footer id="footer"></footer>

    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 9: Write the bootstrap**

Replace `src/main.ts`:

```ts
import {
  getWarning,
  listSites,
  onSiteStatus,
  onStoreWarning,
  type Site,
  type StatusEvent,
} from "./api";
import { renderTable, type Row } from "./render";

const sites = new Map<string, Site>();
const statuses = new Map<string, StatusEvent>();

const tbody = document.querySelector<HTMLElement>("#rows")!;
const empty = document.querySelector<HTMLElement>("#empty")!;
const banner = document.querySelector<HTMLElement>("#banner")!;

export function currentRows(): Row[] {
  return [...sites.values()].map((site) => ({
    site,
    status: statuses.get(site.id) ?? null,
  }));
}

export function repaint(): void {
  const rows = currentRows();
  renderTable(tbody, rows, Date.now());
  empty.hidden = rows.length > 0;
}

export function showBanner(message: string): void {
  banner.textContent = message;
  banner.hidden = false;
}

export function upsertSite(site: Site): void {
  sites.set(site.id, site);
  repaint();
}

export function removeSite(id: string): void {
  sites.delete(id);
  statuses.delete(id);
  repaint();
}

async function main(): Promise<void> {
  for (const site of await listSites()) sites.set(site.id, site);
  repaint();

  const startupWarning = await getWarning();
  if (startupWarning) showBanner(startupWarning);

  await onSiteStatus((event) => {
    statuses.set(event.id, event);
    repaint();
  });
  await onStoreWarning(showBanner);

  // The "time since" column ticks locally. It counts from the last completed
  // check, so this needs no backend chatter.
  setInterval(repaint, 1000);
}

main();
```

- [ ] **Step 10: Write the styles**

Replace `src/styles.css`:

```css
:root {
  font-family: -apple-system, BlinkMacSystemFont, "Helvetica Neue", sans-serif;
  font-size: 14px;
  color: #1c1c1e;
  background: #ffffff;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f2f2f7;
    background: #1c1c1e;
  }
}

body {
  margin: 0;
  padding: 16px;
  display: flex;
  flex-direction: column;
  min-height: 100vh;
  box-sizing: border-box;
}

table {
  width: 100%;
  border-collapse: collapse;
}

th {
  text-align: left;
  font-weight: 600;
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  opacity: 0.5;
  padding: 4px 8px;
}

td {
  padding: 8px;
  border-top: 1px solid rgb(128 128 128 / 0.2);
  vertical-align: middle;
}

.site-primary {
  display: block;
}

.site-secondary {
  display: block;
  font-size: 12px;
  opacity: 0.5;
}

.status {
  white-space: nowrap;
}

.dot {
  margin-right: 6px;
  cursor: default;
}

.since {
  white-space: nowrap;
  font-variant-numeric: tabular-nums;
  opacity: 0.7;
}

.actions {
  text-align: right;
  white-space: nowrap;
}

.row-action {
  background: none;
  border: none;
  color: inherit;
  opacity: 0.5;
  cursor: pointer;
  padding: 2px 6px;
  font-size: 12px;
}

.row-action:hover {
  opacity: 1;
}

.empty {
  opacity: 0.5;
  padding: 16px 8px;
}

.banner {
  background: #fff3cd;
  color: #664d03;
  border: 1px solid #ffe69c;
  border-radius: 6px;
  padding: 8px 12px;
  margin-bottom: 12px;
}

footer {
  margin-top: auto;
  padding-top: 16px;
}
```

- [ ] **Step 11: Verify in the app**

```bash
pnpm tauri dev
```

Expected: the site added during Task 6's smoke test appears as a row. Within ~10 seconds (jitter) its dot turns green and "Last checked" starts ticking `0s ago` → `1s ago` → …. Let it run past 60s and confirm the counter resets to `0s ago` when the next check lands.

- [ ] **Step 12: Commit**

```bash
git add src index.html package.json pnpm-lock.yaml vitest.config.ts
git commit -m "feat: render the site table with a live time-since column"
```

---

## Task 8: Add, edit, and delete

**Files:**
- Create: `src/form.ts`
- Modify: `src/main.ts`, `index.html`, `src/styles.css`

**Interfaces:**
- Consumes: `api.ts` and the exported helpers from `main.ts` (Task 7).
- Produces: `mountForm(...)` in `form.ts`, wiring the add/edit form and the per-row edit/delete buttons.

- [ ] **Step 1: Add the form markup**

In `index.html`, replace the `<footer id="footer"></footer>` line with:

```html
    <form id="site-form" class="site-form">
      <input type="hidden" id="site-id" />
      <input id="site-url" type="text" placeholder="example.com" required />
      <input id="site-label" type="text" placeholder="Label (optional)" />
      <input id="site-interval" type="number" min="10" step="1" value="60" />
      <button type="submit" id="site-submit">Add</button>
      <button type="button" id="site-cancel" hidden>Cancel</button>
      <p id="site-error" class="form-error" hidden></p>
    </form>

    <footer id="footer"></footer>
```

- [ ] **Step 2: Write the form controller**

Create `src/form.ts`:

```ts
import { addSite, deleteSite, updateSite, type Site } from "./api";

interface FormHooks {
  onSaved: (site: Site) => void;
  onDeleted: (id: string) => void;
  lookup: (id: string) => Site | undefined;
}

const MIN_INTERVAL = 10;
const DEFAULT_INTERVAL = 60;

export function mountForm(hooks: FormHooks): void {
  const form = document.querySelector<HTMLFormElement>("#site-form")!;
  const idField = document.querySelector<HTMLInputElement>("#site-id")!;
  const urlField = document.querySelector<HTMLInputElement>("#site-url")!;
  const labelField = document.querySelector<HTMLInputElement>("#site-label")!;
  const intervalField = document.querySelector<HTMLInputElement>("#site-interval")!;
  const submit = document.querySelector<HTMLButtonElement>("#site-submit")!;
  const cancel = document.querySelector<HTMLButtonElement>("#site-cancel")!;
  const error = document.querySelector<HTMLElement>("#site-error")!;
  const tbody = document.querySelector<HTMLElement>("#rows")!;

  function showError(message: string): void {
    error.textContent = message;
    error.hidden = false;
  }

  function clearError(): void {
    error.hidden = true;
  }

  function resetToAddMode(): void {
    idField.value = "";
    urlField.value = "";
    labelField.value = "";
    intervalField.value = String(DEFAULT_INTERVAL);
    submit.textContent = "Add";
    cancel.hidden = true;
    clearError();
  }

  function enterEditMode(site: Site): void {
    idField.value = site.id;
    urlField.value = site.url;
    labelField.value = site.label ?? "";
    intervalField.value = String(site.interval_secs);
    submit.textContent = "Save";
    cancel.hidden = false;
    clearError();
    urlField.focus();
  }

  form.addEventListener("submit", async (e) => {
    e.preventDefault();
    clearError();

    const url = urlField.value.trim();
    if (url === "") {
      showError("Enter a URL");
      return;
    }

    // The backend clamps too; doing it here keeps the field honest about what
    // was actually saved.
    const parsed = Number.parseInt(intervalField.value, 10);
    const interval = Number.isNaN(parsed)
      ? DEFAULT_INTERVAL
      : Math.max(MIN_INTERVAL, parsed);

    const label = labelField.value.trim() || null;
    const id = idField.value;

    try {
      const saved = id
        ? await updateSite(id, url, label, interval)
        : await addSite(url, label, interval);
      hooks.onSaved(saved);
      resetToAddMode();
    } catch (message) {
      // Rust `Err(String)` arrives here as the bare string.
      showError(String(message));
    }
  });

  cancel.addEventListener("click", resetToAddMode);

  tbody.addEventListener("click", async (e) => {
    const button = (e.target as HTMLElement).closest<HTMLElement>("[data-action]");
    if (!button) return;

    const id = button.dataset.id!;
    if (button.dataset.action === "edit") {
      const site = hooks.lookup(id);
      if (site) enterEditMode(site);
      return;
    }

    if (button.dataset.action === "delete") {
      await deleteSite(id);
      hooks.onDeleted(id);
      if (idField.value === id) resetToAddMode();
    }
  });

  resetToAddMode();
}
```

- [ ] **Step 3: Wire it into the bootstrap**

In `src/main.ts`, add the import at the top:

```ts
import { mountForm } from "./form";
```

and add this inside `main()`, immediately after the `repaint()` call that follows the initial `listSites()` loop:

```ts
  mountForm({
    onSaved: upsertSite,
    onDeleted: removeSite,
    lookup: (id) => sites.get(id),
  });
```

- [ ] **Step 4: Style the form**

Append to `src/styles.css`:

```css
.site-form {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  padding-top: 16px;
}

.site-form input[type="text"] {
  flex: 1 1 140px;
  min-width: 0;
}

.site-form input[type="number"] {
  width: 72px;
}

.site-form input,
.site-form button {
  font: inherit;
  padding: 6px 8px;
  border-radius: 6px;
  border: 1px solid rgb(128 128 128 / 0.4);
  background: transparent;
  color: inherit;
}

.site-form button {
  cursor: pointer;
}

.form-error {
  flex-basis: 100%;
  margin: 0;
  color: #c0392b;
  font-size: 12px;
}
```

- [ ] **Step 5: Verify by hand**

```bash
pnpm tauri dev
```

Walk through each of these:

1. Add `example.com` with no label and interval 60 → row appears immediately as Pending, turns green within ~10s, and `sites.json` gains an entry with `"url": "https://example.com"`.
2. Add `ftp://nope` → inline error `Only http and https URLs are supported`, no row added, `sites.json` unchanged.
3. Add with an empty URL → inline error `Enter a URL`.
4. Add with interval `3` → saved as `10` (check `sites.json` and the edit form).
5. Add a URL that 404s, e.g. `https://example.com/nope` → row goes red; hovering the red dot shows `HTTP 404`.
6. Add `https://this-host-does-not-exist.invalid` → row goes red; hovering shows `Could not connect` or similar.
7. Edit a row's interval to 15 → form pre-fills, Save updates it, that row's timer restarts and other rows keep ticking undisturbed.
8. Delete a row → it disappears and is gone from `sites.json`.
9. Quit and relaunch → all sites reload, every row starts Pending.

- [ ] **Step 6: Run the full test suite**

```bash
pnpm test && cd src-tauri && cargo test
```

Expected: all frontend and Rust tests pass.

- [ ] **Step 7: Commit**

```bash
git add src index.html
git commit -m "feat: add site add/edit/delete form with inline validation"
```

---

## Task 9: Launch at login

**Files:**
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/src/commands.rs`
- Modify: `src/api.ts`, `src/main.ts`, `index.html`, `src/styles.css`

**Interfaces:**
- Consumes: the `AppState` and command surface from Task 6.
- Produces: commands `get_autostart() -> Result<bool, String>` and `set_autostart(enabled: bool) -> Result<bool, String>` (returns the resulting state); `api.ts` gains `getAutostart` / `setAutostart`.

All autostart calls happen in Rust, so `capabilities/default.json` needs no new permissions.

- [ ] **Step 1: Add the plugin**

Add to `src-tauri/Cargo.toml` under `[dependencies]`:

```toml
tauri-plugin-autostart = "2.5"
```

- [ ] **Step 2: Register the plugin and enable on first run**

In `src-tauri/src/lib.rs`, add the import:

```rust
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
```

Register the plugin on the builder, immediately before `.setup(...)`:

```rust
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
```

Then, inside the `setup` closure, after `app.manage(AppState { .. })` and before `Ok(())`:

```rust
            // On by default, registered once. A marker file distinguishes
            // "first run" from "the user deliberately turned this off", so
            // unchecking the box sticks across restarts.
            let marker = app.path().app_config_dir()?.join("autostart.initialized");
            if !marker.exists() {
                let _ = app.autolaunch().enable();
                if let Some(parent) = marker.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&marker, b"");
            }
```

- [ ] **Step 3: Add the commands**

Append to `src-tauri/src/commands.rs`:

```rust
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub fn get_autostart(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

/// Returns the state actually in effect afterwards, so the checkbox can
/// correct itself if the OS refused the change.
#[tauri::command]
pub fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|e| e.to_string())?;
    manager.is_enabled().map_err(|e| e.to_string())
}
```

Register both in the `invoke_handler` list in `lib.rs`:

```rust
            commands::get_autostart,
            commands::set_autostart,
```

- [ ] **Step 4: Verify the backend compiles**

```bash
cd src-tauri && cargo test
```

Expected: compiles, all existing tests pass.

- [ ] **Step 5: Add the frontend binding**

Append to `src/api.ts`:

```ts
export function getAutostart(): Promise<boolean> {
  return invoke("get_autostart");
}

export function setAutostart(enabled: boolean): Promise<boolean> {
  return invoke("set_autostart", { enabled });
}
```

- [ ] **Step 6: Add the checkbox**

In `index.html`, replace `<footer id="footer"></footer>` with:

```html
    <footer id="footer">
      <label class="setting">
        <input type="checkbox" id="autostart" />
        Launch at login
      </label>
    </footer>
```

In `src/main.ts`, add `getAutostart, setAutostart` to the existing import from `./api`, and add this function above `main()`:

```ts
async function mountAutostart(): Promise<void> {
  const checkbox = document.querySelector<HTMLInputElement>("#autostart")!;

  try {
    checkbox.checked = await getAutostart();
  } catch (message) {
    showBanner(`Could not read the login item: ${String(message)}`);
    checkbox.disabled = true;
    return;
  }

  checkbox.addEventListener("change", async () => {
    try {
      // Trust what the OS reports rather than what was clicked.
      checkbox.checked = await setAutostart(checkbox.checked);
    } catch (message) {
      checkbox.checked = !checkbox.checked;
      showBanner(`Could not change the login item: ${String(message)}`);
    }
  });
}
```

Call it inside `main()`, right after the `mountForm({ ... })` call:

```ts
  await mountAutostart();
```

- [ ] **Step 7: Style the footer control**

Append to `src/styles.css`:

```css
.setting {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  opacity: 0.7;
  cursor: pointer;
}
```

- [ ] **Step 8: Verify by hand**

Autostart registers a LaunchAgent that points at the built binary, so verify this against a **built** app, not `tauri dev`:

```bash
pnpm tauri build
open src-tauri/target/release/bundle/macos/Site\ Checker.app
```

Then check:

1. The checkbox is **checked** on first launch.
2. `ls ~/Library/LaunchAgents/ | grep -i site-checker` → a plist exists.
3. Uncheck the box → the plist disappears.
4. Quit and relaunch → the checkbox is still **unchecked** (the marker file kept first-run logic from re-enabling it).
5. Re-check the box → the plist comes back.

```bash
ls ~/Library/Application\ Support/com.clintparker.site-checker/
```

Expected: `sites.json` and `autostart.initialized`.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src src index.html
git commit -m "feat: add launch-at-login toggle, enabled on first run"
```

---

## Task 10: Build, install, and verify against the spec

**Files:**
- Create: `README.md`
- Modify: `.gitignore` (if the scaffolder missed anything)

**Interfaces:**
- Consumes: the complete app.
- Produces: an installed `.app` and a README covering build and run.

- [ ] **Step 1: Confirm the whole suite is green**

```bash
pnpm test
cd src-tauri && cargo test && cargo clippy -- -D warnings
```

Expected: all tests pass and clippy is clean. Fix any warnings before continuing.

- [ ] **Step 2: Build the release bundle**

```bash
cd /Users/clint/src/clintcparker/site-checker
pnpm tauri build
```

Expected: `Finished` plus paths to `Site Checker.app` and a `.dmg` under `src-tauri/target/release/bundle/`.

Check the size claim from the spec's stack rationale:

```bash
du -sh src-tauri/target/release/bundle/macos/Site\ Checker.app
```

Expected: single-digit MB. A much larger number means something pulled in an unexpected dependency — worth investigating before shipping.

- [ ] **Step 3: Install and run it for real**

```bash
cp -R src-tauri/target/release/bundle/macos/Site\ Checker.app /Applications/
open /Applications/Site\ Checker.app
```

The app is unsigned, so Gatekeeper will block the first launch. Right-click the app in Finder → Open → Open to approve it.

Note: the login item registered in Task 9 points at wherever the app was when the box was ticked. After moving it to `/Applications`, toggle the checkbox off and on once so the LaunchAgent points at the installed copy.

- [ ] **Step 4: Verify each spec behavior against the installed app**

Walk the list and confirm every item:

- [ ] Add 3+ real sites at different intervals; all report Up.
- [ ] Watch for two full minutes: checks land staggered, not all on the same second (jitter working).
- [ ] A 404 URL shows red with `HTTP 404` on hover.
- [ ] An unreachable host shows red with a short reason on hover.
- [ ] "Last checked" ticks every second and resets when a check lands.
- [ ] Editing one site's interval leaves the other rows' counters undisturbed.
- [ ] Closing the window quits the process (`pgrep -f "Site Checker"` returns nothing).
- [ ] Relaunching reloads every site, all starting Pending.
- [ ] Corrupt-file handling: quit, run `echo "broken" > ~/Library/Application\ Support/com.clintparker.site-checker/sites.json`, relaunch → a banner appears, the list is empty, and the file still reads `broken`. Then restore it from the backup you took before this step.

Take that backup first:

```bash
cp ~/Library/Application\ Support/com.clintparker.site-checker/sites.json /tmp/sites.json.bak
```

- [ ] **Step 5: Write the README**

Create `README.md`:

````markdown
# Site Checker

A personal status dashboard for the websites and endpoints I care about. It
answers one question: is this thing up, and how long ago did we last confirm
that?

Not a monitoring service — no alerting, no history, no SLA math. One Mac, one
person.

## Requirements

- macOS
- Rust stable (≥ 1.88) via [rustup](https://rustup.rs)
- Node + pnpm

## Develop

```bash
pnpm install
pnpm tauri dev
```

## Test

```bash
pnpm test                  # frontend: the relative-time formatter
cd src-tauri && cargo test  # backend: model, store, and HTTP classifier
```

## Build

```bash
pnpm tauri build
```

The bundle lands in `src-tauri/target/release/bundle/`.

## Where data lives

`~/Library/Application Support/com.clintparker.site-checker/sites.json`

Check results are never written to disk — every site starts Pending on launch.

## Design

See [docs/superpowers/specs/2026-07-23-site-checker-design.md](docs/superpowers/specs/2026-07-23-site-checker-design.md).
````

- [ ] **Step 6: Confirm build artifacts are ignored**

```bash
git status --short
```

Expected: no `target/`, `dist/`, or `node_modules/` entries. If any appear, add them to `.gitignore`.

- [ ] **Step 7: Commit**

```bash
git add README.md .gitignore
git commit -m "docs: add README with build and run instructions"
```

---

## Deferred to v2

Recorded here so they don't get quietly picked up mid-implementation. Each is listed as out of scope in the spec:

- Desktop notifications on up→down / down→up transitions
- Status history, sparklines, uptime percentages
- A manual "check now" button
- A menu-bar icon
- Continuing to run after the window is closed
- Per-URL expected-status configuration
- Auth headers / private endpoints
