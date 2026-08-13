# Phase 0 Research: Launch-at-login survives upgrades

**Feature**: `20260812-202608-autostart-launchagent-path` · **Spec**: [spec.md](./spec.md) · **Issue**: [#25](https://github.com/clintcparker/site-checker/issues/25)

All findings below were read out of the actual crate sources vendored in
`~/.cargo/registry` and the repository as it stands on this branch, not from memory. Every
NEEDS CLARIFICATION raised in the plan's Technical Context is resolved here.

---

## R1 — Why the recorded path is version-pinned today

**Question**: What writes `~/Library/LaunchAgents/Site Checker.plist`, and where does the path in
it come from?

**Finding**: `src-tauri/src/lib.rs:20-23` initialises `tauri-plugin-autostart` v2.5 with
`MacosLauncher::LaunchAgent`. The plugin
(`tauri-plugin-autostart-2.5.1/src/lib.rs`, `Builder::build`) constructs an `auto_launch::AutoLaunch`
during plugin setup and does this on macOS:

```rust
let exe_path = current_exe.canonicalize()?.display().to_string();
let parts: Vec<&str> = exe_path.split(".app/").collect();
let app_path = if parts.len() == 2 && matches!(self.macos_launcher, MacosLauncher::AppleScript) {
    format!("{}.app", parts.first().unwrap())
} else {
    exe_path
};
builder.set_app_path(&app_path);
```

Two things follow, and together they are the whole defect:

1. `current_exe().canonicalize()` **resolves every symlink**. Homebrew's stable
   `#{prefix}/opt/site-checker` is a symlink to the version-scoped keg, and the optional
   `/Applications/Site Checker.app` symlink points at the `opt` path. So no matter which of the
   three spellings the user launches, `canonicalize` collapses it to
   `/opt/homebrew/Cellar/site-checker/1.0.0/libexec/Site Checker.app/Contents/MacOS/site-checker`.
   The one path Homebrew guarantees will be deleted is the one that gets written.
2. The `.app`-trimming branch only fires for `AppleScript`, so under `LaunchAgent` the full
   executable path is recorded — which is correct for a LaunchAgent (`ProgramArguments` must name
   an executable, not a bundle) and is not something to change.

**Decision**: The path must be corrected *before* `AutoLaunch` is constructed. There is no
post-construction way in: `AutoLaunchManager` is a tuple struct over a private field, and its
`enable`/`disable`/`is_enabled` are the only public surface.

---

## D1 — Replace `tauri-plugin-autostart` with a direct `auto-launch` dependency

**Decision**: Drop `tauri-plugin-autostart` and depend on `auto-launch = "0.5"` directly,
constructing the `AutoLaunch` ourselves in a new `src-tauri/src/autostart.rs` with the path we
choose. Keep the Tauri commands `get_autostart` / `set_autostart` exactly as they are named and
shaped today; they call into our own managed state instead of the plugin's.

**Rationale**:

- The plugin's `Builder` exposes only `arg`/`args`/`app_name`/`macos_launcher`. There is **no**
  `set_app_path`, and the manager it registers wraps a private `AutoLaunch`. Passing the corrected
  path through the plugin is not possible without a fork.
- Nothing else in this repository uses the plugin. `src-tauri/capabilities/default.json` grants only
  `core:default` — the plugin's own JS commands (`plugin:autostart|enable`, `…|is_enabled`) were
  never permitted, and `package.json` has no `@tauri-apps/plugin-autostart`. The frontend reaches
  autostart solely through `invoke("get_autostart")` / `invoke("set_autostart")` in `src/api.ts`,
  which are our commands in `src-tauri/src/commands.rs:136-155`.
- Dependency count does not grow: `auto-launch` 0.5.0 is already in the tree as the plugin's own
  dependency; we remove one crate and promote another that was already being compiled.
- The plugin's remaining value over calling `auto-launch` directly is one line of app-name defaulting
  and one line of macOS launcher selection, both of which we now need to state explicitly anyway.

**Alternatives considered**:

| Alternative | Rejected because |
|---|---|
| Keep the plugin; rewrite the plist after `enable()` returns | Two writers for one file, and the second one has to re-derive the first one's filename and template. Every future plugin upgrade is a chance for the two to disagree silently. |
| Keep the plugin for `enable`/`disable`, add a second `AutoLaunch` for the corrected path | Strictly worse than the above: two live objects that disagree about where the app is, with the winner decided by call order. |
| Fork/vendor the plugin with a `set_app_path` builder method | A vendored fork of a Tauri plugin to add one setter, carried forever, against a crate we would then only use as a thin wrapper over the crate we already have. |
| Upstream a `set_app_path` to `tauri-plugin-autostart` | Right long-term answer, wrong timescale — the fix needs to ship before `v1.0.1`. Worth doing separately; noted in the plan. |

**Compatibility note**: the plist filename is `{app_name}.plist` and the `Label` is `{app_name}`
(`auto-launch-0.5.0/src/macos.rs`). The plugin defaulted `app_name` to `app.package_info().name`,
which for this app is the `productName` from `src-tauri/tauri.conf.json` — `"Site Checker"`, matching
the `~/Library/LaunchAgents/Site Checker.plist` reported in the issue. We must keep using
`package_info().name` so existing users' registrations are recognised rather than orphaned beside a
second file under a new name.

---

## R2 — Deriving the version-independent path (FR-001, FR-002)

**Question**: How is the stable path obtained from the running copy alone, with no build-time or
install-time configuration?

**Finding**: Homebrew guarantees `#{HOMEBREW_PREFIX}/opt/#{formula}` as a symlink to the currently
installed keg `#{HOMEBREW_PREFIX}/Cellar/#{formula}/#{version}`. The formula in this repo already
relies on exactly that: `install/homebrew/site-checker.rb` uses `opt_libexec` in both its caveats and
its wrapper, and the README's symlink instruction is
`ln -s "$(brew --prefix site-checker)/libexec/Site Checker.app" /Applications/` — `brew --prefix
<formula>` prints the `opt` path.

**Decision**: Rewrite the canonicalised executable path by component:

```text
<prefix>/Cellar/<formula>/<version>/<rest…>   →   <prefix>/opt/<formula>/<rest…>
```

Concretely,
`/opt/homebrew/Cellar/site-checker/1.0.0/libexec/Site Checker.app/Contents/MacOS/site-checker`
becomes
`/opt/homebrew/opt/site-checker/libexec/Site Checker.app/Contents/MacOS/site-checker`.

Rules that fall out of the edge cases in the spec:

- Match on the **last** path component literally equal to `Cellar`, and require at least two
  components after it (formula, version) plus a non-empty remainder. Everything before it is the
  prefix — so `/usr/local` (Intel), `/opt/homebrew` (Apple silicon) and any relocated prefix all work
  with no branch of their own. This satisfies FR-002 and the first edge case.
- **Verify before use**: the rewritten path must exist *and* canonicalise to the same path as the
  running executable. The existence check is FR-003. The equality check is the cheap guard that makes
  a false positive harmless — if the running copy happens to live under a user directory that merely
  contains a `Cellar` component, the rewrite lands somewhere that is not this application, the
  comparison fails, and we fall back to today's behaviour.
- No match, or verification fails → return the canonicalised executable path unchanged (FR-003,
  FR-004). Development builds and hand-placed copies take this branch, so their recorded path is
  byte-for-byte what it is today (SC-004).

**Alternatives considered**:

| Alternative | Rejected because |
|---|---|
| Shell out to `brew --prefix site-checker` | Adds a process launch on the startup path, needs `brew` on `PATH` (it is not, under `launchd`), and answers a question we can already answer from our own location. |
| Read `HOMEBREW_PREFIX` from the environment | Not set for a GUI launch, and would not tell us we are *inside* a keg. |
| Bake the `opt` path in at build time | Wrong for relocated prefixes and for the second architecture, and adds a configuration surface the spec's assumptions explicitly refuse. |
| Walk up looking for a `.app` and use `/Applications/Site Checker.app` | The `/Applications` entry is an optional user-made symlink that may not exist, and pointing at it would break for anyone who did not make it. |

---

## R3 — Reading the existing registration for repair (FR-005, FR-007)

**Question**: How do we learn what path is currently recorded, in order to decide whether to repair?

**Finding**: `auto-launch` writes a fixed, single-line-array XML template
(`auto-launch-0.5.0/src/macos.rs`):

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC …>
<plist version="1.0">
  <dict>
  <key>Label</key>
  <string>Site Checker</string>
  <key>ProgramArguments</key>
  <array><string>/…/site-checker</string></array>
  <key>RunAtLoad</key>
  <true/>
  </dict>
</plist>
```

`enable()` writes it with `File::create` (truncating), so re-running `enable()` **is** the rewrite —
no separate write path is needed for repair. `disable()` removes the file. `is_enabled()` is
literally `self.get_file().exists()`.

**Decision**: Extract the first `<string>…</string>` inside the `<array>` that follows the
`ProgramArguments` key, using a small pure string function, and treat *any* deviation from that
shape as "cannot be interpreted" → return `None` → leave the file untouched (FR-007). No new
dependency.

**Rationale**: The only files this has to understand are the ones this app wrote. A strict parser
that bails on anything else is not a limitation here, it is precisely the behaviour FR-007 asks for.
Adding a `plist` crate would buy correct handling of binary plists and arbitrary nesting — neither of
which can occur in a file we author and macOS never rewrites — at the cost of a dependency on the
startup path, against a constitution that keeps the backend deliberately small.

**Consequence accepted**: a registration that is *not* in this shape (hand-written, or a future
plugin's format) is never repaired. It is also never damaged. That is the trade the spec asks for.

**Alternatives considered**:

| Alternative | Rejected because |
|---|---|
| Add the `plist` crate | Generality we cannot use, on the startup path, for a file we write ourselves. |
| Shell out to `/usr/libexec/PlistBuddy` | A process launch per app start to read one string, and it fails in the same cases the string scan does. |
| Skip reading; unconditionally `enable()` on every start | Violates FR-006 — it would create a registration for a user who deliberately turned it off, on every single launch. |
| Compare file mtime or a stored hash | Answers a different question. The recorded path is the thing that matters. |

---

## R4 — What the checkbox reports, and why repair cannot disturb it (FR-006, FR-009)

**Finding**: On the LaunchAgent path, `is_enabled()` is a file-existence check and nothing else.
The checkbox therefore reflects *presence of the plist*, not its contents — which is exactly why the
defect is silent: the box stays ticked while the path inside is dead.

**Decision**: Repair is `if plist exists && recorded_path != desired_path { enable() }`.

- It cannot create a registration where none exists, because it is gated on the file existing
  (FR-006). A user who turned it off has no file, and stays off.
- It cannot change what the checkbox reports, because the file exists before and after (FR-009,
  SC-005).
- It cannot delete anything (FR-012) — `disable()` is never called from the repair path.
- An unreadable or unexpected file yields `None` from the parser, which compares as "do not repair"
  (FR-007).

---

## R5 — Where repair runs, and how failure is contained (FR-008, SC-006)

**Decision**: In `run()`'s `setup` closure in `src-tauri/src/lib.rs`, immediately after the existing
first-run marker block and before `app.manage(AppState { … })`. Every fallible step is swallowed
(`let _ = …`); no error is surfaced to the user and no error can propagate out of `setup`.

**Rationale**:

- The ordering matters and is cheap to get right: on a genuine first run the marker block calls
  `enable()` with the corrected path, so the repair that follows finds the file already correct and
  does nothing. On every later run the marker block is skipped and the repair is the only actor.
- The store is loaded and the engine started *above* this point and are untouched by it, so no
  failure here can lose or alter the site list (FR-008, SC-006). The site list is never read or
  written by any code added in this feature.
- Startup latency is one `read_to_string` of a ~400-byte file plus at most one small write — not
  worth deferring off-thread, and deferring would introduce a race with `set_autostart`.

**Rejected**: running the repair from the frontend after mount (via a new command). It would put a
data-repair decision behind the window being created and the JS having run, and it would need a new
command in the Rust/TS contract for something the user never sees.

---

## R6 — Documentation surfaces (FR-010, FR-011)

**Finding**: two places document removal today and neither mentions the login item.

- `README.md:56-69` — the `### Uninstall` block: `brew uninstall`, the optional `/Applications`
  symlink, and the site list, with the "your site list is never touched" explanation.
- `install/homebrew/site-checker.rb:108-136` — `def caveats`, whose closing paragraph covers the site
  list and the `/Applications` symlink. This file is a *template*: the release workflow renders it
  into `clintcparker/homebrew-tap`, so editing it here is how the printed notes change.

**Decision**: Add the same one-liner to both, next to the existing `/Applications` symlink step,
since that is the other file a user has to remove by hand:

```sh
rm ~/Library/LaunchAgents/"Site Checker.plist"
```

The literal filename is safe to hard-code in prose because it is derived from `productName`, which is
part of the bundle identity and changing it would be a rename of the product.

---

## R7 — What can and cannot be verified automatically

**Finding**, and the reason the spec's last assumption says what it does:

| Behaviour | How it is verified |
|---|---|
| Cellar → opt derivation, all shapes | `cargo test`, pure function, plain strings |
| Verification/fallback when `opt` is missing or points elsewhere | `cargo test` with `tempfile`, building a real `Cellar/<f>/<v>/…` tree and a real `opt/<f>` symlink |
| `ProgramArguments` extraction, including malformed input | `cargo test`, pure function |
| The repair decision (missing / stale / current / unreadable) | `cargo test`, pure function over `Option<&str>` |
| The plist actually written | Manual: run the app, `cat ~/Library/LaunchAgents/"Site Checker.plist"` |
| Survives a real `brew upgrade` | Manual, and needs two real signed builds installed in sequence — no CI runner can observe it |

No test harness change is needed: `src-tauri/src/store.rs` already establishes the `tempfile`
pattern, and `tempfile` is already a dev-dependency.

---

## Resolved unknowns

| Raised as | Resolved by |
|---|---|
| Can the plugin be told which path to register? | R1 — no; D1 replaces it |
| How is the stable path found without configuration? | R2 |
| How is the current registration read? | R3 |
| Does repairing move the checkbox? | R4 |
| Where does repair run without risking startup or the site list? | R5 |
| Which files carry the removal instructions? | R6 |
| What is testable? | R7 |

No NEEDS CLARIFICATION remains.
