# Implementation Plan: Packaging & Distribution

**Branch**: `20260806-190127-packaging-and-distribution` | **Date**: 2026-08-06 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/20260806-190127-packaging-and-distribution/spec.md`

## Summary

Give Site Checker a published install path — `brew install clintcparker/tap/site-checker` —
driven entirely by pushing one annotated `v<MAJOR>.<MINOR>.<PATCH>` tag, and back it with the
three workflows `name-on` already proves: a tag-triggered release pipeline whose pre-flight
fails in seconds when one-time setup is missing, a channel-verification pass on cron plus
post-release, and ordinary per-push CI. Nothing about the application's behaviour changes.

The work is almost entirely new files under `.github/workflows/`, `install/homebrew/`, and
`docs/how-to/`, plus three small edits to existing build configuration (delete the duplicated
version, stamp the surviving one from the tag, keep the DMG target out of headless builds).

**Three findings shape this plan and are not in the spec:**

1. **The repository is private.** A Homebrew cask cannot download a release asset from a
   private repository — `brew` is unauthenticated, so every install would 404. FR-001 is
   unreachable until `clintcparker/site-checker` is public. See [research.md R1](./research.md).
2. **`docs/` is gitignored** (`.gitignore:25`). FR-024's `docs/how-to/release.md` would never
   be committed, and FR-026's stated size expectation lives in a file no PR can show. Both need
   a `.gitignore` change. See [research.md R8](./research.md).
3. **`spctl -a -t exec` (FR-021) can only pass on a notarized bundle**, which settles FR-006 in
   favour of Developer ID signing + notarization — an Apple Developer Program membership
   ($99/yr) the project does not have today. See [research.md R2](./research.md).

All three [NEEDS CLARIFICATION] markers are resolved in `research.md`; the two that carry cost
or an irreversible disclosure decision (R1 repo visibility, R2 Apple membership) are flagged for
explicit confirmation before implementation starts, and the plan sequences the work so
everything not gated on them can land first.

## Technical Context

**Language/Version**: Rust 1.x (edition 2021) + TypeScript 5.6; the new work is GitHub Actions
YAML and POSIX shell, plus one Ruby DSL file (a Homebrew cask).

**Primary Dependencies**: Tauri 2.11 (bundler, macOS signing/notarization driver), pnpm + Vite 6,
`reqwest` 0.13 with `default-tls` (which resolves to rustls + `rustls-platform-verifier` +
`aws-lc-rs` — confirmed by `cargo tree`, and the reason for the 15 MB bundle).
New: `softprops/action-gh-release@v2`, `actions/checkout@v4`, `actions/upload-artifact@v4`,
`actions/download-artifact@v4`, `dtolnay/rust-toolchain`, `pnpm/action-setup`.

**Storage**: unchanged — `~/Library/Application Support/com.clintparker.site-checker/sites.json`.
This feature only gains the ability to *delete* it, via the cask's opt-in `zap trash`.

**Testing**: `cargo test` (55), `pnpm test` (30), `cargo clippy -- -D warnings`. This feature adds
no unit tests — its logic lives in CI YAML. Its verification is behavioural instead: render-time
placeholder guards that fail the release in both directions, and `verify-install-channels.yml`.

**Target Platform**: macOS on Apple Silicon (`aarch64-apple-darwin`) and Intel
(`x86_64-apple-darwin`). Build host: GitHub-hosted `macos-latest` (arm64), cross-compiling the
Intel slice — see [research.md R6](./research.md).

**Project Type**: Desktop application (Tauri). This feature is release engineering around it.

**Performance Goals**: SC-005 — a missing credential fails the run in under 60 seconds, before any
build. SC-001 — install to running app in under 5 minutes on a typical connection.

**Constraints**:
- FR-016 / SC-009: no step may block on desktop GUI automation. `pnpm tauri build`'s
  `bundle_dmg.sh` calls `osascript` against the Finder and hangs headless — CI must not build the
  `dmg` target.
- FR-015: the tap-writing credential is scoped to `clintcparker/homebrew-tap` only.
- FR-008 / SC-003: zero hand-maintained version numbers survive in the repository.
- Constitution II: the plain uninstall must leave `sites.json` intact.

**Scale/Scope**: 3 new workflow files, 1 cask template, 1 how-to document, ~4 edited files, 0
changes to `src/` or `src-tauri/src/`.

## Constitution Check

*GATE: evaluated before Phase 0 and re-evaluated after Phase 1 design. Both passes recorded.*

| Principle | Verdict | Notes |
|---|---|---|
| **I. One Mac, One Person** | PASS | The product's answer to its one question is unchanged. Distribution is not scope: no alerting, no history, no sync, no multi-machine anything. The spec's Out of Scope already refuses auto-update, which is the one place packaging could have leaked into product scope. |
| **II. Results Are Ephemeral, Config Is Sacred** | PASS, with a named new power | This feature introduces the *first* mechanism that can delete `sites.json`: the cask's `zap trash`. It is opt-in (`brew uninstall --zap`), never runs on a plain uninstall (FR-004), and names exactly one directory — the one the constitution says the app owns. Nothing else in this feature reads or writes the store. The design contract for this is in [contracts/install-channel.md](./contracts/install-channel.md). |
| **III. Be a Polite Client** | PASS | No request behaviour changes. `verify-install-channels.yml` issues a handful of `curl -sfIL` HEAD requests per day against `github.com` and `raw.githubusercontent.com`, which is well inside ordinary use of those hosts. |
| **IV. Testable Core, Thin Shell** | PASS, with a deviation logged below | No pure logic is added to `model.rs`/`check.rs`/`store.rs`, and no shell grows. The deviation is that this feature's *own* logic is not `cargo test`-able at all. See Complexity Tracking. |
| **V. The Rust/TS Contract Is snake_case** | PASS | No persisted or event field name is touched. The only serialized change is deleting `version` from `tauri.conf.json`, which is build configuration, not the data contract. |
| **Quality Gates** | STRENGTHENED | The four gates the constitution names by hand are exactly what US5 automates. After this, "green before merge" stops depending on run discipline. |
| **Development Workflow** | PASS | Worktree + feature branch + PR against `main`, as here. |

**Post-Phase-1 re-evaluation**: unchanged. The Phase 1 design added no application code and no new
data location; the one constitutional pressure point (II) is bounded by the cask contract, which
pins `zap` to the single owned directory and forbids it in the `uninstall` stanza.

## Project Structure

### Documentation (this feature)

```text
specs/20260806-190127-packaging-and-distribution/
├── plan.md                        # This file
├── spec.md                        # Input
├── research.md                    # Phase 0 — R1..R15, all NEEDS CLARIFICATION resolved
├── data-model.md                  # Phase 1 — entities and the release state machine
├── quickstart.md                  # Phase 1 — how to validate this without publishing junk
├── contracts/
│   ├── version-and-artifacts.md   # tag → version → artifact names, formats, checksums
│   ├── install-channel.md         # cask template placeholders + rendered-entry contract
│   └── workflows.md               # triggers, jobs, secrets, failure semantics
├── checklists/requirements.md     # From /speckit-specify
└── tasks.md                       # Phase 2 — NOT created by /speckit-plan
```

### Source Code (repository root)

```text
.github/
└── workflows/
    ├── ci.yml                      # NEW — US5: 4 gates on push-to-main + every PR
    ├── release.yml                 # NEW — US2/US3: preflight → test → build matrix
    │                               #        → release → homebrew
    └── verify-install-channels.yml # NEW — US4: cron + workflow_run + dispatch

