# Contract: Workflows

Three files under `.github/workflows/`, deliberately separate (FR-023). This records each one's
triggers, job graph, permissions, and — most importantly — its failure semantics: what must fail,
where, and what must not have happened by then.

---

## `ci.yml` — Continuous Integration (US5)

**Triggers**

```yaml
on:
  push:
    branches: [main]
  pull_request:
```

**Permissions**: `contents: read`

**Jobs** (parallel, independent):

| Job | Runner | Steps |
|---|---|---|
| `rust` | `macos-latest` | checkout → `dtolnay/rust-toolchain@stable` (components: `clippy`) → `Swatinem/rust-cache` → `cargo test --locked --manifest-path src-tauri/Cargo.toml` → `cargo clippy --locked --manifest-path src-tauri/Cargo.toml -- -D warnings` → version-sentinel guard |
| `frontend` | `ubuntu-latest` | checkout → `pnpm/action-setup` → `actions/setup-node` (cache: pnpm) → `pnpm install --frozen-lockfile` → `pnpm test` → `pnpm build` |

**Why `rust` runs on macOS**: `src-tauri` links macOS system frameworks through `tauri` 2.11;
building it on Ubuntu would require WebKitGTK dev packages and would test a configuration nobody
ships. See research R11.

**Version-sentinel guard** (enforces FR-008 / SC-003):

```sh
grep -q '^version = "0.0.0"' src-tauri/Cargo.toml || {
  echo "::error::src-tauri/Cargo.toml version must stay 0.0.0 — the release tag is the source of truth"
  exit 1
}
```

…and the equivalent for `package.json`.

**Failure semantics**

| Condition | Result |
|---|---|
| Any of the four gates red | Job fails, named by its step; PR check red (FR-022, SC-008) |
| A hand-bumped version | `rust` job fails at the guard |
| No interactive desktop available | Irrelevant — nothing here runs the bundler (SC-009 holds trivially) |

**Explicitly not here**: `pnpm tauri build`. FR-022 names four gates; a full bundle is not one,
and adding it would drag the DMG hang and the signing credentials into every PR.

---

## `release.yml` — Release (US1, US2, US3)

**Triggers**

```yaml
on:
  push:
    tags: ['v*']
```

**Permissions**: `contents: write`

**Job graph**

```text
preflight ──▶ test ──▶ build (matrix ×2) ──▶ release ──▶ homebrew
    │                                                        ▲
    └────────────────── version ────────────────────────────┘
```

### `preflight` — `ubuntu-latest`

Outputs `version`. Two steps:

1. **Derive and validate the version** from `GITHUB_REF_NAME` per
   [version-and-artifacts.md](./version-and-artifacts.md). A malformed tag exits non-zero here
   (FR-009).
2. **Check one-time setup** — presence of `TAP_PUSH_TOKEN`, `APPLE_CERTIFICATE`,
   `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_API_ISSUER`, `APPLE_API_KEY_ID`,
   `APPLE_API_KEY`. Accumulate, then exit once — so a first-time setup reports *all* missing
   secrets in one run rather than one per attempt. Each emits
   `::error::one-time setup missing: <NAME> (see docs/how-to/release.md)`.

**Contract**: no checkout, no toolchain, completes in seconds. Every other job `needs:` it
directly or transitively, so a failure here means nothing was built, no release object exists, and
the tap was not touched (FR-017, FR-018, SC-005).

**Known limit**: presence ≠ validity. An expired certificate or a mis-scoped token passes
pre-flight and fails later. Documented in `docs/how-to/release.md`; see research R9.

### `test` — `macos-latest`

The same four gates as `ci.yml`. Present because a tag can be pushed at a commit CI never
evaluated, and FR-023 permits the release pipeline to run its own gate.

### `build` — `macos-latest`, matrix over `[aarch64, x86_64]`

Per entry:

1. checkout; `rustup target add <arch>-apple-darwin`; pnpm + node setup
2. **Stamp** `src-tauri/Cargo.toml` with `needs.preflight.outputs.version`
3. **Import signing certificate** — decode `APPLE_CERTIFICATE` into a temporary keychain
4. `pnpm install --frozen-lockfile`
5. `pnpm tauri build --bundles app --target <arch>-apple-darwin`
   — with `APPLE_SIGNING_IDENTITY`, `APPLE_API_ISSUER`, `APPLE_API_KEY_ID`, `APPLE_API_KEY_PATH`
   exported, so Tauri signs, submits for notarization, waits, and staples in-band
