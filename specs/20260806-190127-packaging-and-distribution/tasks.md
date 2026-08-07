# Tasks: Packaging & Distribution

**Input**: Design documents from `specs/20260806-190127-packaging-and-distribution/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: No unit tests. The specification does not request them and
[plan.md](./plan.md#complexity-tracking) records why: this feature's logic is ~400 lines of
workflow YAML with no pure function to extract. Its verification is **behavioural** instead —
render-time placeholder guards that fail the release in both directions, a CI sentinel guard, and
`verify-install-channels.yml` re-checking the published result. Every task below labelled
**VALIDATE** runs a numbered scenario from [quickstart.md](./quickstart.md); those are this
feature's test suite. The standing gates (`cargo test` 55, `pnpm test` 30,
`cargo clippy -- -D warnings`) must stay green throughout — no application code is touched.

**Organization**: Grouped by user story. Plan phases A–G map onto these phases as noted in each
heading, so the plan's gating (what can land before the R1/R2 answers) is preserved.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1..US6)
- Include exact file paths in descriptions

## Path Conventions

- Repository root of the **feature worktree**:
  `/Users/clint/src/clintcparker/site-checker--20260806-190127-packaging-and-distribution/`
- Repository root of the **primary checkout**: `/Users/clint/src/clintcparker/site-checker/`
- All paths below are relative to the feature worktree **unless the task says otherwise**.

> ⚠️ **`docs/` is gitignored** (`.gitignore:25`). `docs/ROADMAP.md` and
> `docs/superpowers/plans/` edits **must be made in the primary checkout**, not this worktree, or
> they will not survive the merge — this has already happened once to this repository (see
> `docs/ROADMAP.md` §2's preamble). `docs/how-to/release.md` is the exception: T003 makes it
> tracked, so it belongs on the branch like any other file. Tasks that must run in the primary
> checkout are marked **[PRIMARY CHECKOUT]**.

> 📎 **Reference implementation is on disk.** `clintcparker/name-on` is cloned at
> `/Users/clint/src/clintcparker/name-on/`. Read
> `.github/workflows/release-cli.yml`, `.github/workflows/verify-install-channels.yml`,
> `install/homebrew/name-on.rb`, and `docs/how-to/release.md` there before writing the
> equivalents — this feature mirrors that shape deliberately, with the divergences recorded in
> [research.md](./research.md) R2, R3, R6, R12, R13.

---

## Status — 2026-08-06

**32 of 47 complete.** Everything authorable and everything locally verifiable is
done: all three workflows, the cask template, the single version source, the
headless-safe build, `docs/how-to/release.md`, and the record edits. Standing gates
re-run and unmoved — `cargo test` 55, `pnpm test` 30, `cargo clippy -- -D warnings`
clean, `pnpm build` fine.

**The 15 open tasks are open for exactly three reasons, none of them code:**

| Open | Why |
|---|---|
| T001, T002 | Maintainer decisions with cost and irreversibility attached — a public repository and a $99/yr Apple membership. Not the implementer's to make (Two-track split, below). |
| T005–T008 | The one-time credentials those decisions gate. |
| T027, T031, T035, T043–T047 | Validations that require something published, a PR raised on GitHub, or a tag pushed. |
| T017 | Half-done: `ruby -c` passes and the render is placeholder-free, but `brew audit` could not run — see below. |

**Track A landed in full.** Phase 2 (one version, headless-safe build) and Phase 7
(CI) are the ungated "pure wins", and both are complete and verified. They merge
regardless of how T001 and T002 come back.

**Track B is authored but unrun**, exactly as the plan sequences it: `release.yml`,
`verify-install-channels.yml`, and the cask template all exist and are statically
verified (`actionlint` clean, all eight `uses:` pins resolve, every job provably
reaches `preflight`, render guards proven to fail in both directions), but nothing
has been published.

**Do not treat this as done.** Research R10 is explicit that "done" means a real
`v1.0.0` a clean Mac can install, and that if either decision comes back no, the
reduced scope must be redefined deliberately at that moment rather than quietly
degrading into "the YAML merged". `docs/ROADMAP.md` §2 now carries the reduced
scope in that spirit.

### Three findings from implementation, not present in the plan

1. **`build` needed `needs: [preflight, test]`, not `needs: test`** (T021 as
   written). GitHub's `needs` context carries only *direct* dependencies, so
   `needs.preflight.outputs.version` would have been empty and the stamp step
   would have written `version = ""`. Caught by T026's audit, which is what that
   task exists for. The gating property T026 checks is unchanged — every job still
   reaches `preflight`.
2. **`package.json` needed a `packageManager` field.** `pnpm/action-setup@v4` has
   no other version source and fails at the first CI step without it. `actionlint`
   cannot catch this. Added as `pnpm@10.30.3`, matching the local toolchain; it
   does not collide with the `0.0.0` sentinel guard.
3. **`Cargo.lock` had to be committed with the sentinel.** Setting `Cargo.toml`'s
   version to `0.0.0` rewrites the lock's own package entry, and `ci.yml` uses
   `--locked`. Committing the config change without the lock would have made the
   very first CI run fail.

Also confirmed by experiment, contradicting an assumption made while writing
`release.yml`: **`cargo test` does not need `../dist` to exist.** `ci.yml`'s `rust`
job therefore needs no frontend build, and the contract's two-job split stands as
written.

### T017's unfinished half

`brew audit --cask --new` was never run against the rendered cask. Invoking
`brew audit` auto-enables Homebrew's developer mode and installs a vendored gem
bundle; that install was interrupted, leaving Homebrew's Ruby with a `json` gem
whose native extension never built, which breaks every `brew` command on this
machine. Developer mode has been switched back off, but the partial gems remain
and need removing:

```sh
rm -rf /opt/homebrew/Library/Homebrew/vendor/bundle/ruby/4.0.0/gems/{json-2.21.1,prism-1.9.0,racc-1.8.1,base64-0.3.0} \
       /opt/homebrew/Library/Homebrew/vendor/bundle/ruby/4.0.0/extensions/arm64-darwin-20/4.0.0-static/{json-2.21.1,prism-1.9.0,racc-1.8.1} \
       /opt/homebrew/Library/Homebrew/vendor/bundle/ruby/4.0.0/specifications/base64-0.3.0.gemspec
