# Phase 0 Research: Packaging & Distribution

Every decision below was checked against something real — the local `name-on` checkout at
`~/src/name-on`, this repository's actual files, `cargo tree`, the
Tauri 2.9.3 config source in the local Cargo registry, or the GitHub API. Where a claim is
inherited rather than verified, it says so.

The spec's three `[NEEDS CLARIFICATION]` markers are resolved in **R2** (FR-006), **R7**
(FR-026), and **R10** (FR-027). Two further blockers the spec did not know about are resolved
in **R1** and **R8**.

---

## R1. The repository is private, and a Homebrew cask cannot install from a private repository

**BLOCKER for FR-001. Not raised in the spec.**

**Finding.** `gh repo view` reports `clintcparker/site-checker` as `"isPrivate": true`.
Homebrew downloads a cask's `url` with an unauthenticated `curl`. A GitHub release asset on a
private repository returns `404` to an unauthenticated request, so
`brew install clintcparker/tap/site-checker` would fail for every user including the maintainer
on a machine without a token — and it would fail *after* the tap resolved, with a download error
rather than anything that names the cause. The public tap itself is not the problem:
`clintcparker/homebrew-tap` is public and currently holds `Formula/name-on.rb` and a README (no
`Casks/` directory yet — the release job creates it).

**Decision.** Make `clintcparker/site-checker` public before the first tag is pushed. It is a
prerequisite of Phase A, not a step inside the release pipeline.

**Rationale.** It is the only option that does not invent infrastructure, and the spec's
Assumptions already lean on `name-on`'s shape, which is public. Three secondary arguments:

- **Cost.** GitHub Actions minutes are free on public repositories. On a private one, macOS
  runners bill at a **10× multiplier** against the account's included minutes — a ~10-minute
  Tauri build × 2 architectures is ~200 charged minutes per release, and FR-021's daily
  end-to-end `brew install` check on a macOS runner would consume the entire included private
  allowance on its own. US4 is close to unaffordable while the repo is private.
