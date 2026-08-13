# Implementation Notes: Launch-at-login survives upgrades

**Feature**: `20260812-202608-autostart-launchagent-path` ·
**Issue**: [#25](https://github.com/clintcparker/site-checker/issues/25) ·
**Tasks**: [tasks.md](./tasks.md) — all 34 complete

Unattended run, no user present. Everything below is a decision taken during implementation, plus
the evidence for the verification that was actually performed. The ship step should carry the
**Open decisions** and **Not verified** sections into the PR description.

---

## What was inherited

Nothing. The pinned worktree contained the four spec-artifact commits (`8fd1f08`, `f0fa64f`,
`dedf03b`, `30955da`) and no implementation. Every source change here is new.

A **different** worktree — `site-checker--20260811-141232-fix-autostart-launchagent`, branch
`20260811-141232-fix-autostart-launchagent` — holds uncommitted work against the same issue from an
earlier, abandoned run. It was read as prior art and **not adopted**: it implements a different
design (a `LoginItem` wrapper plus a three-state `Health` enum that reports a dead login item as
broken), which this feature's spec and plan deliberately do not call for. Its `stable_path`
component-scan is the one piece that survives, and it was re-derived here against
[data-model.md](./data-model.md) rules 1–3. That worktree is now redundant and can be pruned.

## What was added

| File | Change |
|---|---|
| `src-tauri/Cargo.toml` | `tauri-plugin-autostart` → `auto-launch` (already present transitively) |
| `src-tauri/Cargo.lock` | net −30 lines; drops `tauri-plugin-autostart` and `tauri-plugin` |
| `src-tauri/src/autostart.rs` | new — `manager`, `stable_path`, `desired_path`, `recorded_path`, `needs_repair`, `repair_if_stale`, `repair_with`, and 30 tests |
| `src-tauri/src/lib.rs` | owns the `AutoLaunch` as managed state; calls `repair_if_stale` after the first-run marker block |
| `src-tauri/src/commands.rs` | `get_autostart` / `set_autostart` read managed state instead of the plugin extension trait |
| `README.md` | `### Uninstall` names the LaunchAgent |
| `install/homebrew/site-checker.rb` | `def caveats` names the LaunchAgent |
| `capabilities/backend/spec.md` | new capability section, added alongside the existing one |

Test count: **65 → 95** Rust (30 added), **47 → 47** frontend (untouched, as the spec requires).

---

## Open decisions (for the PR to surface)

1. **`repair_if_stale` was split around an injected write.** T023 specifies
   `repair_if_stale(manager, plist_path)` calling `manager.enable()` directly, and that is exactly
   what ships. But `AutoLaunch::enable()` resolves its own target from `$HOME` — it ignores the
   `plist_path` argument entirely — so a test calling it would rewrite the *developer's real*
   `~/Library/LaunchAgents/Site Checker.plist` on every `cargo test`. The decision logic therefore
   lives in `repair_with(plist_path, desired, rewrite)`, which takes the write as a closure; the
   public function is a one-line wrapper. Tests drive `repair_with`. Verified after the fact: the
   real login item's mtime was unchanged by the full suite.
   *If review prefers the literal signature, the cost is a test suite that mutates the developer's
   login item.*
2. **§2a was exercised by deleting the plist rather than unticking the checkbox.** No user was
   present to click. `set_autostart(false)` does nothing but `disable()`, which removes that file,
   so the startup path under test sees an identical world. The checkbox's own behaviour is
   unchanged by this feature and is covered by the unchanged command surface.
3. **`stable_path` requires three components after `Cellar`** (formula, version, non-empty
   remainder), matching data-model.md rule 2. A path ending at the version directory yields `None`
   rather than a rewrite, because an executable is never the version directory itself.
4. **The two remaining `tauri-plugin-autostart` mentions are prose.** T008's grep expects zero
   hits; the two that remain are comments in `Cargo.toml` and `lib.rs` explaining *why* the plugin
   was replaced. No code references it and it is gone from the lockfile.

## Verified by hand (quickstart §2)

Run against `pnpm tauri dev` on this branch. The real login item was backed up to
`~/site-checker-qa-safety/` and `/tmp` first, and restored byte-for-byte afterwards (confirmed by
`diff`); `sites.json` was untouched throughout (mtime unchanged, still 6 entries).

| Step | Expected | Observed | |
|---|---|---|---|
| §2 | dev build records `target/debug/…` (FR-004) | recorded `…/src-tauri/target/debug/site-checker` | ✅ |
| §2a | repair never creates a registration (FR-006) | file stayed absent, app started | ✅ |
| §2b | stale registration repairs itself (FR-005) | `…/Cellar/site-checker/0.0.0/…` → running path; still enabled; no warning | ✅ |
| §2c | unreadable registration left alone (FR-007) | byte-for-byte identical (sha unchanged), app started, no output | ✅ |
| §2d | a failing rewrite cannot block startup (FR-008) | app started, file unchanged, no output | ✅ |

**§2d needed correcting as written.** The quickstart says `chmod 500 ~/Library/LaunchAgents`, but
truncating an *existing* file needs write permission on the file, not the directory — so the rewrite
succeeded and the step proved nothing. It was re-run with `chmod 400` on the plist as well, which
makes `File::create` fail with `EACCES`; only then does it test what it claims to.
**quickstart.md §2d should be amended.**

## Not verified by this branch

Neither can run here, and both are named in the spec's own assumptions:

- **quickstart §3** — a real `brew install` proving FR-001 end to end.
- **quickstart §4** — a real `brew upgrade` across two builds proving SC-001.

Both need a released bottle *containing this change*, which does not exist until this merges. The
first release after this lands is the first opportunity to confirm the fix against a real upgrade.
Until then the `Cellar → opt` rewrite is covered only by unit tests over path strings and by the
`desired_path` tempfile tests that build a real `opt/<formula>` symlink to a real keg directory.