install/
└── homebrew/
    └── site-checker.rb             # NEW — cask TEMPLATE (never copied to the tap by hand)

docs/
└── how-to/
    └── release.md                  # NEW — US6; requires the .gitignore change (R8)

.gitignore                          # EDIT — un-ignore docs/how-to/ so the above is committable
README.md                           # EDIT — lead with the brew install line
CHANGELOG.md                        # EDIT — release entry (existing convention)
src-tauri/tauri.conf.json           # EDIT — delete "version"; drop "dmg" from bundle targets
src-tauri/Cargo.toml                # EDIT — version → 0.0.0 sentinel, comment says why
package.json                        # EDIT — version → 0.0.0 sentinel (inert for the bundle)
docs/superpowers/plans/2026-07-23-site-checker.md   # EDIT — FR-026, the size expectation
docs/ROADMAP.md                     # EDIT — FR-025, record what shipped
```

**Structure Decision**: This feature adds no source directories. It mirrors `name-on`'s proven
layout one-for-one — `install/homebrew/<name>.rb` as the canonical template, `.github/workflows/`
for the pipeline, `docs/how-to/release.md` for the procedure — with a single deliberate
divergence, `Casks/site-checker.rb` instead of `Formula/name-on.rb` in the tap, because Site
Checker ships an application bundle rather than a command-line binary (FR-002). Application code
under `src/` and `src-tauri/src/` is untouched.

**A note on where these files must be written.** `docs/` is gitignored in this checkout, and this
repository has already lost one roadmap edit that way (see the "Note on this file's history" in
`docs/ROADMAP.md` §2's preamble). The `docs/ROADMAP.md` and `docs/superpowers/plans/` edits above
must be applied in the **primary checkout**, not in this worktree, or they will not survive the
merge. `docs/how-to/release.md` is different — the `.gitignore` change makes it tracked, so it
belongs on the branch like any other file.

## Implementation Phasing

Ordered so that everything not gated on an external decision lands first. Each phase is
independently mergeable and independently verifiable.

| Phase | Delivers | Stories / FRs | Gated on |
|---|---|---|---|
| **A. Repository prerequisites** | `.gitignore` un-ignores `docs/how-to/`; repo made public; `TAP_PUSH_TOKEN` created and set; Apple Developer enrolment + certificate + App Store Connect API key stored as secrets | FR-015, FR-024's viability | **R1 and R2 confirmations** |
| **B. Continuous integration** | `ci.yml` — `cargo test`, `cargo clippy -- -D warnings`, `pnpm test`, `pnpm build` on push-to-`main` and every PR | US5 · FR-022, FR-023, SC-008 | nothing |
| **C. One version, headless-safe build** | Delete `version` from `tauri.conf.json`; `Cargo.toml`/`package.json` → `0.0.0` sentinel; drop `dmg` from the default bundle targets; `ci.yml` guard that the sentinel is intact | FR-008, FR-016, SC-003, SC-009 | nothing |
| **D. Release pipeline + install channel** | `install/homebrew/site-checker.rb`; `release.yml` (preflight → test → build matrix → release → homebrew) | US1, US2, US3 · FR-001..FR-018 | A |
| **E. Channel verification** | `verify-install-channels.yml` — cron, `workflow_run`, `workflow_dispatch`; asset reachability, tap-version match, `brew install` + `spctl` end-to-end | US4 · FR-019..FR-021, SC-006 | D |
| **F. Documentation & records** | `docs/how-to/release.md`; README install section; the FR-026 size amendment; `docs/ROADMAP.md` §2 drained; CHANGELOG entry | US6 · FR-024, FR-025, FR-026, SC-011, SC-012 | B–E (documents what they do) |
| **G. First real release** | Push `v1.0.0`; verify `brew install clintcparker/tap/site-checker` on a clean Mac | FR-027's answer, SC-001, SC-002, SC-004 | A–F |

Phases B and C are pure wins with no external dependency and no cost; they can merge even if R1
and R2 come back "no". Phase G is what makes FR-027 answerable — see [research.md R10](./research.md).

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| **Principle IV: this feature's logic is not unit-testable.** ~400 lines of workflow YAML and inline shell, exercised only by running them. | Release automation is inherently a description of what a hosted runner does; there is no pure function to extract. The alternative is not "testable release automation", it is "no release automation", which is the roadmap item itself. | Extracting the render step into a testable script was considered and rejected: it would move ~15 lines of `sed` out of the workflow while leaving the other ~385 untested, and would add a second place for the template and the renderer to drift apart — the exact failure FR-013 exists to prevent. Instead the guarantees are enforced *at run time, in the release path*: the render step fails when a placeholder the renderer expects is missing from the template **and** when any placeholder survives into the output (FR-013), the tap is built from assets actually attached to the release rather than predicted names (FR-011), and `verify-install-channels.yml` re-checks the published result daily and after every release (FR-019–FR-021). Those are behavioural tests that run against the real artifact, which is stronger than a unit test of the renderer would have been. |
| **Three workflow files rather than one.** | `name-on`'s proven split, and FR-023 requires it: ordinary CI must be separate from the release pipeline, and channel verification must run on a schedule independent of both. | One combined workflow was rejected because a cron trigger and a tag trigger in one file means every scheduled run evaluates release jobs it must skip, and a red channel check would be indistinguishable from a red release in the run list. |