- **Nothing is secret.** The application is a personal status dashboard; the repository holds no
  credentials (secrets live in GitHub's secret store, not the tree) and no third-party code under
  a licence that forbids redistribution.
- **The release notes are public anyway.** `generate_release_notes: true` publishes commit
  subjects and PR titles to a public release page. Publishing those while keeping the diff
  private is a strange halfway state.

**Consequences to accept, stated plainly.** Making the repository public is effectively
irreversible: the full history becomes visible and mirrorable, including every `specs/` document,
`CHANGELOG.md`, and all commit messages. This history is unusually verbose — it contains staff
reviews, QA transcripts, and ship records. Nothing scanned in it looks sensitive, but this is a
disclosure decision, and it is the user's to make, not the plan's.

**Alternatives considered.**

| Alternative | Rejected because |
|---|---|
| Publish releases from a separate public repository (`site-checker-releases`) | This is exactly the "new infrastructure" the roadmap says the tap avoids. It also splits the tag from the code: `verify-install-channels.yml` resolves "latest release" in one repo while `release.yml` runs in another, and `generate_release_notes` would produce notes for a repository with no commits. |
| Host the `.zip`s outside GitHub (R2 bucket, etc.) | Adds a storage account, a credential, and a lifecycle policy to a project whose entire premise is one Mac and one person. Also loses `gh release download`, which is what makes FR-011's "checksums from the artifacts actually attached" cheap. |
| Ship the cask with an authenticated URL | Casks cannot carry a per-user credential. `brew` supports `HOMEBREW_GITHUB_API_TOKEN` for API calls, not for asset downloads in a cask `url`. Even if it worked, requiring a token defeats FR-001's "single documented command". |

**Fallback if the answer is "keep it private".** Phases B, C, and F still deliver (CI, the single
version source, headless-safe builds, the documentation). Phase D can be built and merged, but
`release.yml`'s `homebrew` job must be left disabled behind an `if:` guard and FR-001, FR-011,
FR-020, FR-021, SC-001, SC-002, SC-004, and SC-006 become deferred rather than delivered. That is
a materially smaller feature and should be recorded as such rather than shipped quietly.

---

## R2. FR-006 — how the Gatekeeper problem is solved

**Resolves the spec's first `[NEEDS CLARIFICATION]`.**

**Decision.** Full Developer ID Application signing with the hardened runtime, plus notarization
and stapling. The cheap `quarantine: false` cask alternative is rejected.

**Rationale — the spec already decided this, in FR-021.** FR-021 requires the verification pass
to "confirm the operating system accepts the installed application for execution", which in
practice is `spctl -a -t exec -vv "/Applications/Site Checker.app"`. That command evaluates the
notarization ticket and the signature. An ad-hoc-signed bundle fails it with
`rejected (the code is valid but does not seem to be an app)` / `source=no usable signature`
regardless of whether Homebrew applied the quarantine attribute. So:

- `quarantine: false` satisfies SC-002 (the *user* sees no prompt) but **cannot** satisfy FR-021.
- Notarization satisfies both.

There is no arrangement in which the quarantine-bypass path passes the spec as written. The only
way to choose it is to amend FR-021 and drop the end-to-end Gatekeeper check, which is the one
check that catches the exact failure US4 exists to catch — "the maintainer sees a green release,
the user sees a broken install".

**What it costs, and what it requires.** This is the real blocker the roadmap already identified,
and it is the only line item in this feature that costs money:

1. **Apple Developer Program membership — $99/yr.** Nothing else in this plan requires payment.
2. A **Developer ID Application** certificate, exported as a `.p12`, base64-encoded into a
   repository secret.
3. An **App Store Connect API key** (Issuer ID + Key ID + `.p8`) for notarization.

**Notarization credential: API key, not app-specific password.** Tauri accepts either. Use the
API key trio (`APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_PATH`) because an
app-specific password is tied to the Apple ID's 2FA and is revoked whenever the Apple ID password
changes — a silent per-release breakage that pre-flight would catch but that would still cost a
release run. API keys are revocable independently and are not tied to the account password.

**Mechanism.** Tauri drives all three steps from the bundler when the environment is present:
`bundle.macOS.signingIdentity` (or `APPLE_SIGNING_IDENTITY`) selects the certificate and applies
the hardened runtime; when the notarization variables are also set, `tauri build` submits, waits,
and staples without a separate `notarytool`/`stapler` invocation. The workflow's job is to import
the `.p12` into a temporary keychain and export the variables — not to re-implement the flow.

**Sequencing note.** The certificate is Phase A. Enrolment is not instant (Apple's identity
verification can take days for a new individual account), so if this answer is "yes", starting
enrolment is the first action of the whole feature, in parallel with Phases B and C.

**Fallback if the answer is "not paying $99/yr".** Ship `quarantine: false` in the cask with a
`caveats` block that says why, amend FR-021 to check `codesign -v` only, and record the
Gatekeeper gap in `docs/ROADMAP.md` as deferred. The install line still works and the user still
sees no prompt; what is lost is the guarantee that the binary they got is the one that was built.
This is a defensible position for a personal tool and should be chosen deliberately, not by
default.

---

## R3. Artifact format — a `.zip` produced by `ditto`, not `.tar.gz` and not `.dmg`

**Decision.** Each architecture publishes `site-checker-<arch>-apple-darwin.zip`, created with
`ditto -c -k --sequesterRsrc --keepParent "Site Checker.app" <name>.zip`, where `<arch>` is
`aarch64` or `x86_64`.

**Rationale.**

- **`ditto`-made zip is the format Apple's own toolchain expects.** It preserves the bundle's
  extended attributes, resource forks, and — critically — the code signature. `notarytool submit`
  accepts `.zip`, `.dmg`, and `.pkg` only.
- **`.tar.gz` (what `name-on` uses) is wrong here.** `name-on` ships a single unsigned binary, so
  tar is fine. macOS `tar` handles a signed `.app`'s extended attributes by emitting AppleDouble
  `._` sidecar files, which unpack into a bundle whose signature no longer validates. This is a
  deliberate divergence from the mirrored convention, for the same underlying reason as
  cask-vs-formula: one project ships a binary, the other ships a bundle.
- **`.dmg` is rejected for the release artifact** because producing it is what hangs headless
  (R4). Homebrew casks handle `.zip` containing an `.app` natively via the `app` stanza; a DMG
  buys nothing a cask user ever sees.

**Naming.** The arch token matches the Rust target triple prefix, so the workflow derives the
asset name from the `--target` it just built and the cask interpolates it from its `arch` stanza.
No third place holds the mapping.

---

## R4. FR-016 — the headless DMG hang

**Decision.** CI and release builds pass `--bundles app`. `src-tauri/tauri.conf.json`'s
`bundle.targets` changes from `["app", "dmg"]` to `["app"]`.

**Rationale.** The roadmap's diagnosis is confirmed by the config: `targets: ["app", "dmg"]`
means every `pnpm tauri build` runs Tauri's `bundle_dmg.sh`, which calls `osascript` to lay out
the disk image's Finder window. On a machine with no interactive session that request blocks
waiting for a GUI-automation approval that can never be granted. The compile, the `.app`, and
even the `.dmg` itself all complete — only the cosmetic re-layout hangs — but the process never
exits, so the job burns its full timeout.

**Why change the config rather than only pass the flag.** Two reasons. First, defence in depth:
if a future workflow forgets the flag, the default is already safe. Second, honesty — nothing in
this project distributes a DMG any more. The cask serves a `.zip`; the DMG was only ever the
local-build artifact. Anyone who wants one can still ask for it explicitly with
`pnpm tauri build --bundles dmg` on their own Mac, and `docs/how-to/release.md` says so.

**Alternative rejected.** Setting `CI=true` (which some bundlers check) is not something Tauri's
`bundle_dmg.sh` honours for the `osascript` step; relying on it would be a guess.

---

## R5. FR-008 — one version, and no hand-maintained copy of it

**Finding.** Three files currently carry `0.1.0`: `src-tauri/tauri.conf.json`,
`src-tauri/Cargo.toml`, and `package.json`. Tauri 2.9.3's config source documents the resolution
rule directly (`tauri-utils-2.9.3/src/config.rs:3612`): *"If removed the version number from
`Cargo.toml` is used."*

**Decision.** Reduce three sources to one, then make that one machine-written:

1. **Delete `version` from `tauri.conf.json` entirely.** This removes a source of truth rather
   than synchronising it — the bundle's version now provably comes from `Cargo.toml`.
2. **Set `src-tauri/Cargo.toml`'s version to `0.0.0`** with a comment stating that the real value
   is written by `release.yml` from the tag and that hand-editing it is a build failure.
3. **Set `package.json`'s version to `0.0.0`** likewise. It does not reach the bundle, but SC-003
   counts *files containing a hand-maintained version number*, and leaving a stale `0.1.0` there
   is the kind of thing that later gets "helpfully" bumped.
4. **`release.yml` stamps `Cargo.toml`** with the tag-derived version before building.
5. **`ci.yml` asserts the committed sentinel is still `0.0.0`**, so a human bump fails a PR
   rather than silently competing with the tag.

**Rationale.** Step 5 is what turns FR-008 from a convention into a guarantee, and it is the
cheapest test in this feature — a `grep` on two files. Without it, SC-003's "the number of files
in the repository containing a hand-maintained version number is zero" is unenforced and reverts
the first time someone edits `Cargo.toml` for an unrelated reason.

**Note on `Cargo.lock`.** Stamping `Cargo.toml` makes the workspace's own package version
disagree with `Cargo.lock`, so the release build must **not** use `--locked` or `--frozen`;
cargo updates the lock entry in the runner's working copy and the change is never committed. The
plain `pnpm tauri build` path does exactly this already. `ci.yml`, which does not stamp, is free
to use `--locked` and should, so dependency drift is still caught.

**Alternative rejected.** A `build.rs` reading `GITHUB_REF_NAME` was considered — it removes the
in-place file edit, but it makes a *local* `cargo build` produce a version that depends on
whether an environment variable happens to be set, which is worse than an explicit `sed` in one
workflow step that the how-to document names.

---

## R6. Per-architecture builds, and which runner builds them

**Decision.** Keep per-architecture artifacts (as the spec assumes). Build both on
`macos-latest`, as a two-entry matrix over `--target aarch64-apple-darwin` and
`--target x86_64-apple-darwin`.

**Rationale.**

- **Per-arch over universal.** The spec permits revisiting this if the matrix proves
  disproportionately expensive; it does not. Both entries run on the same runner image, so the
  matrix costs a second job, not a second platform. Per-arch also keeps each download at ~15 MB
  rather than ~30 MB, and matches the asset convention already established in the tap.
- **One runner class, not two.** The obvious mirror of `name-on` is `macos-latest` (arm64) plus
  `macos-13` (Intel). Rejected: GitHub's Intel macOS runner images are on a retirement path, and
  a build matrix that depends on a deprecated image is a scheduled outage. Apple ships both
  architectures' SDK slices with Xcode, so `rustup target add x86_64-apple-darwin` plus
  `--target x86_64-apple-darwin` cross-compiles cleanly from the arm64 runner, including the
  native C in `aws-lc-sys`, which builds with `clang -arch x86_64` against the same SDK.
- **Signing works either way** — `codesign` signs for the target architecture regardless of host.

**Risk and its mitigation.** Cross-compiling `aws-lc-sys` is the one step here that is more
likely to surprise than the rest. It is `cmake`-driven and honours `CMAKE_OSX_ARCHITECTURES`,
which the `cc`/`cmake` crates set from the Rust target. If it does misbehave, the escape hatches
in order of preference are: (1) add `macos-13` back as the Intel matrix entry while it still
exists, (2) switch the matrix to `--target universal-apple-darwin` and publish one asset, which
collapses the cask's `arch` stanza to a single `url`/`sha256`. Both are contained changes to
`release.yml` and the cask template; neither touches application code. Phase D's first task should
be a throwaway prerelease tag that proves the Intel slice builds *before* the rest of the pipeline
is wired up.

---

## R7. FR-026 — the bundle-size mismatch

**Resolves the spec's second `[NEEDS CLARIFICATION]`.**

**Finding.** The roadmap's attribution is correct. `cargo tree -e normal` on this tree shows
`rustls v0.23.42`, `rustls-platform-verifier v0.7.0`, `aws-lc-rs v1.17.3`, and `aws-lc-sys
v0.43.0` — the last of which compiles native C. This is despite `Cargo.toml` requesting
`default-tls`: in `reqwest` 0.13, `default-tls` resolves to rustls with the `aws-lc-rs` provider,
not to `native-tls`.

The stated expectation lives at `docs/superpowers/plans/2026-07-23-site-checker.md:2554` —
*"Expected: single-digit MB. A much larger number means something pulled in an unexpected
dependency — worth investigating before shipping."* That is the only project document that
states a size; `CHANGELOG.md:327` records the actual 15 MB as history, which is a fact, not a
claim, and needs no change.

**Decision.** Amend the expectation. Replace that line with the measured size and a one-sentence
note that the cause was investigated and attributed to `aws-lc-rs`, so a future reader gets the
answer rather than the open question. Do not change the TLS backend.

**Rationale.** The roadmap's own framing settles it — *"This is a spec-expectation mismatch, not
a code defect."* The line's stated purpose was to prompt an investigation if the number came in
high. The investigation happened and produced an explanation; leaving the trigger in place after
it has fired and been answered is what makes the document wrong. 15 MB is unremarkable for a
Tauri application with a bundled TLS stack, and this feature's Out of Scope forbids changes to
what the application does — swapping the crypto provider changes which code performs every TLS
handshake the product makes, which is a behaviour change dressed as a packaging one.

**Alternative considered and rejected: switch to the `ring` provider.** Mechanically it is
`default-features = false` plus `rustls-tls-manual-roots-no-provider` and an explicit
`rustls::crypto::ring` provider install, or the equivalent feature combination — a real code
change in the HTTP client's construction, in a file whose logic is under `check.rs`'s test
suite. The saving is a few MB at best (much of the 15 MB is the WebKit-adjacent Tauri runtime and
the frontend bundle, not the crypto backend), and it would be traded for a divergence from
reqwest's default TLS path and from `rustls-platform-verifier`'s use of the macOS trust store —
the thing that makes checks behave like a browser (Constitution III). Not worth it to satisfy a
sentence.

**Where the edit goes.** `docs/` is gitignored (see R8), so this edit must be applied in the
**primary checkout**, not this worktree, or it will not survive the merge — exactly the failure
already recorded in `docs/ROADMAP.md` §2's preamble.

---

## R8. `docs/` is gitignored, so FR-024's document would never be committed

**BLOCKER for FR-024. Not raised in the spec.**

**Finding.** `.gitignore:25` is a bare `docs/`. `git check-ignore -v` confirms both
`docs/ROADMAP.md` and `docs/superpowers/plans/2026-07-23-site-checker.md` are ignored.
`README.md` and `CHANGELOG.md` are tracked. `name-on`, by contrast, tracks
`docs/how-to/release.md` — the exact path FR-024 asks for.

Writing `docs/how-to/release.md` under the current `.gitignore` produces a file that exists on the
maintainer's disk, never appears in the PR, and never reaches anyone else. SC-011 ("a maintainer
who has never released this project can complete a release using only the written procedure")
would be unsatisfiable, and this repository has already lost one documentation edit precisely
this way.

**Decision.** Change `.gitignore:25` from `docs/` to:

```gitignore
docs/*
!docs/how-to/
```

**Rationale and the subtlety that forces this exact shape.** Git cannot re-include a file whose
*parent directory* is excluded — a bare `docs/` makes `!docs/how-to/release.md` inert, because
git never descends into `docs/` to consider it. Excluding the directory's *contents* (`docs/*`)
instead of the directory leaves `docs/` itself walkable, so the negation for the subdirectory
takes effect. Everything else under `docs/` — the roadmap, the handoffs, the superpowers plan —
stays ignored exactly as today.

**Scope discipline.** The temptation is to un-ignore all of `docs/` while in here. Resist it:
that would add ~2,600 lines of planning documents and two handoff notes to this feature's diff,
none of it packaging, and it would change the status of `docs/ROADMAP.md` — a file the project
deliberately keeps local. One directory, for one document FR-024 names.

---

## R9. Pre-flight — what it can and cannot check

**Decision.** `preflight` runs on `ubuntu-latest`, derives the version, and probes for the
presence (never the validity) of every required secret, failing with one `::error::` line per
missing item that names the secret and points at `docs/how-to/release.md`. Required secrets:
`TAP_PUSH_TOKEN`, `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`,
`APPLE_API_ISSUER`, `APPLE_API_KEY_ID`, `APPLE_API_KEY`.

**Rationale.** This is `name-on`'s pattern verbatim and it satisfies FR-017, FR-018, and SC-005:
an Ubuntu job with no checkout starts in a few seconds, and every downstream job `needs:` it, so
nothing builds and nothing publishes when it fails.

**Honest limits, which the how-to must state.** Presence is not validity. Pre-flight cannot detect
an expired Developer ID certificate, a revoked API key, or a `TAP_PUSH_TOKEN` whose fine-grained
permission was scoped to the wrong repository. Those surface later — at the signing step, at
`notarytool submit`, or at the tap push. `name-on` has the same gap and documents it for the
NuGet OIDC policy; this project's version of that paragraph covers the Apple credentials. The
spec's edge case "a signing or notarization credential that expires between releases" is
therefore only *partly* met by pre-flight: expiry is caught by `verify-install-channels.yml`'s
`spctl` step and by the build failing, not by a fast pre-flight message. This is a known,
recorded limitation rather than a solved problem, and FR-017 is read as "every required
credential is *present*", which is what it says.

---

## R10. FR-027 — when is this feature done?

**Resolves the spec's third `[NEEDS CLARIFICATION]`.**

**Decision.** Done means a real `v1.0.0` tag has been pushed and
`brew install clintcparker/tap/site-checker` verifiably works on a Mac that did not build it.
Merging the automation is not the finish line.

**Rationale.** Every one of this feature's own success criteria is a statement about a published
artifact, not about a merged file. SC-001 times a real install. SC-002 asserts what a real user
does not see. SC-004 claims 100% of published versions run on both architectures — a claim with
an empty set behind it until something is published. US1's Independent Test is written as "on a
Mac that has never built this project". A release pipeline that has never run is not evidence
that a release pipeline works; it is a plausible-looking YAML file, and the entire category of
bug this feature exists to prevent (the maintainer sees green, the user sees broken) is invisible
until the first real install.

**What that implies for the ship run.** Phase G is inside the feature, not after it. Concretely:
the PR merges, `v1.0.0` is tagged on `main`, the release run goes green, the post-release
`verify-install-channels.yml` run goes green, and a clean-machine
`brew install clintcparker/tap/site-checker` is performed and recorded in the ship record
alongside the usual gate results. The `spctl -a -t exec` output is the artifact worth pasting.

**The caveat, stated so it cannot be discovered late.** This definition makes the feature's
completion depend on R1 and R2 — a public repository and an Apple Developer membership. If either
answer is no, "done" must be redefined at that moment, deliberately, and the reduced scope
recorded in `docs/ROADMAP.md`. It should not quietly degrade into "the YAML merged".

---

## R11. `ci.yml` — shape

**Decision.** One workflow, `on: push` (branches: `main`) and `on: pull_request`, two jobs.

- **`rust`** on `macos-latest`: checkout → `dtolnay/rust-toolchain@stable` with `clippy` →
  `Swatinem/rust-cache` → `cargo test --locked --manifest-path src-tauri/Cargo.toml` →
  `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`.
- **`frontend`** on `ubuntu-latest`: checkout → `pnpm/action-setup` → `actions/setup-node` with
  pnpm cache → `pnpm install --frozen-lockfile` → `pnpm test` → `pnpm build`.

**Rationale.**

- **The Rust job must run on macOS.** `src-tauri` depends on `tauri` 2.11, which links against
  macOS system frameworks; `cargo test` on Ubuntu would need the Linux WebKitGTK development
  packages and would then be testing a configuration nobody ships. The suite is pure-logic
  (`model.rs`, `check.rs`, `store.rs`) and fast, so a macOS runner is affordable — once the
  repository is public (R1).
- **The frontend job stays on Ubuntu.** `pnpm test` and `pnpm build` are platform-independent and
  Ubuntu minutes are cheaper and faster to provision. Splitting also gives FR-022's four gates two
  independently readable results.
- **No `pnpm tauri build` in CI.** FR-022 lists four gates and a full bundle is not among them;
  adding one would put the DMG hang and the signing credentials into the PR path for no gain.
  This is also why SC-009 holds for `ci.yml` trivially — nothing it runs touches the bundler.
- **`--locked` here, not in `release.yml`** — see R5.

**Version-sentinel guard.** A final step in the `rust` job asserts `src-tauri/Cargo.toml` and
`package.json` still carry `0.0.0`, enforcing R5/FR-008.

---

## R12. `verify-install-channels.yml` — shape

**Decision.** Mirror `name-on`'s file closely, with the channel set reduced to one (no NuGet) and
the end-to-end step changed from `dotnet tool install` to `brew install` + `spctl`.

Triggers: `schedule` (daily cron), `workflow_run` on the release workflow's completion, and
`workflow_dispatch` — satisfying FR-019's three cases.

Steps, all compared against the **same** resolved latest `v*` tag so a lagging channel fails by
construction:

1. **Resolve latest `v*` release** via `gh release list --json tagName`, erroring when none
   exists.
2. **Assets exist (2/2)** — `curl -sfIL` each expected `.zip` download URL. Satisfies FR-020's
   reachability half and the spec's "artifacts 404" edge case.
3. **Tap cask matches the version** — fetch
   `raw.githubusercontent.com/clintcparker/homebrew-tap/main/Casks/site-checker.rb` and
   `grep -qF "version \"$L\""`. Satisfies FR-020's lag half.
4. **End-to-end (post-release and dispatch only), on `macos-latest`**:
   `brew install clintcparker/tap/site-checker`, then
   `spctl -a -t exec -vv "/Applications/Site Checker.app"`. Satisfies FR-021.

**Why step 4 is not on the daily cron.** A macOS runner doing a real `brew install` every day is
the most expensive thing in this feature, and what it catches — a signature or notarization
problem — cannot appear between releases without something else having changed. Steps 1–3 run
daily on Ubuntu in seconds and are what catch rot (SC-006's 24-hour bound). Step 4 runs when the
thing it checks could actually have changed. `name-on` makes the same split for the same reason.

**No retry loop.** `name-on` retries the NuGet check to absorb indexing lag. GitHub release assets
and `raw.githubusercontent.com` are available essentially immediately after the release job
commits, so the retry has nothing to absorb here and would only slow a genuine failure. Dropped
deliberately, not by oversight.

---

## R13. The cask — `uninstall` vs `zap`, and FR-003/FR-004

**Decision.**

```ruby
uninstall quit: "com.clintparker.site-checker"

zap trash: [
  "~/Library/Application Support/com.clintparker.site-checker",
]
```

**Rationale.** Homebrew's semantics map exactly onto the two requirements, which is why FR-003 and
FR-004 can both be satisfied without a choice:

- `brew uninstall site-checker` removes the `.app` and runs the `uninstall` stanza. Because the
  data directory is named **only** under `zap`, the site list survives — FR-004, and Constitution
  II's "config is sacred".
- `brew uninstall --zap site-checker` additionally trashes the listed path — FR-003, SC-010.

`quit:` in the `uninstall` stanza asks a running instance to exit before the bundle is removed;
without it, uninstalling while the app is open leaves a running process backed by a deleted
bundle. The identifier is the one already in `tauri.conf.json`.

**`trash:` rather than `delete:`** — the removal is recoverable from the Trash. For the only file
the application owns, and the only irreversible act this feature introduces, that is the right
default.

**The pre-existing hand-built copy (spec edge case).** A user who previously built locally has an
`.app` wherever they put it and a `sites.json` at the same standard path. `brew install` writes to
`/Applications`, so the bundles do not collide unless the hand-built one is already there — in
which case Homebrew refuses rather than overwrites. The store is shared by design: the installed
copy picks up the existing site list, which is the desired behaviour. Worth one sentence in the
README, not a mechanism.

---

## R14. Convergent re-runs (FR-014, SC-007)

**Decision.** Inherit all three of `name-on`'s convergence properties:

1. **The tap commit is skipped when the rendered cask is unchanged** — `git status --porcelain`
   in the cloned tap, early-exit when empty. Prevents empty commits on re-run.
2. **`softprops/action-gh-release@v2` updates the existing release** for a tag rather than
   failing, re-uploading assets as needed.
3. **Nothing else writes anywhere else.** (`name-on`'s third property, `--skip-duplicate` on the
   NuGet push, has no analogue here.)

**Rationale.** Verified in `name-on`'s `release-cli.yml` at the "Commit formula to tap" step, and
documented in its `docs/how-to/release.md` "Re-running a failed release" section. Together these
give SC-007 exactly: one release per version, no duplicate channel entries.

**One asymmetry to note.** Re-running produces *fresh* `.zip`s, and a Tauri build is not
bit-reproducible (timestamps, and the signature embeds a signing time). So a re-run changes the
checksums, which changes the rendered cask, which means the tap *will* commit on re-run — the
skip only fires when the release job did not actually re-upload. This is correct behaviour (the
cask must match the assets that exist), but "unchanged content → no commit" should not be
mistaken for "re-running is a no-op". The how-to says so.

---

## R15. Secret scope (FR-015)

**Decision.** `TAP_PUSH_TOKEN` is a fine-grained personal access token with **Contents: Read and
write** on **`clintcparker/homebrew-tap` only**, set as a repository secret on
`clintcparker/site-checker`.

**Rationale.** Directly mirrors `name-on`'s documented setup, and satisfies FR-015 and the spec's
"a credential that only permits writing to the channel" edge case. The spec's assumption that
secrets do not cross repositories is correct — GitHub Actions secrets are per-repository (or
per-organization), so the same token must be configured on this repository even though it already
exists on `name-on`.

**`permissions:` blocks.** `release.yml` declares `contents: write` (it creates releases);
`ci.yml` and `verify-install-channels.yml` declare `contents: read`. Least privilege, and it makes
the intent of each workflow legible at the top of the file.
</content>
