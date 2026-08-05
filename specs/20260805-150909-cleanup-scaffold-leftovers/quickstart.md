# Quickstart — Validating the v1 Cleanup

How to prove this feature landed correctly. Everything here is runnable; nothing depends on
a new test harness.

The whole feature's claim is "observably identical app, honest metadata." Validation is
therefore mostly **negative** — grep for what should be gone, diff what should not have
moved — plus the existing suite to catch anything that did move.

## Prerequisites

- macOS with the repo's pinned Rust toolchain (`rust-toolchain.toml`) and `pnpm`
- Run from the worktree root: `site-checker--20260805-150909-cleanup-scaffold-leftovers/`
- **Record the baseline before touching anything** — see
  [contracts/build-identity.md](./contracts/build-identity.md). The pre-change bundle
  under `src-tauri/target/release/bundle/macos/` is what the post-change build is compared
  against; do not `cargo clean` until it has been recorded.

---

## Tier 1 — the gate, after every item (FR-011)

```bash
cargo test --manifest-path src-tauri/Cargo.toml
pnpm test
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

Expected: Rust ≥ 30 tests passing (29 baseline + the new warning-distinction test),
frontend 12 passing, clippy silent. Per SC-006 the count may rise but must never fall, and
no test may be deleted to accommodate a change.

Run this after each of the five items, not once at the end — it is what makes the items
independently shippable.

---

## Tier 2 — per-item checks

### US1 · Warning messages (`store.rs`)

The distinction is pinned by the new unit test, so Tier 1 covers the regression. To read
the two strings side by side and confirm SC-004 by eye:

```bash
grep -n "Starting with an empty list" src-tauri/src/store.rs
```

Expected: two messages that share no opening phrase — one about *opening* the file, one
about the contents being *damaged*, the latter still promising the file was left alone.
Exact text and the pairwise property: [contracts/warning-messages.md](./contracts/warning-messages.md).

Optional live check (destructive to your real `sites.json` — back it up first):

```bash
CFG=~/Library/Application\ Support/com.clintparker.site-checker
cp "$CFG/sites.json" "$CFG/sites.json.bak"
echo '{ this is not json' > "$CFG/sites.json"
# launch the app → banner names damage and says the file was left alone
cp "$CFG/sites.json.bak" "$CFG/sites.json"
```

### US2 · Opener plugin removed (FR-001, FR-002, SC-001)

Must be verified from a **fresh install** — an existing `node_modules` still containing the
package would hide an incomplete removal (spec edge case).

```bash
rm -rf node_modules && pnpm install
grep -rn "opener" --include="*.json" --include="*.toml" --include="*.rs" \
  --include="*.ts" --include="*.html" . | grep -v node_modules | grep -v target
grep -c "opener" pnpm-lock.yaml src-tauri/Cargo.lock
```

Expected: zero matches from the grep; `0` from both lockfiles.

### US3 · Identity metadata (FR-003, FR-004, SC-002, SC-005)

```bash
grep -rn "tauri-app\|tauri_app\|A Tauri App\|\"you\"" \
  package.json src-tauri/Cargo.toml src-tauri/src/ index.html
```

Expected: exactly one match — `"mainBinaryName": "tauri-app"` is in `tauri.conf.json`, not
in the files listed above, so this grep should return **nothing**. Add
`src-tauri/tauri.conf.json` to the paths and the single expected match is the deliberate
pin (research R1).

Then the shipped-identity check — the important half:

```bash
cd src-tauri && cargo clean && cd ..
pnpm tauri build --bundles app
ls "src-tauri/target/release/bundle/macos/Site Checker.app/Contents/MacOS/"
plutil -p "src-tauri/target/release/bundle/macos/Site Checker.app/Contents/Info.plist" \
  | grep -E "CFBundle(Name|DisplayName|Identifier|Executable)"
```

Expected: executable still named `tauri-app`; all four `CFBundle*` values identical to the
baseline. `cargo clean` is required (SC-005) — an incremental build can resolve a stale
`tauri_app_lib` from cache and hide a broken rename. `--bundles app` skips the DMG step,
which hangs on `osascript` without an interactive session (roadmap §5).

**If `CFBundleExecutable` reads `site-checker`, the `mainBinaryName` pin is missing or
misspelled.** Fix it before shipping — that state breaks launch-at-login on the installed
copy with no self-repair.

### US4 · Scaffold assets removed (FR-005, SC-003)

```bash
ls src/assets 2>&1                      # expect: No such file or directory
grep -rn "tauri.svg\|typescript.svg\|vite.svg" src/ index.html   # expect: no matches
```

### US5 · Doc comment (FR-009)

```bash
grep -n -B 8 "fn has_leading_scheme" src-tauri/src/model.rs
```

Expected: the comment names the accepted character class (ASCII alphanumeric, `+`, `-`,
`.`) and states that a `://` at position zero does not count. Read it without looking at
the body and predict whether `my-app.v2://x`, `://x`, and `foo bar://x` have a leading
scheme; check against the code (answers: yes, no, no).

---

## Tier 3 — manual launch (SC-007)

Automated tests do not cover UI wiring, by design (Principle IV). Launch the built app once
and confirm the behavior is unchanged:

1. Window opens titled **Site Checker**, sized as before.
2. Add a site → row appears **Pending**, then resolves to up/down within its interval.
3. Edit its URL and interval → the row updates.
4. Watch the "time since" column tick.
5. Delete the site → the row goes; the empty-state message returns.
6. Toggle **Launch at login** off and on → no banner, state sticks across a relaunch.
7. Quit by closing the window → the process exits.

Step 6 matters most: it is the behavior research R1 identified as at risk from the rename.
If launch-at-login is already on before you start, verify
`~/Library/LaunchAgents/Site Checker.plist` still points at an executable that exists:

```bash
ls -l "$(plutil -extract ProgramArguments.0 raw ~/Library/LaunchAgents/Site\ Checker.plist)"
```

---

## Done when

- [ ] Tier 1 green after every item; test count ≥ baseline, none deleted (SC-006)
- [ ] Zero opener references in declarations, source, or either lockfile, from a fresh install (SC-001)
- [ ] Zero scaffold placeholders in identity metadata; the only `tauri-app` left is the documented `mainBinaryName` pin (SC-002)
- [ ] `src/assets/` gone, nothing references the SVGs (SC-003)
- [ ] The two warnings are distinguishable on their own (SC-004)
- [ ] Clean build succeeds; bundle name, identifier, window title, and executable match the baseline (SC-005)
- [ ] Manual pass complete, including the launch-at-login toggle (SC-007)
- [ ] Anything found outside roadmap section 1 was appended to `docs/ROADMAP.md`, not fixed here
