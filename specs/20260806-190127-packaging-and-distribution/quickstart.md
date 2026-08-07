# Quickstart: Validating Packaging & Distribution

How to prove this feature works — without publishing junk, and without burning a real version
number to find out that a placeholder was misspelled.

Read alongside [contracts/workflows.md](./contracts/workflows.md) and
[contracts/install-channel.md](./contracts/install-channel.md); nothing here restates them.

## Prerequisites

| For | Need |
|---|---|
| Everything | This branch checked out; `gh` authenticated (already true here) |
| Phases D–G | `clintcparker/site-checker` public (research R1) |
| Phases D–G | Apple Developer Program membership + Developer ID certificate + App Store Connect API key (research R2) |
| Phase D | `TAP_PUSH_TOKEN` set on this repository, scoped to `clintcparker/homebrew-tap` only |
| Scenario 6 | A second Mac, or a fresh user account, that has never built this project |

Scenarios 1–4 need none of the above and can be run today.

---

## Scenario 1 — The headless build no longer hangs (FR-016, SC-009)

The failure this replaces: `pnpm tauri build` reaching `bundle_dmg.sh`, calling `osascript`, and
blocking forever on a GUI-automation prompt.

```sh
cd src-tauri && cargo build --release 2>&1 | tail -3   # warm the cache first
cd .. && pnpm tauri build --bundles app
```

**Expected**: exits 0, and `src-tauri/target/release/bundle/macos/Site Checker.app` exists.
**Expected**: `src-tauri/target/release/bundle/dmg/` is *not* created.

Then confirm the default is safe, not just the flag — with `tauri.conf.json`'s targets reduced to
`["app"]`, a bare `pnpm tauri build` must also produce no DMG.

---

## Scenario 2 — One version, and it comes from the tag (FR-008, SC-003)

```sh
grep -c '"version"' src-tauri/tauri.conf.json   # expect 0
grep '^version' src-tauri/Cargo.toml            # expect version = "0.0.0"
```

Then prove the fallback is real — the claim that deleting `version` from `tauri.conf.json` makes
`Cargo.toml` authoritative:

```sh
sed -i '' 's/^version = "0.0.0"/version = "9.9.9"/' src-tauri/Cargo.toml
pnpm tauri build --bundles app
/usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" \
  "src-tauri/target/release/bundle/macos/Site Checker.app/Contents/Info.plist"
# expect: 9.9.9
git checkout src-tauri/Cargo.toml
```

**Expected**: `9.9.9`. Do not skip this — it is the one assumption in R5 that comes from reading
Tauri's source rather than from running it.

---

## Scenario 3 — CI catches a hand-bumped version (FR-008)

On a scratch branch, set `src-tauri/Cargo.toml`'s version to `1.0.0` and open a PR.

**Expected**: the `rust` job fails at the sentinel guard with a message naming the tag as the
source of truth. Revert; expect green.

---

## Scenario 4 — The render guards fail in both directions (FR-013)

Run the render step's logic locally against the template, with fake checksums.

**Direction 1 — template missing a placeholder.** Delete `SHA256_X86_64` from
`install/homebrew/site-checker.rb` and run the guard.
**Expected**: `::error::placeholder SHA256_X86_64 missing from …`, non-zero exit.

**Direction 2 — placeholder survives.** Restore the template, then run the substitution with the
`SHA256_X86_64` case removed from the `sed` script.
**Expected**: `::error::placeholder SHA256_X86_64 survived rendering …`, non-zero exit.

Finally, render cleanly and check the output parses as Ruby:

```sh
ruby -c rendered/site-checker.rb     # Syntax OK
brew audit --cask --new rendered/site-checker.rb   # style/consistency
```

**Expected**: no placeholder tokens remain, a literal `version "…"`, two 64-hex checksums.

---

## Scenario 5 — Pre-flight fails fast and names the gap (FR-017, FR-018, SC-005, US3)

Use a **throwaway prerelease tag**, not a version you intend to keep.

```sh
gh secret delete TAP_PUSH_TOKEN --repo clintcparker/site-checker   # temporarily
git tag -a v0.0.1-preflight-test -m "scratch" && git push origin v0.0.1-preflight-test
gh run watch
```

