# Quickstart: validating "Launch-at-login survives upgrades"

**Feature**: `20260812-202608-autostart-launchagent-path` · **Spec**: [spec.md](./spec.md) ·
**Contract**: [contracts/launch-agent-plist.md](./contracts/launch-agent-plist.md)

How to prove this feature works. Everything in §1 and §2 runs on any machine in under a minute.
§3 needs a real Homebrew install; §4 needs two real builds and is the one thing no runner can do
for you.

> **Careful:** §2 and §3 read and rewrite your *real*
> `~/Library/LaunchAgents/Site Checker.plist`. Back it up first
> (`cp ~/Library/LaunchAgents/"Site Checker.plist" /tmp/`) and restore it when you are done, or you
> will be debugging your own login item afterwards.

## Prerequisites

- macOS (the Rust side of this project builds and tests on macOS only — see `.github/workflows/ci.yml`)
- Rust stable ≥ 1.88, Node + pnpm
- `pnpm install` once

---

## 1. Automated checks — the whole decision surface

```sh
cargo test  --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml -- -D warnings
pnpm test
```

**Expected**: all green, and `pnpm test` unchanged in count — this feature touches no frontend file.

The new tests in `src-tauri/src/autostart.rs` cover, without a Homebrew install or a real plist:

| Covers | Scenario |
|---|---|
| FR-001, FR-002 | `…/Cellar/site-checker/1.0.0/libexec/…` derives `…/opt/site-checker/libexec/…`, for both `/opt/homebrew` and `/usr/local` and a relocated prefix |
| FR-003 | Derived path that does not exist → the running path is used |
| FR-003 | Derived path that exists but resolves to a different application → the running path is used |
| FR-004 | No `Cellar` component (hand-built, `target/debug`) → the running path, unchanged |
| Edge: unrelated `Cellar` | A user directory called `Cellar` does not produce a bogus registration |
| Contract | `ProgramArguments[0]` is extracted from the real template, including a path containing spaces |
| FR-007 | Empty file, truncated XML, binary bytes, missing `ProgramArguments`, empty `<array>` → no recorded path, no repair |
| FR-005 | Recorded ≠ desired → repair |
| FR-005 | Recorded == desired → no repair |
| FR-006 | No registration at all → no repair (never creates one) |

**Success criteria exercised**: SC-004 (unchanged for non-package installs), SC-006 (no failure path
introduced), and the decision half of SC-001/SC-002.

---

## 2. Run it and look at the file

```sh
pnpm tauri dev
```

Then, in another terminal:

```sh
cat ~/Library/LaunchAgents/"Site Checker.plist"
```

**Expected**: `ProgramArguments[0]` is the dev binary under `src-tauri/target/debug/…` — a
development build has no stable path, so nothing changed for it (FR-004). The "Launch at login"
checkbox in the window is ticked, matching the file's presence (FR-009).

### 2a. Repair does not resurrect a setting you turned off (FR-006, US2 scenario 2)

1. Untick "Launch at login". Confirm the plist is gone:
   `ls ~/Library/LaunchAgents/"Site Checker.plist"` → *No such file*.
2. Quit and relaunch the app.

**Expected**: still no file, and the checkbox is still unticked. Repair never creates a registration.

### 2b. A stale registration repairs itself (FR-005, US2 scenario 1)

With the app **not** running, point the registration at a path that does not exist:

```sh
sed -i '' 's|<array><string>[^<]*</string></array>|<array><string>/opt/homebrew/Cellar/site-checker/0.0.0/libexec/Site Checker.app/Contents/MacOS/site-checker</string></array>|' \
  ~/Library/LaunchAgents/"Site Checker.plist"
```

Relaunch the app, then `cat` the file again.

**Expected**: `ProgramArguments[0]` is back to the running copy's desired path, the file still
exists, and the checkbox still reads ticked. No warning banner appeared. **SC-002, SC-005.**

### 2c. An unreadable registration is left alone (FR-007, US2 scenario 4)

With the app not running:

```sh
printf 'not a plist at all' > ~/Library/LaunchAgents/"Site Checker.plist"
```

Relaunch the app.

**Expected**: the app starts normally, your site list is intact, the file still contains
`not a plist at all` byte for byte, and nothing was reported to the user. **SC-006.**

### 2d. Nothing can stop the app starting (FR-008, SC-006)

Repeat 2b and 2c with `~/Library/LaunchAgents` made unwritable (`chmod 500`), so the rewrite itself
fails.

**Expected**: the app still starts, the window still lists your sites, no warning. Restore with
`chmod 700 ~/Library/LaunchAgents`.

---

## 3. The real thing: a Homebrew install (US1, FR-001)

Needs an actual `brew install clintcparker/tap/site-checker` of a build containing this change.

```sh
brew install clintcparker/tap/site-checker
open "$(brew --prefix site-checker)/libexec/Site Checker.app"
cat ~/Library/LaunchAgents/"Site Checker.plist"
```

**Expected**: `ProgramArguments[0]` contains `/opt/site-checker/` and **no version number** —
`/opt/homebrew/opt/site-checker/libexec/Site Checker.app/Contents/MacOS/site-checker`. **US1
scenario 2.**

Repeat via the optional symlink (`ln -s "$(brew --prefix site-checker)/libexec/Site Checker.app" /Applications/`
then open it from Finder): the recorded path must be identical, because `canonicalize` collapses both
launches to the same keg. **Edge case: launched through the `/Applications` shortcut.**

Sanity-check the fallback: `brew unlink site-checker` removes the `opt` link; launching the copy
directly out of the Cellar then records the keg path rather than a dangling `opt` path (FR-003).
`brew link site-checker` afterwards.

---

## 4. The upgrade itself (US1 scenario 1, SC-001) — manual, two builds

No automated test can observe a package-manager upgrade; this is why the spec says so explicitly.

1. Install version *N*, launch it once, leave "Launch at login" on.
2. Record `ProgramArguments[0]` — it must contain `/opt/site-checker/`.
3. `brew upgrade site-checker` to version *N+1*. The old `Cellar/site-checker/N/` is now gone.
4. Without launching anything: `ls -l "$(sed -n 's|.*<array><string>\(.*\)</string></array>.*|\1|p' ~/Library/LaunchAgents/"Site Checker.plist")"`

**Expected**: the recorded path still resolves — it now points through `opt` at version *N+1*.
Log out and back in: Site Checker opens by itself. **SC-001.**

A downgrade (`brew install site-checker@…`, or reinstalling an older bottle) is the same event and
must behave identically.

---

## 5. Documentation (US3, FR-010, FR-011, SC-003)

```sh
grep -n "LaunchAgents" README.md install/homebrew/site-checker.rb
```

**Expected**: both files, each in its removal instructions, naming
`rm ~/Library/LaunchAgents/"Site Checker.plist"` alongside the existing `/Applications` symlink step.

Then follow the README's `### Uninstall` block end to end on a real install and confirm nothing under
`~/Library/LaunchAgents`, `/Applications`, or the Homebrew prefix still references Site Checker —
only `~/Library/Application Support/com.clintparker.site-checker`, which is documented as
deliberately kept. **SC-003.**
