# Contract: the LaunchAgent registration

**Feature**: `20260812-202608-autostart-launchagent-path`

This is the one external interface this feature changes: the file Site Checker hands to macOS's
`launchd` so the app opens at login. It is a contract in the strict sense — macOS reads it, the user
can read and delete it, and a previous version of Site Checker wrote the copy that is already on
disk.

The Tauri command surface is documented at the bottom and is **unchanged**.

---

## File

| | |
|---|---|
| Path | `~/Library/LaunchAgents/Site Checker.plist` |
| Filename | `{app_name}.plist` where `app_name` = `productName` in `src-tauri/tauri.conf.json` |
| Encoding | UTF-8, XML plist |
| Written by | `auto_launch::AutoLaunch::enable()` (truncating create) |
| Removed by | `auto_launch::AutoLaunch::disable()` — reached only from `set_autostart(false)` |
| Read by | macOS `launchd`; this app, to decide whether a repair is needed |

## Shape

Exactly what `auto-launch` 0.5 writes, byte for byte. The array is on one line.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
  <key>Label</key>
  <string>Site Checker</string>
  <key>ProgramArguments</key>
  <array><string>ABSOLUTE_EXECUTABLE_PATH</string></array>
  <key>RunAtLoad</key>
  <true/>
  </dict>
</plist>
```

| Key | Value | Notes |
|---|---|---|
| `Label` | `Site Checker` | Same as the filename stem. Not compared or rewritten by this app. |
| `ProgramArguments` | one-element array | Element 0 is the recorded executable path. The app is launched with no arguments (`Some(vec![])` today), so there is never an element 1. |
| `RunAtLoad` | `true` | Constant. |

`ProgramArguments[0]` **must be the inner executable**
(`…/Site Checker.app/Contents/MacOS/site-checker`), not the `.app` bundle — `launchd` execs it
directly.

## What changes

Only the value of `ProgramArguments[0]`.

| Install | Before this feature | After this feature |
|---|---|---|
| Homebrew, `opt` present | `/opt/homebrew/Cellar/site-checker/1.0.0/libexec/Site Checker.app/Contents/MacOS/site-checker` | `/opt/homebrew/opt/site-checker/libexec/Site Checker.app/Contents/MacOS/site-checker` |
| Homebrew, `opt` missing/dangling | keg path | keg path (unchanged) |
| Hand-built / `/Applications` copy | its own path | its own path (unchanged) |
| Development build | `target/debug/…` | `target/debug/…` (unchanged) |

The file's structure, filename, `Label`, and `RunAtLoad` are all unchanged, so a registration written
by v1.0.0 stays valid input to v1.0.1 and vice versa. There is no migration and no version marker.

## Reader contract (repair)

The app reads this file on every start to decide whether to rewrite it.

| Input | Interpretation | Action |
|---|---|---|
| File does not exist | The user has launch-at-login off | Nothing. Never create it (FR-006) |
| Parses, `ProgramArguments[0]` == desired path | Current | Nothing |
| Parses, `ProgramArguments[0]` != desired path | Stale | `enable()` — rewrite in place, still enabled (FR-005) |
| Unreadable (I/O error), or does not match the shape above | Not ours to interpret | Nothing. No warning to the user (FR-007) |

"Matches the shape above" means: a `<key>ProgramArguments</key>` followed by an `<array>` containing
at least one `<string>…</string>`. Anything else — a binary plist, a hand-written variant, a nested
structure, a truncated file — yields no recorded path and therefore no action. The file is never
partially written: `enable()` truncates and rewrites whole.

## Removal contract (documentation)

Removing this file is a user action, documented in two places that must agree:

- `README.md`, `### Uninstall`
- `install/homebrew/site-checker.rb`, `def caveats` — a template; the release workflow renders it
  into the tap, which is what `brew install` prints

```sh
rm ~/Library/LaunchAgents/"Site Checker.plist"
```

The app never runs this itself (FR-012).

---

## Tauri command surface — unchanged

Stated so the tasks and review steps can confirm nothing here moved.

| Command | Args | Returns | Behaviour |
|---|---|---|---|
| `get_autostart` | — | `Result<bool, String>` | Whether the registration file exists |
| `set_autostart` | `enabled: bool` (`enabled` in JS, per Tauri's argument convention) | `Result<bool, String>` | Creates or removes the registration, then returns the state actually in effect so the checkbox can correct itself |

Both keep their names, arguments, return types, and error-to-string behaviour. `src/api.ts` and
`src/main.ts` are not modified. `src-tauri/capabilities/default.json` keeps `core:default` only —
no plugin permission is added or removed, because none was ever granted.
