# Contract — Shipped Build Identity

FR-004 says the rename must not move the shipped product. This file is the machine-checkable
form of that: the values recorded from the **pre-change** build, and the commands that
re-read them afterwards. Research R1 showed one of them moves by default, so this is a real
check, not a formality.

Source of truth for these values is `src-tauri/tauri.conf.json` plus the built bundle at
`src-tauri/target/release/bundle/macos/Site Checker.app`.

---

## Invariants

| Key | Required value | Where it comes from |
|---|---|---|
| `CFBundleName` | `Site Checker` | `tauri.conf.json` → `productName` |
| `CFBundleDisplayName` | `Site Checker` | `tauri.conf.json` → `productName` |
| `CFBundleIdentifier` | `com.clintparker.site-checker` | `tauri.conf.json` → `identifier` |
| `CFBundleExecutable` | `tauri-app` | **default: Cargo `[package] name` → after this change, pinned by `mainBinaryName`** |
| Bundle path | `…/bundle/macos/Site Checker.app` | `productName` |
| Window title | `Site Checker` | `tauri.conf.json` → `app.windows[0].title` |
| Config dir | `~/Library/Application Support/com.clintparker.site-checker/` | derived from `identifier` |

`CFBundleIdentifier` is the load-bearing one: `app_config_dir()` derives the user's
`sites.json` location from it, so a change there would strand existing data (Principle II).
`CFBundleExecutable` is the one that silently moves — see below.

---

## Baseline (recorded from the pre-change build, 2026-08-05)

```
$ ls "src-tauri/target/release/bundle/macos/Site Checker.app/Contents/MacOS/"
tauri-app

$ plutil -p "src-tauri/target/release/bundle/macos/Site Checker.app/Contents/Info.plist"
  "CFBundleDisplayName" => "Site Checker"
  "CFBundleExecutable"  => "tauri-app"
  "CFBundleIdentifier"  => "com.clintparker.site-checker"
  "CFBundleName"        => "Site Checker"
```

## Verification after the change

```bash
cd src-tauri && cargo clean && cd ..
pnpm tauri build --bundles app     # --bundles app skips the DMG step (roadmap §5: osascript hangs headless)

ls "src-tauri/target/release/bundle/macos/Site Checker.app/Contents/MacOS/"
plutil -p "src-tauri/target/release/bundle/macos/Site Checker.app/Contents/Info.plist" \
  | grep -E "CFBundle(Name|DisplayName|Identifier|Executable)"
```

Every line must match the baseline exactly. `cargo clean` is not optional — an incremental
build can resolve a stale `tauri_app_lib` from cache and hide an unresolved reference
(spec edge case; SC-005).

---

## Why `mainBinaryName` is pinned

`CFBundleExecutable` is not configured anywhere today — it defaults to cargo's output
binary, i.e. `[package] name`, which is why it currently reads `tauri-app`. Renaming the
package to `site-checker` renames the executable inside the bundle.

The already-installed LaunchAgent hardcodes the full path to it:

```xml
<!-- ~/Library/LaunchAgents/Site Checker.plist -->
<array><string>/Applications/Site Checker.app/Contents/MacOS/tauri-app</string></array>
```

so the rename would point launch-at-login at a file that no longer exists. Worse, the
`autostart.initialized` marker already exists, so `lib.rs`'s first-run block will not
re-register it — the checkbox keeps reading "on" while nothing launches. That is an
observable behavior change (FR-010) and an explicitly pinned value (FR-004).

Adding to `tauri.conf.json`:

```json
"mainBinaryName": "tauri-app"
```

tells `tauri build` to rename cargo's output back to `tauri-app` before bundling, holding
`CFBundleExecutable` still while the source-side identity is corrected. Supported by the
pinned toolchain (`tauri-utils 2.9.3`, `main_binary_name: Option<String>`); applies at
build/bundle time only, so `tauri dev` is unaffected.

**Known tension**: this leaves the literal string `tauri-app` in `tauri.conf.json` after a
feature whose SC-002 asks for zero scaffold placeholder values. It is a deliberate
compatibility anchor, not a missed placeholder — but it is the same string, and the
decision is flagged for the user in [../research.md](../research.md) R1 along with the
alternative (drop the pin, then untick/re-tick **Launch at login** once after installing).

## Non-goals

- Not renaming the bundle, identifier, window title, or config directory.
- Not changing bundle targets, icons, version, or signing.
- Not addressing bundle size — that is roadmap §5 and explicitly out of scope.