6. `ditto -c -k --sequesterRsrc --keepParent "…/Site Checker.app" site-checker-<arch>-apple-darwin.zip`
7. `actions/upload-artifact`

**Contract**: `--bundles app` is mandatory — the `dmg` target hangs on `osascript` with no
interactive session (FR-016, SC-009). `tauri.conf.json`'s default targets are also reduced to
`["app"]` so a forgotten flag cannot reintroduce the hang. No `--locked` (stamping desynchronises
`Cargo.lock`).

### `release` — `ubuntu-latest`

`actions/download-artifact` → `softprops/action-gh-release@v2` with `generate_release_notes: true`
and both `.zip`s. Updates an existing release for the tag in place rather than failing (FR-010,
FR-014).

### `homebrew` — `ubuntu-latest`, `needs: [preflight, release]`

1. checkout (for the template)
2. `gh release download "v$VERSION" --pattern 'site-checker-*-apple-darwin.zip'` — **from the
   published release**, not from build artifacts (FR-011)
3. render with both placeholder guards (FR-013 — see
   [install-channel.md](./install-channel.md))
4. clone the tap with `TAP_PUSH_TOKEN`, write `Casks/site-checker.rb`, commit **only if changed**,
   push (FR-012, FR-014, FR-015)

**Contract**: this job is the only writer of the tap, and it cannot run before a release exists.
A missing or renamed asset fails at step 2, leaving the tap untouched.

**Failure semantics summary**

| Condition | Fails at | Published state |
|---|---|---|
| Malformed tag | `preflight` (seconds) | Nothing |
| Missing secret | `preflight` (seconds) | Nothing |
| Red test | `test` | Nothing |
| Signing/notarization failure | `build` | Nothing |
| Missing asset | `homebrew` step 2 | Release exists; tap untouched |
| Template/renderer drift | `homebrew` step 3 | Release exists; tap untouched |
| Re-run of a green release | — | Converges: one release, tap commit only if content changed |

---

## `verify-install-channels.yml` — Channel Verification (US4)

**Triggers** (FR-019, all three cases):

```yaml
on:
  schedule:
    - cron: '23 9 * * *'
  workflow_run:
    workflows: ["Release"]
    types: [completed]
  workflow_dispatch:
```

**Permissions**: `contents: read`

### `verify` — `ubuntu-latest`

Every step compares against the **same** resolved tag, so one channel lagging another fails by
construction.

1. **Resolve latest `v*` release** — `gh release list --json tagName`, selecting the first
   matching `^v[0-9]`. Errors when none exists.
2. **Assets exist (2/2)** — `curl -sfIL` each expected download URL; accumulate failures, exit
   once (FR-020).
3. **Tap cask matches** — fetch
   `raw.githubusercontent.com/clintcparker/homebrew-tap/main/Casks/site-checker.rb` and
   `grep -qF "version \"$L\""` (FR-020).

### `e2e` — `macos-latest`, `if: github.event_name != 'schedule'`

4. `brew install clintcparker/tap/site-checker`
5. `spctl -a -t exec -vv "/Applications/Site Checker.app"` — must report `accepted` /
   `source=Notarized Developer ID` (FR-021)
6. Assert `CFBundleShortVersionString` equals the resolved version

**Why step 4–6 skip the cron**: a macOS runner performing a real install daily is the most
expensive thing in this feature, and what it catches cannot change between releases. Steps 1–3
run daily in seconds on Ubuntu and are what bound SC-006 at 24 hours. See research R12.

**No retry loop.** `name-on` retries to absorb NuGet indexing lag; GitHub releases and
`raw.githubusercontent.com` have no equivalent lag, so a retry here would only delay a real
failure.

**Failure semantics**

| Condition | Result |
|---|---|
| No `v*` release exists | Fail, naming `docs/how-to/release.md` |
| An asset 404s | Fail, listing each unreachable URL |
| Tap on an older version | Fail, naming the lagging channel (FR-020) |
| Gatekeeper rejects the installed app | Fail at `spctl` — the notarization gap, caught automatically |

---

## Cross-cutting

| Property | Applies to |
|---|---|
| Least-privilege `permissions:` declared at file scope | all three |
| No step blocks on desktop interaction (SC-009) | all three |
| Secrets referenced by name only, never echoed | `release.yml` |
| macOS runners used only where a macOS toolchain is genuinely required | all three |
</content>