```

The cask itself is not implicated: `ruby -c` reports `Syntax OK`, all three
placeholders substitute, the `#{version}`/`#{arch}` interpolations survive intact
(confirming the substitution-safety claim), and the `zap` path is the literal
`~/Library/Application Support/com.clintparker.site-checker`.

---

## Phase 1: Setup (Repository Prerequisites — plan Phase A)

**Purpose**: The two external decisions and the one-time credentials the release path depends on,
plus the `.gitignore` change that makes FR-024's document committable at all.

**Gating note**: T001 and T002 are **decisions for the maintainer, not work**. Phases 2 and 7
(Foundational, US5/CI) do not depend on them and can proceed while they are open — see
[plan.md](./plan.md#implementation-phasing). Nothing in Phases 3–6 can *run* until T005–T008 are
done, though the files can all be authored first.

- [ ] T001 Obtain the maintainer's explicit answer to R1 — make `clintcparker/site-checker` public — and record it as a **Decision confirmed** line under `## R1` in `specs/20260806-190127-packaging-and-distribution/research.md`. State in the record that the change is effectively irreversible and exposes the full history including every `specs/` document, `CHANGELOG.md`, and all commit messages. If the answer is **no**, stop and apply R1's "Fallback if the answer is keep it private" instead: Phases 2 and 7 still deliver, Phase 4's `homebrew` job is authored behind an `if: false` guard, and FR-001/FR-011/FR-020/FR-021/SC-001/SC-002/SC-004/SC-006 are recorded as deferred in `docs/ROADMAP.md`.
- [ ] T002 Obtain the maintainer's explicit answer to R2 — Apple Developer Program membership, $99/yr — and record it as a **Decision confirmed** line under `## R2` in `specs/20260806-190127-packaging-and-distribution/research.md`. If **yes**, start enrolment immediately (identity verification can take days) and let Phases 2 and 7 proceed in parallel. If **no**, apply R2's fallback: `quarantine: false` plus a `caveats` block in the cask template (T015), FR-021 amended to `codesign -v` only (T031), and the Gatekeeper gap recorded in `docs/ROADMAP.md`.
- [X] T003 Change `.gitignore:25` from the bare `docs/` to the two lines `docs/*` and `!docs/how-to/`, then confirm with `git check-ignore -v docs/how-to/release.md` (expect no match) and `git check-ignore -v docs/ROADMAP.md` (expect still ignored). The exact shape is load-bearing — git cannot re-include a file whose parent directory is excluded (research R8). Do not un-ignore anything else under `docs/`.
- [X] T004 [P] Create the empty destination directories `.github/workflows/`, `install/homebrew/`, and `docs/how-to/` at the worktree root.
- [ ] T005 After T001 confirms: make the repository public with `gh repo edit clintcparker/site-checker --visibility public --accept-visibility-change-consequences`, then verify with `gh repo view clintcparker/site-checker --json isPrivate` (expect `false`).
- [ ] T006 [P] Create a fine-grained personal access token with **Contents: Read and write** on **`clintcparker/homebrew-tap` only** (FR-015, research R15) and set it with `gh secret set TAP_PUSH_TOKEN --repo clintcparker/site-checker`. Verify the scope by listing the token's repository access — it must name exactly one repository.
- [ ] T007 [P] After T002 confirms: export the Developer ID Application certificate as a `.p12`, base64-encode it, and set `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, and `APPLE_SIGNING_IDENTITY` (the certificate common name) with `gh secret set --repo clintcparker/site-checker`.
- [ ] T008 [P] After T002 confirms: create an App Store Connect API key and set `APPLE_API_ISSUER`, `APPLE_API_KEY_ID`, and `APPLE_API_KEY` (the `.p8`, base64-encoded) with `gh secret set --repo clintcparker/site-checker`. Use the API-key trio rather than an app-specific password — research R2 explains why (an app-specific password dies with the Apple ID password).

**Checkpoint**: `docs/how-to/` is committable, the seven secrets named in
[data-model.md](./data-model.md#one-time-setup-credentials) exist, and both external decisions are
recorded rather than assumed.

---

## Phase 2: Foundational (One Version, Headless-Safe Build — plan Phase C)

**Purpose**: Collapse three competing version numbers to one machine-written source, and make the
default build safe to run where no interactive desktop session exists. Both are prerequisites:
US2's `build` job stamps the surviving version, and US5's CI guard asserts the sentinel.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

**Not gated on T001/T002** — this phase is a pure win that merges even if both answers are no.

- [X] T009 Delete the `"version": "0.1.0"` key entirely from `src-tauri/tauri.conf.json` (do not set it to anything). Tauri then falls back to `Cargo.toml` — documented at `tauri-utils-2.9.3/src/config.rs:3612` and proven by T014. This removes a source of truth rather than synchronising it (research R5).
- [X] T010 Change `bundle.targets` in `src-tauri/tauri.conf.json` from `["app", "dmg"]` to `["app"]`, so a build that forgets `--bundles app` still cannot reach `bundle_dmg.sh`'s `osascript` call (FR-016, SC-009, research R4).
- [X] T011 [P] Set `[package].version` in `src-tauri/Cargo.toml` to `0.0.0` and add a comment above it stating that the real value is written by `.github/workflows/release.yml` from the tag and that hand-editing it fails CI (FR-008, research R5).
- [X] T012 [P] Set `version` in `package.json` to `0.0.0`. It never reaches the bundle, but SC-003 counts files carrying a hand-maintained version number, and a stale `0.1.0` here is exactly what gets "helpfully" bumped later.
- [X] T013 **VALIDATE** — quickstart Scenario 1 (FR-016, SC-009): run `pnpm tauri build --bundles app` from the worktree root; expect exit 0, `src-tauri/target/release/bundle/macos/Site Checker.app` present, and `src-tauri/target/release/bundle/dmg/` **absent**. Then run a bare `pnpm tauri build` and confirm the reduced default targets also produce no DMG.
- [X] T014 **VALIDATE** — quickstart Scenario 2 (FR-008, SC-003): confirm `grep -c '"version"' src-tauri/tauri.conf.json` returns 0 and `grep '^version' src-tauri/Cargo.toml` returns `version = "0.0.0"`. Then temporarily set `Cargo.toml`'s version to `9.9.9`, build, and read `CFBundleShortVersionString` out of the produced `Info.plist` with `/usr/libexec/PlistBuddy`; expect `9.9.9`, then `git checkout src-tauri/Cargo.toml`. Do not skip — this is the one R5 assumption that came from reading Tauri's source rather than running it.

**Checkpoint**: exactly one version number exists in the tree, it is an inert sentinel, and no
build path in the repository can hang on Finder automation. User story work can now begin.

---

## Phase 3: User Story 1 - Install Site Checker in one line (Priority: P1) 🎯 MVP

**Goal**: The user-facing half of the feature — a Homebrew **cask** that installs
`Site Checker.app`, keeps `sites.json` on a plain uninstall, and trashes it only on
`--zap`, served by the unchanged advertised command
`brew install clintcparker/tap/site-checker`.

**Independent Test**: Hand-render the template with a real version and two real checksums, then
`ruby -c` it and run `brew audit --cask --new` against the rendered file. Full delivery of this
story additionally needs US2's pipeline to publish assets and Phase 9's real install — see
Dependencies.

**Contract**: [contracts/install-channel.md](./contracts/install-channel.md). **Rationale**:
research R13.

- [X] T015 [US1] Create `install/homebrew/site-checker.rb` with the cask body from [contracts/install-channel.md](./contracts/install-channel.md#template): the `arch arm: "aarch64", intel: "x86_64"` stanza (FR-005), `version "VERSION"`, `sha256 arm: "SHA256_ARM64", intel: "SHA256_X86_64"`, the `url` interpolating `#{version}` and `#{arch}` against the release download path, `app "Site Checker.app"` (FR-002), `uninstall quit: "com.clintparker.site-checker"` (FR-004), and `zap trash: ["~/Library/Application Support/com.clintparker.site-checker"]` (FR-003, SC-010). The data directory appears in **exactly one stanza and it is `zap`** — that split, not discipline, is what satisfies Constitution II. Use `trash:`, never `delete:`.
- [X] T016 [US1] Add the required header comment to `install/homebrew/site-checker.rb`, mirroring `/Users/clint/src/clintcparker/name-on/install/homebrew/name-on.rb` in spirit: that it is a TEMPLATE and must never be copied to the tap by hand, that `.github/workflows/release.yml` is the only thing that renders it, that its destination is `clintcparker/homebrew-tap` → `Casks/site-checker.rb`, and that the render step fails the release when a placeholder is missing here or survives into the output (FR-012).
- [ ] T017 [US1] **VALIDATE** — hand-render `install/homebrew/site-checker.rb` to a scratch path with a plausible version and two 64-hex checksums, then run `ruby -c` on it (expect `Syntax OK`) and `brew audit --cask --new` against it. Confirm zero placeholder tokens survive and the `zap` path is the literal `~/Library/Application Support/com.clintparker.site-checker`.
- [X] T018 [P] [US1] Update `README.md` to lead with `brew install clintcparker/tap/site-checker` as the primary install path, document both `brew uninstall site-checker` (list kept) and `brew uninstall --zap site-checker` (list trashed), and add the one sentence research R13 calls for: a previously hand-built copy shares the same `sites.json`, so the installed app picks up the existing list, and Homebrew refuses rather than overwrites if a hand-built `Site Checker.app` already sits in `/Applications`.

**Checkpoint**: the canonical cask template exists, parses, audits clean, and the README advertises
the command it serves.

---

## Phase 4: User Story 2 - Publish a version by pushing one tag (Priority: P1)

**Goal**: `release.yml` — one annotated `v<MAJOR>.<MINOR>.<PATCH>` tag produces a public release
with two signed, notarized, stapled artifacts and an updated tap entry, with zero file edits.

**Independent Test**: Push a version tag at a known commit; confirm a release exists for that tag
with one artifact per architecture, the tap advertises that exact version, and the installed app
reports it.

**Contract**: [contracts/workflows.md](./contracts/workflows.md#releaseyml--release-us1-us2-us3)
and [contracts/version-and-artifacts.md](./contracts/version-and-artifacts.md). **Reference**:
`/Users/clint/src/clintcparker/name-on/.github/workflows/release-cli.yml`.

- [X] T019 [US2] Create `.github/workflows/release.yml` with `on: push: tags: ['v*']`, file-scope `permissions: contents: write`, and a `preflight` job on `ubuntu-latest` (no checkout, no toolchain) whose first step derives `VERSION="${GITHUB_REF_NAME#v}"`, validates it is exactly three dot-separated non-empty numeric components per [contracts/version-and-artifacts.md](./contracts/version-and-artifacts.md#tag--version), exits non-zero on anything else including `v1.2.3-rc.1` (FR-009), and exposes `version` as a job output.
- [X] T020 [US2] Add the `test` job to `.github/workflows/release.yml` — `needs: preflight`, `macos-latest`, running the same four gates as `ci.yml`. Present because a tag can be pushed at a commit CI never evaluated (FR-023).
- [X] T021 [US2] Add the `build` job to `.github/workflows/release.yml` — `needs: test`, `macos-latest`, `strategy.matrix` over `[aarch64, x86_64]`, per [contracts/workflows.md](./contracts/workflows.md#build--macos-latest-matrix-over-aarch64-x86_64): checkout → `rustup target add <arch>-apple-darwin` → pnpm + node setup → **stamp** `src-tauri/Cargo.toml` with `needs.preflight.outputs.version` → import `APPLE_CERTIFICATE` into a temporary keychain → `pnpm install --frozen-lockfile` → `pnpm tauri build --bundles app --target <arch>-apple-darwin` with `APPLE_SIGNING_IDENTITY`/`APPLE_API_ISSUER`/`APPLE_API_KEY_ID`/`APPLE_API_KEY_PATH` exported so Tauri signs, notarizes, waits, and staples in-band → `ditto -c -k --sequesterRsrc --keepParent "…/Site Checker.app" site-checker-<arch>-apple-darwin.zip` → `actions/upload-artifact`. **`--bundles app` is mandatory** (FR-016) and **`--locked` must not be used** (stamping desynchronises `Cargo.lock`, research R5). Staple the `.app` *before* zipping — a ticket stapled to a zip does not survive extraction.
- [X] T022 [US2] Add the `release` job to `.github/workflows/release.yml` — `needs: build`, `ubuntu-latest`, `actions/download-artifact` then `softprops/action-gh-release@v2` with `generate_release_notes: true` and both `.zip`s attached. It must **update an existing release for the tag in place** rather than failing (FR-010, FR-014, SC-007).
- [X] T023 [US2] Add the `homebrew` job to `.github/workflows/release.yml` — `needs: [preflight, release]`, `ubuntu-latest`: checkout for the template → `gh release download "v$VERSION" --pattern 'site-checker-*-apple-darwin.zip'` **from the published release** (never from build artifacts — FR-011) → `sha256sum` those downloaded bytes → run the two render guards and the `sed` substitution from [contracts/install-channel.md](./contracts/install-channel.md#render-guards-fr-013--both-directions) → clone `clintcparker/homebrew-tap` with `TAP_PUSH_TOKEN`, write `Casks/site-checker.rb`, and commit with message `site-checker <VERSION>` **only when `git status --porcelain` is non-empty**, logging `Cask unchanged — skipping commit` otherwise (FR-012, FR-014, FR-015, research R14).
- [X] T024 [US2] **VALIDATE** — quickstart Scenario 4 (FR-013, both directions): run the render step's logic locally against `install/homebrew/site-checker.rb` with fake checksums. Direction 1 — delete `SHA256_X86_64` from the template and expect `::error::placeholder SHA256_X86_64 missing from …` with a non-zero exit. Direction 2 — restore the template, drop the `SHA256_X86_64` case from the `sed` script, and expect `::error::placeholder SHA256_X86_64 survived rendering …` with a non-zero exit. Both must be hard failures, never warnings.

**Checkpoint**: the whole tag → release → tap path exists in one file, its render guards are proven
to fail in both directions, and nothing writes the tap except this job.

---

## Phase 5: User Story 3 - Be told what one-time setup is missing, immediately (Priority: P2)

**Goal**: A release with a missing credential dies in seconds naming the gap, having built nothing
and published nothing.

**Independent Test**: Temporarily remove one required secret, push a throwaway tag, and confirm the
run fails in under 60 seconds naming that secret, with no release object and an untouched tap.

- [X] T025 [US3] Add the one-time-setup probe as a second step of the `preflight` job in `.github/workflows/release.yml`: check the **presence** of `TAP_PUSH_TOKEN`, `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_API_ISSUER`, `APPLE_API_KEY_ID`, and `APPLE_API_KEY`. **Accumulate every missing name and exit once**, so a first-time setup reports all gaps in one run rather than one per attempt; emit `::error::one-time setup missing: <NAME> (see docs/how-to/release.md)` per gap (FR-017, research R9). Never echo a secret's value.
- [X] T026 [US3] Audit `.github/workflows/release.yml` and confirm every job reaches `preflight` through its `needs:` chain (`test` → `preflight`, `build` → `test`, `release` → `build`, `homebrew` → `[preflight, release]`), so a pre-flight failure means nothing was built, no release object exists, and the tap was not touched (FR-018, SC-005). Record the verified chain against the diagram in [data-model.md](./data-model.md#the-release-state-machine).
- [ ] T027 [US3] **VALIDATE** — quickstart Scenario 5 (FR-017, FR-018, SC-005): with the repository public and secrets set, `gh secret delete TAP_PUSH_TOKEN` temporarily, push the throwaway tag `v0.0.1`, and `gh run watch`. Expect failure in under 60 seconds naming `TAP_PUSH_TOKEN` and pointing at `docs/how-to/release.md`, with no release object created and the tap unchanged. Then delete the tag, delete any release it produced, and restore the secret. Separately push `v0.0.1-preflight-test` to confirm the version-validation gate rejects it before the secret probe matters.

**Checkpoint**: missing setup is a fast, named, harmless failure rather than a half-published
release.

---

## Phase 6: User Story 4 - Catch a stale or broken install channel automatically (Priority: P2)

**Goal**: `verify-install-channels.yml` — daily, after every release, and on demand, the project
checks that what it advertises is what it actually serves.

**Independent Test**: Dispatch it against the current published version and expect green; point the
tap at an older version and expect a failure naming the lagging channel.

**Reference**: `/Users/clint/src/clintcparker/name-on/.github/workflows/verify-install-channels.yml`
— mirror it with the channel set reduced to one (no NuGet) and the end-to-end step changed from
`dotnet tool install` to `brew install` + `spctl` (research R12).

- [X] T028 [US4] Create `.github/workflows/verify-install-channels.yml` with all three triggers — `schedule: cron '23 9 * * *'`, `workflow_run` on the Release workflow's `completed`, and `workflow_dispatch` (FR-019) — plus file-scope `permissions: contents: read`.
- [X] T029 [US4] Add the `verify` job (`ubuntu-latest`) to `.github/workflows/verify-install-channels.yml` with steps 1–3 from [contracts/workflows.md](./contracts/workflows.md#verify--ubuntu-latest), every one comparing against the **same** resolved tag: resolve the latest `v*` release via `gh release list --json tagName` selecting the first `^v[0-9]` and erroring when none exists; `curl -sfIL` both expected asset download URLs, accumulating failures and exiting once; and `grep -qF "version \"$L\""` against `raw.githubusercontent.com/clintcparker/homebrew-tap/main/Casks/site-checker.rb` (FR-020). **No retry loop** — research R12 explains why `name-on`'s NuGet-lag retry has no analogue here.
- [X] T030 [US4] Add the `e2e` job (`macos-latest`, `if: github.event_name != 'schedule'`) to `.github/workflows/verify-install-channels.yml`: `brew install clintcparker/tap/site-checker`, then `spctl -a -t exec -vv "/Applications/Site Checker.app"` asserting `accepted` and `source=Notarized Developer ID` (FR-021), then assert `CFBundleShortVersionString` equals the resolved version. If T002 came back **no**, this step is the amended `codesign -v` check instead, and the Gatekeeper gap is recorded rather than checked.
- [ ] T031 [US4] **VALIDATE** — quickstart Scenario 8 (FR-019, FR-020, SC-006): `gh workflow run verify-install-channels.yml` and expect green against the current release. Then hand-edit `Casks/site-checker.rb` in the tap to an older `version` and re-dispatch; expect a failure naming the tap as the lagging channel. Restore by re-running `release.yml`'s `homebrew` job — which also proves the tap is machine-recoverable and never needs hand-editing.

**Checkpoint**: a green release with a broken install is now caught within 24 hours, automatically.

---

## Phase 7: User Story 5 - Every push is checked automatically (Priority: P3)

**Goal**: `ci.yml` — the four gates the constitution names by hand run on every push to `main` and
every pull request, so "green before merge" stops depending on run discipline.

**Independent Test**: Open a PR containing a deliberately failing test; confirm the checks report
failure with no local action, then remove it and confirm green.

**Not gated on T001/T002 in authoring** (though runs cost private-repo minutes until T005).

- [X] T032 [US5] Create `.github/workflows/ci.yml` with `on: push: branches: [main]` and `on: pull_request`, file-scope `permissions: contents: read`, and a `rust` job on **`macos-latest`**: checkout → `dtolnay/rust-toolchain@stable` with the `clippy` component → `Swatinem/rust-cache` → `cargo test --locked --manifest-path src-tauri/Cargo.toml` → `cargo clippy --locked --manifest-path src-tauri/Cargo.toml -- -D warnings`. macOS is required, not preferred — `src-tauri` links macOS system frameworks through `tauri` 2.11 (research R11). Use `--locked` here (unlike `release.yml`, this job does not stamp).
- [X] T033 [US5] Add the version-sentinel guard as a final step of the `rust` job in `.github/workflows/ci.yml`: assert `src-tauri/Cargo.toml` still matches `^version = "0.0.0"` and `package.json` still carries `0.0.0`, failing with `::error::src-tauri/Cargo.toml version must stay 0.0.0 — the release tag is the source of truth`. This step is what turns FR-008 from a convention into a guarantee and is the cheapest check in the feature (research R5 step 5, SC-003).
- [X] T034 [P] [US5] Add the `frontend` job to `.github/workflows/ci.yml` on `ubuntu-latest`: checkout → `pnpm/action-setup` → `actions/setup-node` with the pnpm cache → `pnpm install --frozen-lockfile` → `pnpm test` → `pnpm build`. Deliberately **no `pnpm tauri build`** — FR-022 names four gates and a full bundle is not one; adding it would drag the DMG hang and the signing credentials into every PR (research R11).
- [ ] T035 [US5] **VALIDATE** — quickstart Scenario 3 (FR-008, SC-008): on a scratch branch set `src-tauri/Cargo.toml`'s version to `1.0.0` and open a PR; expect the `rust` job to fail at the sentinel guard with the message naming the tag as the source of truth. Revert and expect green. Confirm all four gates report as separate, readable PR checks.

**Checkpoint**: every PR carries an automated pass/fail for all four constitutional gates.

---

## Phase 8: User Story 6 - The release procedure is written down (Priority: P3)

**Goal**: One document that says how to release, what the one-time setup is, and what to do when a
release fails partway through.

**Independent Test**: Hand it to someone who has never released this project and have them perform
a release using only it.

**Reference**: `/Users/clint/src/clintcparker/name-on/docs/how-to/release.md`.

- [X] T036 [US6] Create `docs/how-to/release.md` covering: **the procedure** (push one annotated `v<MAJOR>.<MINOR>.<PATCH>` tag; there is nothing else — FR-007, SC-003); **the one-time setup checklist** (the seven secrets from [data-model.md](./data-model.md#one-time-setup-credentials), how each is produced, and `TAP_PUSH_TOKEN`'s exact fine-grained scope); **re-running a failed release** (the release updates in place, the tap commits only when content changed, and the honest caveat from research R14 that a re-run rebuilds and Tauri is not bit-reproducible, so checksums *do* change and the tap *will* commit — "unchanged content → no commit" is not "re-running is a no-op"); **the pre-flight's honest limit** (presence ≠ validity — an expired certificate, a revoked API key, or a mis-scoped token passes pre-flight and fails later, at signing, at `notarytool submit`, or at the tap push — research R9); and **the local DMG note** (nothing distributes a DMG any more; `pnpm tauri build --bundles dmg` on your own Mac if you want one — research R4).
- [X] T037 [US6] Confirm `docs/how-to/release.md` is actually tracked: `git check-ignore -v docs/how-to/release.md` must report no match and `git status --short` must show it as a new file. If it is still ignored, T003 was not applied in the shape research R8 requires.

**Checkpoint**: SC-011 is satisfiable — the procedure exists, is committed, and appears in the PR.

---

## Phase 9: Polish, Records & First Real Release (plan Phases F and G)

**Purpose**: The document amendments the feature owes, the standing gates, and the release that
makes FR-027 answerable. **Phase G is inside this feature, not after it** (research R10) — a
release pipeline that has never run is a plausible-looking YAML file, and the exact failure class
this feature exists to prevent is invisible until the first real install.

- [X] T038 [P] **[PRIMARY CHECKOUT]** Amend the size expectation at `/Users/clint/src/clintcparker/site-checker/docs/superpowers/plans/2026-07-23-site-checker.md:2554` (FR-026, SC-012): replace *"Expected: single-digit MB. A much larger number means something pulled in an unexpected dependency — worth investigating before shipping"* with the measured ~15 MB and one sentence recording that the cause was investigated and attributed to `aws-lc-rs` via reqwest 0.13's `default-tls`. Leave `CHANGELOG.md:327` alone — it records the actual size as history, which is a fact, not a claim. Do **not** change the TLS backend (research R7).
- [X] T039 [P] **[PRIMARY CHECKOUT]** Drain §2 of `/Users/clint/src/clintcparker/site-checker/docs/ROADMAP.md` (FR-025): record what shipped in the style §§1–3 already use, name this feature's directory, and state plainly anything deliberately left — in particular whichever of R1/R2's fallbacks was taken, if either. Renumber the sections that follow, consistent with how previous drains did it.
- [X] T040 [P] Add a release entry to `CHANGELOG.md` at the worktree root following the file's existing convention.
- [X] T041 Validate all three workflow files parse as YAML and reference only actions that exist — `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/workflows/verify-install-channels.yml`. Run `actionlint` if available; otherwise parse each with a YAML loader and confirm every `uses:` pin resolves.
- [X] T042 Run the standing constitutional gates from the worktree root and record the counts: `cargo test` (expect 55), `pnpm test` (expect 30), `cargo clippy -- -D warnings` (expect clean). No application code was touched, so any movement here is a regression to investigate before shipping.
- [ ] T043 Merge the PR, then push the first real tag: `git tag -a v1.0.0 -m "site-checker 1.0.0"` and `git push origin v1.0.0`, then `gh run watch`. Expect a public release for `v1.0.0` carrying both `site-checker-aarch64-apple-darwin.zip` and `site-checker-x86_64-apple-darwin.zip` plus generated notes, and a tap commit `site-checker 1.0.0` creating `Casks/site-checker.rb`.
- [ ] T044 **VALIDATE** — quickstart Scenario 6, on a Mac (or fresh user account) that has never built this project: `brew install clintcparker/tap/site-checker`, then `spctl -a -t exec -vv "/Applications/Site Checker.app"` (expect `accepted` / `source=Notarized Developer ID`), then `CFBundleShortVersionString` via `PlistBuddy` (expect `1.0.0`). Open the app and confirm no security dialog and no right-click-open (SC-002). **Time the whole thing** — SC-001 claims under 5 minutes. Paste the `spctl` output into the ship record; it is the artifact worth keeping.
- [ ] T045 **VALIDATE** — quickstart Scenario 6's uninstall half (FR-003, FR-004, SC-010): add a site and quit, `brew uninstall site-checker`, confirm `~/Library/Application Support/com.clintparker.site-checker/sites.json` is **present**; re-install and confirm the sites are restored; then `brew uninstall --zap site-checker` and confirm the directory is **gone**.
- [ ] T046 **VALIDATE** — quickstart Scenario 7 (FR-014, SC-007): re-run the completed `v1.0.0` release from the Actions UI and confirm exactly one release for `v1.0.0` and exactly one `Casks/site-checker.rb`. Then re-run **only** the `homebrew` job and confirm it logs `Cask unchanged — skipping commit`, which is the path that actually exercises the skip.
- [ ] T047 Confirm the post-release `verify-install-channels.yml` run (triggered by `workflow_run`) went green, including its `e2e` job, and record the result alongside the gate results in the ship record.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup / plan A)**: T003 and T004 have no dependencies. T005 needs T001; T007 and T008 need T002. T006 is independent.
- **Phase 2 (Foundational / plan C)**: no dependencies on Phase 1 — **this is deliberate**. Phases 2 and 7 are the "pure wins" that merge even if T001 and T002 both come back no.
- **Phase 3 (US1)**: needs Phase 2's checkpoint for a coherent build, but the template itself only needs T004.
- **Phase 4 (US2)**: needs Phase 2 (stamping requires the sentinel) and Phase 3 (the `homebrew` job renders the template). **Authoring** is not gated on Phase 1; **running** is.
- **Phase 5 (US3)**: needs T019 (the `preflight` job must exist to add a step to it).
- **Phase 6 (US4)**: needs Phase 4 — it verifies what that pipeline publishes.
- **Phase 7 (US5)**: needs Phase 2 (T033's guard asserts the sentinel T011/T012 create). Independent of Phases 3–6.
- **Phase 8 (US6)**: needs T003 (or the file is invisible) and documents Phases 4–7, so it is written last among the authoring phases.
- **Phase 9**: T038–T042 need the preceding phases; T043–T047 need all of them plus Phase 1 complete.

### User Story Dependencies

- **US1 (P1)**: the template is independently authorable and locally testable (T017). Its *user-facing outcome* needs US2's pipeline and T043–T045 — this is the one place the stories are genuinely coupled, and [research.md](./research.md) R10 says so plainly.
- **US2 (P1)**: needs Foundational and US1's template. Independently testable once Phase 1's credentials exist.
- **US3 (P2)**: extends US2's `preflight` job. Not independently deliverable — it is a step inside a job US2 creates.
- **US4 (P2)**: verifies US2's output. Independently testable by dispatch once a release exists.
- **US5 (P3)**: fully independent of US1–US4. Could ship first and alone.
- **US6 (P3)**: documents US2–US5. Independently readable; its test (SC-011) needs the pipeline to exist.

### Within Each User Story

- Workflow files: create the file and its trigger/permission block before adding jobs; add jobs in `needs:` order so the graph is always valid.
- The cask template: body before header before validation.
- Every **VALIDATE** task runs last within its phase.

### Parallel Opportunities

- **Phase 1**: T004 with T006; T007 with T008 (different secrets, same gate).
- **Phase 2**: T011 with T012 (`Cargo.toml` and `package.json` are different files). T009 and T010 both edit `tauri.conf.json` — sequential.
- **Phase 3**: T018 (`README.md`) runs parallel to T015–T017 (`install/homebrew/site-checker.rb`).
- **Phase 7**: T034 (`frontend` job) is parallel-safe against T032/T033 only if the file already exists — create it in T032 first.
- **Phase 9**: T038, T039, T040 are three different files in two different checkouts.
- **Across phases**: Phase 7 (US5, CI) can be built entirely in parallel with Phases 3–6 by a second person, and merged separately.

---

## Parallel Example: Phase 2 (Foundational)

```bash
# T009 and T010 touch the same file — run them in sequence:
Task: "Delete the version key from src-tauri/tauri.conf.json"
Task: "Reduce bundle.targets to [\"app\"] in src-tauri/tauri.conf.json"

# T011 and T012 are different files — run them together:
Task: "Set [package].version to 0.0.0 in src-tauri/Cargo.toml with the explanatory comment"
Task: "Set version to 0.0.0 in package.json"
```

## Parallel Example: Phase 9 (Records)

```bash
# Three files, two checkouts — all independent:
Task: "Amend the size expectation in /Users/clint/src/clintcparker/site-checker/docs/superpowers/plans/2026-07-23-site-checker.md"
Task: "Drain §2 of /Users/clint/src/clintcparker/site-checker/docs/ROADMAP.md"
Task: "Add the release entry to CHANGELOG.md"
```

---

## Implementation Strategy

### MVP First

The honest MVP here is **not** User Story 1 alone — a cask template that nothing renders installs
nothing. The smallest thing that delivers the section's point is
**Phase 2 + Phase 3 + Phase 4 + Phase 9's T043–T045**: one version source, a cask template, the
release pipeline, and a real `v1.0.0` that a clean Mac can install. That is FR-027's definition of
done, and everything else hardens it.

### The two-track split, and why it matters

T001 and T002 are decisions with cost and irreversibility attached, and neither is the
implementer's to make. The plan is deliberately sequenced so waiting on them costs nothing:

- **Track A (ungated, start now)**: Phase 2 → Phase 7. One version, headless-safe build, full CI.
  Merges and delivers value regardless of how T001 and T002 land.
- **Track B (gated on T001/T002)**: Phase 3 → Phase 4 → Phase 5 → Phase 6 → Phase 8 → Phase 9.
  The files can all be **authored** while the decisions are open; only the runs are blocked.

If either answer is no, apply that research item's recorded fallback and **redefine done
deliberately, at that moment**, recording the reduced scope in `docs/ROADMAP.md`. Research R10 is
explicit that it must not quietly degrade into "the YAML merged".

### Incremental Delivery

1. Phase 1 (decisions + credentials) — start T002's enrolment first; it is the longest lead time.
2. Phase 2 → checkpoint: one version, no DMG hang. Mergeable alone.
3. Phase 7 (US5) → checkpoint: every PR checked. Mergeable alone.
4. Phase 3 + Phase 4 + Phase 5 → checkpoint: the pipeline exists and its guards are proven locally.
5. Phase 6 (US4) → checkpoint: the published result is watched.
6. Phase 8 (US6) → checkpoint: the procedure is written down.
7. Phase 9 → merge, tag `v1.0.0`, install on a clean Mac. **This is where the feature becomes true.**

---

## Notes

- **No unit tests by design.** See the Tests note at the top and
  [plan.md](./plan.md#complexity-tracking): the guarantees are enforced at run time in the release
  path (both render guards, checksums from the real assets, the CI sentinel) plus daily
  verification, which is stronger than a unit test of a `sed` script would have been.
- **Secrets are referenced by name only and never echoed** — applies to every step in
  `release.yml`.
- **`--locked` in `ci.yml`, never in `release.yml`.** Stamping `Cargo.toml` desynchronises
  `Cargo.lock` in the runner's working copy; that change is never committed (research R5).
- **Placeholder substitution safety is load-bearing.** No placeholder is a substring of another,
  so `sed` ordering does not matter. A future edit that breaks this silently breaks the render —
  see [contracts/install-channel.md](./contracts/install-channel.md#placeholders).
- **The `zap`/`uninstall` split is the mechanism, not the convention.** The data directory appears
  in exactly one stanza. This is the first mechanism in the project's history that can delete the
  user's site list, and Constitution II is satisfied by that split rather than by discipline.
- **Zero changes to `src/` or `src-tauri/src/`.** If a task appears to require one, stop — it is
  out of scope per the spec, and the constitution's "One Mac, One Person" is unchanged by this
  feature.
- Commit after each task or logical group. Stop at any checkpoint to validate independently.