**Expected**: the run fails in under 60 seconds; the log names `TAP_PUSH_TOKEN` and points at
`docs/how-to/release.md`; **no** release object exists and the tap is unchanged.

Note the tag form here is itself a check — `v0.0.1-preflight-test` should be rejected by the
version-validation gate before the secret probe even matters. Use `v0.0.1` if you want to exercise
the secret probe specifically.

Clean up:

```sh
git push origin :refs/tags/v0.0.1-preflight-test
gh release delete v0.0.1-preflight-test --yes   # only if one was created
gh secret set TAP_PUSH_TOKEN --repo clintcparker/site-checker
```

---

## Scenario 6 — The whole thing, end to end (US1, US2, SC-001, SC-002, SC-004, SC-010)

This is the scenario that answers FR-027 (research R10). Nothing before it proves the feature.

```sh
git tag -a v1.0.0 -m "site-checker 1.0.0"
git push origin v1.0.0
gh run watch
```

**Expected from the run**: a public release for `v1.0.0` carrying both
`site-checker-aarch64-apple-darwin.zip` and `site-checker-x86_64-apple-darwin.zip` plus generated
notes; a tap commit `site-checker 1.0.0` creating `Casks/site-checker.rb`.

**Then, on a Mac that has never built this project:**

```sh
brew install clintcparker/tap/site-checker
spctl -a -t exec -vv "/Applications/Site Checker.app"
/usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" \
  "/Applications/Site Checker.app/Contents/Info.plist"
```

**Expected**: `accepted` / `source=Notarized Developer ID`; version `1.0.0`; the app opens with no
security dialog and no right-click-open (SC-002). Time the whole thing — SC-001 claims under 5
minutes.

**Uninstall, both ways** (FR-003, FR-004, SC-010):

```sh
open "/Applications/Site Checker.app"   # add a site, quit
brew uninstall site-checker
ls ~/Library/Application\ Support/com.clintparker.site-checker/sites.json   # expect: present

brew install clintcparker/tap/site-checker   # sites restored?
brew uninstall --zap site-checker
ls ~/Library/Application\ Support/com.clintparker.site-checker   # expect: No such file or directory
```

---

## Scenario 7 — Re-running converges (FR-014, SC-007)

Re-run the completed `v1.0.0` release from the Actions UI.

**Expected**: still exactly one release for `v1.0.0` (updated in place, not duplicated); still
exactly one `Casks/site-checker.rb`. Note that a re-run rebuilds, and a Tauri build is not
bit-reproducible, so the checksums change and the tap **will** commit again — that is correct
(the cask must match the assets that exist). The skip-when-unchanged path is exercised by
re-running only the `homebrew` job, which re-downloads the same assets and should log
`Cask unchanged — skipping commit`.

---

## Scenario 8 — Verification catches a lagging channel (US4, FR-020)

```sh
gh workflow run verify-install-channels.yml
gh run watch
```

**Expected**: green against the current release.

Then, in the tap, hand-edit `Casks/site-checker.rb`'s `version` to an older value and re-dispatch.
**Expected**: fails, naming the tap as the lagging channel. Restore by re-running the release
workflow's `homebrew` job — which is also a check that the tap is genuinely machine-recoverable
and never needs hand-editing.

---

## Gate summary

| Scenario | Proves | Needs R1/R2 |
|---|---|---|
| 1 | FR-016, SC-009 | no |
| 2 | FR-008, SC-003 | no |
| 3 | FR-008 enforcement | no |
| 4 | FR-013 both directions | no |
| 5 | FR-017, FR-018, SC-005 | partly (public repo for Actions minutes) |
| 6 | US1, US2, SC-001/002/004/010 | yes |
| 7 | FR-014, SC-007 | yes |
| 8 | US4, FR-019, FR-020, SC-006 | yes |

Plus the standing constitutional gates, unchanged and expected to stay green throughout, since no
application code is touched: `cargo test` (55), `pnpm test` (30), `cargo clippy -- -D warnings`.
</content>
