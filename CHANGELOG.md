# Changelog

All notable changes to Site Checker are recorded here.

## CI and release actions move to the Node 24 runtime — 2026-08-11

Nothing user-visible; nothing in `src/`, `index.html`, or `src-tauri/` was touched.
Every JS action in `ci.yml` and `release.yml` was pinned to a major whose
`runs.using` is `node20`, so both workflows emitted a Node 20 deprecation
annotation on every otherwise-green run — the failure mode where nothing breaks
and nobody looks. Fourteen `uses:` lines across six distinct actions now sit on
`node24`: `checkout` v4 → v5, `setup-node` v4 → v6, `pnpm/action-setup` v4 → v6,
`upload-artifact` v4 → v6, `download-artifact` v4 → v7, and
`softprops/action-gh-release` v2 → v3. `Swatinem/rust-cache@v2` is already
`node24` and `dtolnay/rust-toolchain@stable` is a composite action; both were
left alone.

- **The rule is the lowest major that runs on `node24`, with two named
  exceptions.** `setup-node@v5` and `pnpm/action-setup@v5` are already `node24`,
  and both are pinned a major higher anyway: `setup-node@v6` narrows automatic
  caching to npm only, and all three call sites pass `cache: pnpm` explicitly, so
  the narrowing is never consulted; `action-setup@v6` only adds pnpm 11 and still
  self-updates to the `packageManager` version `package.json` pins. A first draft
  of the `ci.yml` comment block stated the rule without the exceptions, which made
  it false about two of its own pins — QA failed it, and it was rewritten before
  this shipped.
- **`download-artifact` stops at v7 deliberately.** v8 defaults
  `digest-mismatch` to `error`, turning a logged warning into a failed release
  run, and skips decompression. v7 still unzips and still defaults
  `merge-multiple: false`, so the `release` job's `artifacts/site-checker-<arch>/…`
  globs resolve unchanged. `upload-artifact` stops at v6 and `checkout` at v5 for
  the same kind of reason.
- **`.github/dependabot.yml` is new**, scoped to `github-actions` at `/` on a
  weekly schedule, so the next runtime deprecation arrives as a PR rather than as
  an annotation on a passing run. Its `ignore:` entries name the four declined
  majors *by version* (`checkout` v6/v7, `setup-node` v7, `upload-artifact` v7,
  `download-artifact` v8) rather than by `semver-major` update type: these actions
  are referenced by a bare major tag, so a major bump is the only update
  Dependabot can propose for them, and ignoring the major line would have
  silenced them permanently — reinstating the exact failure this file exists to
  prevent. Cargo and npm are deliberately excluded, and **not** because CI catches
  their drift: `cargo test --locked` and `pnpm install --frozen-lockfile` fail
  when a lockfile disagrees with its own manifest, and notice neither an upstream
  release nor a security advisory. The exclusion is a PR-volume choice for a
  single-user desktop app; turning on security-only updates for those two
  ecosystems is a separate decision, recorded rather than made here.
- **The pins imply an Actions runner floor of ≥ 2.327.1**, now noted in
  `docs/how-to/release.md` so a future self-hosted runner is not introduced
  unknowingly. The repository has no self-hosted runners today
  (`actions/runners` → `total_count: 0`) and every job runs on
  `macos-latest`/`ubuntu-latest`.

`ci.yml` was verified on a live runner: its pull-request runs conclude `success`
on both `rust` and `frontend` with an empty annotation list on each, which is the
evidence — a Node 20 deprecation annotation appears on a run that *passes*, so
the green check alone proves nothing. `release.yml` was verified by inspection
only, against `action.yml` at each target tag and each rejected tag plus the
release notes for every major crossed. Its artifact round-trip through
`upload-artifact@v6` → `download-artifact@v7` → `action-gh-release@v3` has not
been exercised end to end; the first tag push after this merges is its real test.

## Site Checker 1.0.0 — 2026-08-11

**`brew install clintcparker/tap/site-checker` works.** This supersedes the entry
below's **"Still not live"**, which was accurate when written: `v1.0.0` is
published with both `site-checker-aarch64-apple-darwin.zip` and
`site-checker-x86_64-apple-darwin.zip`, `TAP_PUSH_TOKEN` is set, and the tap
carries `Formula/site-checker.rb` at 1.0.0. `verify-install-channels` is green,
including a real `brew install` on a clean macOS runner: the bundle is
unquarantined, `codesign --verify --strict` passes, `Signature=adhoc`, the `bin`
wrapper is on `PATH`, and `CFBundleShortVersionString` reads `1.0.0`.

The thing worth recording is not that a tag was pushed. It is that **the release
pipeline had never executed once**, and four throwaway `v0.0.1` rounds — pushed,
watched, torn down — found **three real defects** that no amount of reading the
YAML had surfaced. All three are Homebrew behaviour, not Tauri or GitHub Actions:

- **Homebrew chdirs into the archive.** When a staged archive leaves exactly one
  top-level directory, `AbstractDownloadStrategy#chdir` enters it. Ours always
  does: `ditto --keepParent` puts `Site Checker.app` at the root and the zip
  strategy removes the sibling `__MACOSX` *before* the count is taken. So
  `install` ran *inside* the bundle and `libexec.install "Site Checker.app"`
  looked for a nested copy of itself. Reproduced locally once named, but only
  ever named by a real install.
- **`.brew_home` landed inside the signed bundle.** `Formula#stage` creates a
  scratch `HOME` at `buildpath/.brew_home` — and because of the defect above,
  `buildpath` *was* the bundle. The fix for the first defect swept it in, and
  verification failed with `unsealed contents present in the bundle root`. The
  install now takes `Contents` alone.
- **`on_arm`/`on_intel` are not a legal home for `url`/`sha256`.** They permit a
  fixed method list that does not include either. The unsupported form
  **installed correctly anyway** — which is precisely why it could not stay:
  something that works by accident stops working on a Homebrew upgrade, in the
  tap, for users, rather than here. Now `on_macos do … Hardware::CPU.arm? … end`,
  matching the sibling formula already in the same tap.

The second of those cost a release cycle per guess before it was named, so the
verification step now dumps the bundle root, `Contents`, the keg and full signing
detail on the failure path — the diagnostics landed between the second and third
fix, and are the reason the third was a single round.

One `brew audit` finding is **deliberately unfixed**: *"`version` is redundant
with version scanned from URL"*. Dropping the stanza breaks the URL's
`#{version}` interpolation and `verify-install-channels.yml`'s lagging-channel
grep for `version "<x>"`. It is a homebrew-core submission nit that does not apply
to a personal tap — which is why that step is non-gating rather than removed.
Filed as an issue so it is a decision on record rather than a recurring surprise.

Gates at this release: `cargo test` 62, `pnpm test` 47, `cargo clippy -- -D
warnings` clean, `pnpm build` clean, `actionlint` clean.

Release: [`v1.0.0`](https://github.com/clintcparker/site-checker/releases/tag/v1.0.0) ·
Tap: [`clintcparker/homebrew-tap` `Formula/site-checker.rb`](https://github.com/clintcparker/homebrew-tap/blob/main/Formula/site-checker.rb) ·
Spec: [`specs/20260806-190127-packaging-and-distribution/spec.md`](specs/20260806-190127-packaging-and-distribution/spec.md)

## The install channel becomes a formula, not a cask — 2026-08-11

The second of the two maintainer decisions the packaging entry left open is
answered: **no Apple Developer Program membership.** What made that interesting
is that the fallback the spec had written down for exactly this answer turned out
not to exist any more.

That fallback was `quarantine: false` in the cask. Checked against Homebrew
6.0.15: `Cask::Installer#initialize` has no `quarantine:` keyword argument, there
is no `quarantine` stanza in the cask DSL, and `no-quarantine` appears nowhere in
Homebrew's source — upstream landed *"Prepare for deprecation of
`--no-quarantine`"* and then *"Remove leftover code for `--no-quarantine`"*.
**Casks now quarantine unconditionally**, with no author-side stanza and no
user-side flag, and a quarantined bundle carrying only an ad-hoc signature opens
as "damaged". So the real choice was not "pay or apply the fallback" but "pay or
change channel".

**Site Checker now ships as a Homebrew formula.** Formulae never set
`com.apple.quarantine` — every `Quarantine.` call site in Homebrew lives under
its `cask/` directory — so the app simply opens. `brew install
clintcparker/tap/site-checker` is unchanged, because Homebrew resolves a
tap-qualified name against both `Formula/` and `Casks/`.

Three things are genuinely lost, and none is recoverable without paying:

- **Provenance.** An ad-hoc signature proves the bundle is unmodified since
  signing and nothing about who signed it. `spctl -a -t exec` cannot pass, so
  FR-021 now asserts the bundle is *unquarantined* and that `codesign --verify
  --strict` passes — a real check of a strictly weaker property.
- **`/Applications`.** A formula cannot write there; Homebrew's sandboxes deny
  every write outside the Cellar and `brew linkapps` is gone. The bundle lives in
  `libexec` — chosen over the keg root because `Cleaner` prunes at `libexec`, so
  nothing inside it is chmodded and the signature stays sealed — reached by a
  `site-checker` command on `PATH` or a one-time symlink.
- **Spotlight and Launchpad**, neither of which indexes an app through a symlink.

**`brew uninstall --zap` is gone too**, and silently: `zap` is a cask stanza, and
`--zap` on a formula exits 0 having done nothing. FR-004 is now satisfied by
construction — Homebrew never touches the site list — while FR-003 has no
mechanism at all and degrades to a documented `rm -rf`.

Nine review findings recorded during the packaging feature and never actioned
were folded in on the way through: checksum drift invisible to daily verification
(R001), the tap read through a CDN-cached URL that can fail a good release
(R002), no `workflow_run` conclusion filter so a macOS runner fired after every
*failed* release (R003), a file-scope write token reaching the jobs that run
third-party build tooling (R005), no syntax gate before publishing to a public
tap (R006), `fail_on_unmatched_files` (R007), a hardcoded repository name (R008),
no `concurrency:` group (R010), and the annotated-tag requirement nothing checked
(R011). R004 and R009 are moot under this design.

**Still not live.** No `v1.0.0` exists and `TAP_PUSH_TOKEN` is not set, so
`brew install` does not work yet. What changed is that the remaining work is one
credential and one tag, with no purchase and no enrolment wait in front of it.
*(Superseded the same day by [Site Checker 1.0.0](#site-checker-100--2026-08-11):
that credential was set and that tag was pushed. `brew install` works.)*

Spec: [`specs/20260806-190127-packaging-and-distribution/spec.md`](specs/20260806-190127-packaging-and-distribution/spec.md) ·
Decision record: [`research.md` R2](specs/20260806-190127-packaging-and-distribution/research.md) ·
Reconciliation: [`tasks.md`](specs/20260806-190127-packaging-and-distribution/tasks.md)

## Correctness and coverage from two rounds of unactioned review findings — 2026-08-11

Nothing user-visible. Three findings carried open across both concurrency-hardening
reviews, plus the coverage gaps the roadmap had been listing as
correct-but-unpinned. Rust tests 55 → 62, frontend 30 → 47.

- **`load` now holds the id-uniqueness invariant.** `AddError::DuplicateId` was
  enforced only on the append path, so a hand-edited, restored, or imported
  `sites.json` with two entries sharing an id loaded cleanly — after which
  `get`/`replace`/`update` acted on the first match while `delete` removed both.
  The first entry per id now wins, reported through the same banner channel the
  corrupt-file case uses, and the file is left alone so the dropped entry stays
  recoverable.
- **The lock-discipline guard covers what it claims.** It read a hand-maintained
  `include_str!` list, so a new module fell outside it silently, and matched one
  literal spelling, so three other ways of taking an unrecovered lock passed. It
  now walks `src/` at run time and matches a set. Widening it immediately failed
  on doc comments that spelled the needles out; those were reworded rather than
  exempted, per the rule the guard's own comment already sets.
- **`SharedStore::lock` names its deliberate exception**, so the two `commands.rs`
  comments warning against emitting under the store guard no longer read as
  contradicted by the function they call.
- **The interval ceiling is pinned across all four copies.** `86400` lived in
  `form.ts`, `index.html`, and both test fixtures, and only the first was covered
  — so a stale `max` attribute changed the product's behaviour with every test
  still green. A new guard globs `index.html` and every `*.test.ts` and fails on
  disagreement, covering files that do not exist yet.
- Plus the paths roadmap §3 listed: `check.rs`'s redirect limit, transport-level
  GET-retry failure, and 405-to-both-methods; `store.rs`'s `update` no-op;
  `render.ts`'s clear-to-empty and front-insertion; `main.ts`'s `removeSite` and
  startup banner; `form.ts`'s `enterEditMode` and cancel.

## Repository made public — 2026-08-06

The repository is now public, which retires the first of the two maintainer
decisions the entry below leaves open. **This supersedes that entry's "this
repository is still private".** The second decision — a $99/yr Apple Developer
Program membership — is still open, and `brew install` still does not work: no
`v1.0.0` tag exists and none of the seven release credentials is set, so
pre-flight fails before anything builds. The README says so where a visitor
will see it.

Going public was preceded by an audit of all 91 commits and every tracked file.
No secrets were found: every CI credential is a `secrets.*` reference, and no
`sites.json`, `.env`, or key material has ever been committed. What the audit
did find was fixed:

- **`LICENSE` (0BSD)** at the root — permissive to the point of not requiring
  attribution. It does not cover `.specify/extensions/`, which vendors six
  third-party MIT extensions; each keeps its own license and copyright.
- **96 files of spec-kit process exhaust untracked**, plus `.specify/feature.json`,
  which held an absolute local worktree path. Releases, `qa/responses/`,
  screenshots, and `.spec-context.json` are machine output that means nothing off
  the machine that produced it. The spec documents stay — every `specs/` link in
  this changelog was enumerated first, and none pointed at a dropped file.
- **21 absolute paths scrubbed** from the kept spec documents.
- **README corrected** — a dead link to a design document that existed neither on
  disk nor in git, and `brew install` presented as working when it 404s.
- **21 merged branches pruned.** `origin` now carries only `main`.

History was deliberately not rewritten. Old commits keep the screenshots and the
scrubbed paths, which reveal only a username already public via the commit email.
A rewrite would break every merged pull request link and the commit SHAs these
specs cite.

## Packaging & distribution — 2026-08-06

Site Checker gets an install path. `brew install clintcparker/tap/site-checker`
replaces "clone the repo, install two toolchains, and run `pnpm tauri build`",
and publishing a version becomes pushing one annotated `v<MAJOR>.<MINOR>.<PATCH>`
tag. **Nothing the application does changes** — `src/` and `src-tauri/src/` are
untouched, and the test counts are unmoved at 55 Rust and 30 frontend.

**Not yet live, and deliberately so.** *(Every claim in this paragraph has since
been overtaken: the repository is public — see [Repository made
public](#repository-made-public--2026-08-06) — the cask became a formula, and
`v1.0.0` shipped. See [Site Checker 1.0.0](#site-checker-100--2026-08-11). It is
left as written because it was true on 2026-08-06.)* What lands here is the
machinery: the workflows, the cask template, the single version source, and the
how-to. No `v1.0.0` exists, this repository is still private, and none of the
seven credentials is set, so `brew install clintcparker/tap/site-checker` does not
work today — an unauthenticated fetch of a private release asset returns 404.
Two decisions that are the maintainer's alone gate the rest: making the
repository public (which exposes its full history), and a $99/yr Apple Developer
Program membership for signing and notarization. Until both are answered,
FR-001, FR-006, FR-015, FR-027 and the runtime half of FR-005 stay deferred, and
`docs/ROADMAP.md` §2 carries the reduced scope. Read this entry as "the pipeline
is built and statically proven", not as "packaging shipped".
*(`docs/ROADMAP.md` is gitignored and so invisible to any reader but the
maintainer — the reason this entry cites it is itself the problem tracked in
[#21](https://github.com/clintcparker/site-checker/issues/21). What §2 recorded
is now public in
[#20](https://github.com/clintcparker/site-checker/issues/20).)*

Spec: [`specs/20260806-190127-packaging-and-distribution/spec.md`](specs/20260806-190127-packaging-and-distribution/spec.md) ·
Plan: [`specs/20260806-190127-packaging-and-distribution/plan.md`](specs/20260806-190127-packaging-and-distribution/plan.md) ·
Tasks: [`specs/20260806-190127-packaging-and-distribution/tasks.md`](specs/20260806-190127-packaging-and-distribution/tasks.md) ·
Research: [`research.md`](specs/20260806-190127-packaging-and-distribution/research.md)

### Added

- **A Homebrew cask** — `install/homebrew/site-checker.rb`, the canonical
  template, rendered by automation into `clintcparker/homebrew-tap` as
  `Casks/site-checker.rb` and never written there by hand. A cask rather than a
  formula because Site Checker ships an application bundle; the advertised
  install line is unchanged either way, because Homebrew resolves a
  tap-qualified name against both directories.
- **The first mechanism in this project's history that can delete your site
  list** — and it is opt-in. `brew uninstall` keeps `sites.json`;
  `brew uninstall --zap` moves it to the Trash. The data directory appears in
  exactly one stanza, and it is `zap`. That split, not discipline, is what makes
  "config is sacred" hold across an uninstall path.
- **`.github/workflows/release.yml`** — `preflight` → `test` → a two-architecture
  `build` matrix → `release` → `homebrew`. Every job reaches `preflight` through
  its `needs:` chain, so a malformed tag or a missing credential fails in seconds
  having built nothing, published no release object, and left the tap untouched.
  The tap is rendered from checksums of the assets *actually attached to the
  release*, never from predicted names.
- **`.github/workflows/ci.yml`** — the four gates the constitution names by hand
  (`cargo test`, `cargo clippy -- -D warnings`, `pnpm test`, `pnpm build`) now run
  on every push to `main` and every pull request. This repository had no CI at
  all; three ship runs stayed green because the runs were disciplined about it,
  not because anything enforced it.
- **`.github/workflows/verify-install-channels.yml`** — daily, after every
  release, and on demand: the assets resolve, the tap advertises the same version,
  and (off the cron) a real `brew install` on a clean runner produces an app
  Gatekeeper accepts. Catches the one failure a green release cannot: the
  maintainer sees success, the user sees a broken install.
- **[`docs/how-to/release.md`](docs/how-to/release.md)** — the procedure, the
  one-time setup, and what to do when a release fails partway through. Committing
  it required un-ignoring `docs/how-to/` specifically, as `docs/*` plus
  `!docs/how-to/`; git will not re-include a file whose parent directory is
  excluded, so the exact shape is load-bearing.

### Changed

- **Three version numbers became one, and it is machine-written.**
  `tauri.conf.json`'s `version` key is deleted outright rather than synchronised,
  so Tauri falls back to `Cargo.toml` — verified by building a stamped `9.9.9` and
  reading it back out of `Info.plist`, not by trusting the documentation.
  `Cargo.toml` and `package.json` now hold an inert `0.0.0` that `release.yml`
  stamps from the tag, and a CI step fails any pull request that edits either
  sentinel. That guard is what turns the single-source rule from a convention
  into a guarantee.
- **`bundle.targets` is `["app"]`.** Nothing distributes a DMG any more, and
  building one calls `osascript` against the Finder, which blocks forever where
  there is no interactive session. Reducing the default means a build that forgets
  `--bundles app` still cannot reach it. `pnpm tauri build --bundles dmg` still
  works locally.
- **README leads with the install line** rather than the build instructions.

## Concurrency & robustness hardening — verification round 2 — 2026-08-06

**No code change.** Nothing under `src-tauri/src/` or `src/` differs from what
shipped in the entry below; `git diff 8feb989..HEAD` over both trees is empty.
What this entry records is evidence, and one thing that evidence closed.

Spec: [`specs/20260806-120353-concurrency-hardening/spec.md`](specs/20260806-120353-concurrency-hardening/spec.md) ·
Review: [`reviews/review-20260806-175302.md`](specs/20260806-120353-concurrency-hardening/reviews/review-20260806-175302.md) ·
QA: [`qa/qa-20260806-181800.md`](specs/20260806-120353-concurrency-hardening/qa/qa-20260806-181800.md)

### Verified

- **The shell layer was finally exercised against a running window.** `commands.rs`
  has no automated coverage by design (Constitution IV, research R7), and the manual
  click-through standing in for it had only ever been half-run — a *refused* add, and
  nothing else. This round drove the real window through the macOS accessibility API
  with synthetic mouse clicks and performed a successful add, an edit, and a delete,
  hashing and parsing `sites.json` after each. The edit kept its list index; the
  delete did not come back after a relaunch. That closes the verification gap review
  finding R002 was tracking.
- **FR-011 and the warning channel are now proven at runtime.** With the app running,
  the store directory was made unwritable and a valid site added: the banner rendered
  *"Could not write sites.json: Permission denied (os error 13)"*, the row appeared
  anyway, its check ran, and `sites.json` on disk stayed byte-identical. Because
  `lock.rs:103` is the crate's only `store-warning` emit and FR-004's warning goes
  through the same private `warn()` helper, this is also the first runtime proof of
  the mechanism FR-004 reuses.
- **Both verdicts were re-derived, not inherited.** All three merge gates were re-run
  from a clean checkout — 55/0/0 Rust, 30/0 frontend unmodified, clippy clean at
  `--all-targets -- -D warnings`. The lock inventory (nine sites, all recovering),
  FR-015/FR-016 by file list, and FR-018 by removal audit were each re-checked from
  source.

### Known gaps

- The poison → `warn()` trigger still has no runtime assertion (QA TC-005, PARTIAL).
  No sequence of clicks can poison a mutex; closing it needs the `tauri` `test`
  feature and a mock-app harness, declined at research time.
- Review findings R001 (the source-text lock guard under-covers), R003, and R004
  (`load` accepts duplicate ids) remain open and are still not in `docs/ROADMAP.md`.
  That miss is itself tracked as R005.
  *(All three were fixed on 2026-08-11 — see "Correctness and coverage from two
  rounds of unactioned review findings" above. R005, the structural miss, is now
  [#21](https://github.com/clintcparker/site-checker/issues/21), and open work
  lives in the issue tracker rather than the gitignored roadmap.)*

## Concurrency & robustness hardening — 2026-08-06

The three actionable items from roadmap section 1, all in the Rust core. The
headline is that a panic inside the app no longer bricks it: today one fault
while the site list is locked poisons that lock and cascades a panic into every
later command, so the window keeps ticking status updates while refusing to add,
edit, delete, or even list anything until relaunch. As with the feature before
it, no frontend file is touched, no dependency is added, and nothing about the
on-disk shape or the IPC contract changes.

Spec: [`specs/20260806-120353-concurrency-hardening/spec.md`](specs/20260806-120353-concurrency-hardening/spec.md) ·
Plan: [`specs/20260806-120353-concurrency-hardening/plan.md`](specs/20260806-120353-concurrency-hardening/plan.md) ·
Tasks: [`specs/20260806-120353-concurrency-hardening/tasks.md`](specs/20260806-120353-concurrency-hardening/tasks.md) ·
Research: [`research.md`](specs/20260806-120353-concurrency-hardening/research.md) ·
Source: section 1 of the roadmap. None of the three was reachable from the
shipped window — the point is that the core stops relying on the window to avoid
states it permits.

### Fixed

- **One internal panic no longer disables the whole app.** Ten
  `Mutex::lock().unwrap()` call sites meant a panic inside any critical section
  poisoned that lock, and every later command panicked on contact with it. The
  fix is not ten call-site edits: `SharedStore` now wraps `Arc<Mutex<Store>>`
  with no accessor for the raw mutex, so a store lock *cannot* be taken
  un-recovered. Recovery is `PoisonError::into_inner()` — which preserves
  whatever the interrupted operation left behind rather than resetting the list
  — plus `Mutex::clear_poison()`, which makes the poison one-shot so the user
  gets one banner per fault instead of one per subsequent action. The two
  task-registry locks and the startup-warning lock call the same `lock::recover`
  helper directly and discard its flag: recovering them is required, warning
  about them is not, because which checks are running is ephemeral by design and
  rebuilt every launch.
- **A refused add no longer leaves a ghost row.** `Store::add` gained a
  duplicate-id refusal in the previous feature, but `add_site` funnelled every
  `Store::add` error into `warn_on_write_failure` and returned `Ok` regardless.
  A refusal therefore surfaced as "could not be saved" — a message that promises
  the change is still there, just un-persisted — while the window added the row
  and a timer started checking a site that was in no list, until it vanished at
  the next launch. A two-variant `AddError` lets the shell tell the two apart:
  a refusal returns `Err` with no row, no timer, and no banner, while a genuine
  write failure keeps today's behaviour exactly.
- **Two overlapping edits to one site can no longer discard each other.**
  `update_site` took the store lock twice — once to read `method_override`, once
  to write — so a second edit that began before the first finished decided from
  the same stale snapshot and silently overwrote it. The concrete loss was the
  app's memory of which request method a site needs, learned at the cost of an
  extra failed request against the user's site. The read-decide-write now lives
  inside `Store::replace`, where a single `&mut self` borrow makes the
  interleaving impossible by construction rather than by call-site discipline.

### Technical Notes

- One module added, `src-tauri/src/lock.rs`, holding `recover` as a Tauri-free
  generic function so poison recovery is unit-testable without a `State`.
- The lock-site count goes 10 → 9: collapsing `update_site`'s two acquisitions
  into one is what removes the tenth.
- No dependency added, runtime or dev. Poison recovery is `std::sync` only;
  `tauri`'s `test` feature was considered and declined.
- `AddError`, `Replaced`, and `SharedStore` are internal Rust types that never
  cross the IPC boundary — the commands map them back to the existing
  `Result<Site, String>` and `Result<(), String>` shapes, so the frontend
  contract is byte-identical and `src/` is untouched.
- Only two user-visible changes are permitted by the spec and only two were
  made: the poison-recovery warning, which reuses the existing banner rather
  than a new mechanism, and the reworded refusal message.
- Gate at merge: 55 Rust tests passing (up from 42), 30 frontend tests
  unchanged, `cargo clippy -- -D warnings` clean.

## Durability & data integrity — 2026-08-06

The three items from roadmap section 1, all in the Rust core. The headline is
that saving `sites.json` is now all-or-nothing: a crash mid-save can no longer
truncate the file and cost the user their list. No frontend file is touched, no
dependency is added, and nothing about the on-disk shape or the IPC contract
changes.

Spec: [`specs/003-durability/spec.md`](specs/003-durability/spec.md) ·
Plan: [`specs/003-durability/plan.md`](specs/003-durability/plan.md) ·
Tasks: [`specs/003-durability/tasks.md`](specs/003-durability/tasks.md) ·
Source: section 1 of the roadmap. Unlike the two features before it, this one
ran the full specify → plan → tasks cycle, so the mechanism decisions are
recorded in [`research.md`](specs/003-durability/research.md) rather than left
to implementation.

### Fixed

- **A crash mid-save can no longer truncate `sites.json`.** `Store::save` called
  `std::fs::write` directly, which empties the file and then refills it — a
  window in which a panic, a kill, or a dev-server restart left a truncated
  file. The next launch parsed that as corrupt, showed the banner, and started
  empty: graceful, but the last edit was gone. The write is now staged to a
  sibling `sites.json.tmp`, flushed with `sync_all`, and published with
  `std::fs::rename`, which is atomic within a filesystem — a reader sees either
  the complete old file or the complete new one, never a mixture. Because `add`,
  `update`, and `delete` all funnel through the one private `save`, this covers
  every mutation and no caller moved (US1).
  - The staging name is *fixed* rather than randomized, so repeated interrupted
    saves reuse the one artifact instead of leaving an orphan per crash, and the
    next successful save reclaims it. `load` opens only the path it was handed,
    so an orphan is never mistaken for the site list.
  - The honest limit, recorded in the code: this defends against the *process*
    dying, because the kernel completes the rename whether or not we survive it.
    It is not a power-loss guarantee — macOS `fsync` does not force the drive's
    own write cache the way `F_FULLFSYNC` does, and the parent directory is
    deliberately not synced.
- **`Store::add` refuses an id it already holds.** It pushed unconditionally, so
  two sites under one id would have made `get`/`update` hit the first while
  `delete` removed both. The check runs *before* the push and before the save, so
  a refusal leaves the in-memory list and the file agreeing. Unreachable from the
  shipped app — `add_site` mints a fresh v4 UUID per site — and added so the
  invariant lives at the layer that owns it rather than being a property of one
  caller's id generator (US3).

### Changed

- **A typed-in scheme is now stored lowercase.** `HTTPS://example.com` persisted
  verbatim, because `normalize_url` returns the user's own text rather than
  `url::Url`'s serialization — deliberately, since that is what keeps
  `example.com` yielding `https://example.com` and not `https://example.com/`.
  `has_leading_scheme` became `leading_scheme_end`, returning the scheme's byte
  index instead of a bool, so exactly the scheme slice is lowercased and the rest
  of the input is passed through untouched: hosts, paths, and query values keep
  their case, and a `HTTP://` inside a query string is not a leading scheme and is
  left alone (US2).
  - There is no migration. `load` does not call `normalize_url`, so a site already
    stored as `HTTPS://…` keeps that value until the user next edits it. On that
    edit it counts as a URL change under the existing rule, so the row drops to
    Pending and `method_override` is cleared — one extra request to re-learn HEAD
    support, once, for that site.
- **A symlink at the `sites.json` path is now replaced rather than followed.** A
  plain write followed the link and wrote through to its target; an atomic replace
  cannot. Nothing is destroyed — the old target keeps every byte it held — but the
  indirection is gone. Inherent to the fix, and the app never creates such a link.

### Added

- Seven `store.rs` tests covering the write path end to end: an interrupted save
  leaving the previous list loadable, the staged copy holding the new contents
  beside the live file, a successful save leaving nothing behind, repeated staging
  never accumulating more than one artifact, a failed staging preserving the
  previous file, a failed publication leaving exactly one orphan, and a stale
  artifact from a crashed run being inert to both `load` and the next save. The
  staging step is split out from `save` specifically so a test can stop a save at
  the instant the guarantee is about, rather than racing a killed subprocess.
- Two `store.rs` tests for the duplicate-id refusal, both asserting on a *reload*
  rather than the in-memory list, which is what proves the refusal preceded any
  write.
- Four `model.rs` tests for the scheme table, including the guard that stops the
  fix from being a lazy whole-string lowercase.
- Rust tests 29 → 42. Frontend stays at 30 — this feature touches no frontend file.

## Robustness — 2026-08-05

Five small correctness wins from the v1 review's Minor findings. One was a real
(if rare) bug; the rest close windows that were reachable but harmless, or
latent. No Rust source is touched, no dependency is added, and nothing about
`sites.json` or the IPC contract changes.

Tasks: [`specs/002-robustness/tasks.md`](specs/002-robustness/tasks.md) ·
Source: section 1 of the roadmap. There is no `spec.md` or `plan.md` — the
roadmap section named the file, the function, and the symptom for each item, so
it served as the spec directly.

### Fixed

- **A status event arriving during startup is no longer dropped.** `src/main.ts`
  registered `onSiteStatus` / `onStoreWarning` only after `await mountAutostart()`
  and `await getWarning()`. Tauri events have no replay, so anything emitted in
  that window was gone. Both registrations now run before every other `await` in
  `main()`. This is the one item the v1 review called a real bug (US1).
- **A row returns to Pending the moment its URL changes.** `upsertSite` updated
  `sites` but never `statuses`, so editing a good URL to a bad one kept showing
  a green dot until the next check landed — the UI claiming a confirmation it
  did not have. The stale status is now dropped on a URL change, and only on a
  URL change: label-only and interval-only edits keep the dot, because that
  result is still about that URL (US2).
- **Double-click can no longer save or delete twice.** The submit handler and
  the per-row Delete in `src/form.ts` had no in-flight guard. Both now disable
  their button around the awaited call and return early if it is already
  disabled. A failed save re-enables in a `finally`, so a rejection stays
  retryable (US3).

### Changed

- **The interval field is bounded at both ends.** `index.html`'s `#site-interval`
  gains `max="86400"` and `src/form.ts`'s clamp gains a matching ceiling, so a
  pasted 21-digit number is clamped at the source instead of failing `u64`
  deserialization at the IPC boundary. The floor behaviour is unchanged. 86400
  is a product guardrail chosen here, not a protocol limit — the backend still
  enforces only `MIN_INTERVAL_SECS` (US4).
- **A missing `#autostart` element degrades to a banner instead of a dead page.**
  `mountAutostart`'s `querySelector(...)!` meant that if the element ever went
  missing, the `catch` block's own `checkbox.disabled = true` threw a second
  time and aborted the rest of `main()`. It now early-returns with a banner,
  leaving `catch` operating on a checkbox known to exist. Latent only — the
  element is static in `index.html` (US5).

### Added

- `src/form.test.ts` — a DOM unit test over `form.ts`, following
  `render.test.ts`'s local-fixture style with `./api` stubbed by deferred
  promises so the in-flight window is inspectable. Covers the submit and delete
  guards, the retryable failure path, Add-vs-Edit dispatch, and the clamp table
  (floor, in-range, ceiling, empty, non-numeric).
- `src/main.test.ts` — coverage for the three `main.ts` stories, which had none.
  Importing `main.ts` *is* running startup (it calls `main()` at module load),
  so the test stubs `./api` and mounts a fixture DOM first. US1 is pinned by
  mock `invocationCallOrder` — that both listeners register before any startup
  IPC call, which is the ordering property, not merely that they register. US2
  is pinned in all four directions: URL change drops the status; label-only,
  interval-only, and first-add do not. US5 is pinned by omitting `#autostart`
  from the fixture, so every other assertion in the file doubles as proof that
  a missing control no longer aborts `main()`.
- Frontend tests: 12 → 30.

### Technical Notes

- The guards check `disabled` explicitly rather than relying on the browser
  refusing to click a disabled button: this handler also runs for a programmatic
  submit, which never consults `disabled`.
- The delete guard deliberately re-enables only on the failure path. On success
  the row — and its button — is removed by `onDeleted`.
- `upsertSite` compares two backend-normalized URLs. `addSite`/`updateSite`
  resolve to the saved `Site` after `normalize_url`, so a cosmetic difference
  the backend already collapsed cannot false-positive into a spurious reset.
- 86400 now lives in three places that must stay in sync: `form.ts`'s constant,
  `index.html`'s `max`, and the ceiling case in `form.test.ts`.
- Every assertion added here was confirmed to fail against the pre-fix code
  before being kept — the five guard tests, the clamp ceiling, the URL-change
  reset, the missing-`#autostart` banner, and the listener ordering.
- Quality gates: `cargo test` (29 passing), `pnpm test` (30 passing),
  `cargo clippy -- -D warnings` (clean), `pnpm build` (clean).

## Scaffold Cleanup — 2026-08-05

Removes the residue `create-tauri-app` left behind and sharpens two imprecise
strings. No new code, no new dependencies, and no behavior change beyond the
wording of one warning banner.

Spec: [`specs/001-scaffold-cleanup/spec.md`](specs/001-scaffold-cleanup/spec.md) ·
Plan: [`specs/001-scaffold-cleanup/plan.md`](specs/001-scaffold-cleanup/plan.md) ·
Tasks: [`specs/001-scaffold-cleanup/tasks.md`](specs/001-scaffold-cleanup/tasks.md)

### Removed

- The unregistered `opener` plugin, in all three places it was declared:
  `"opener:default"` from `src-tauri/capabilities/default.json`,
  `@tauri-apps/plugin-opener` from `package.json`, and `tauri-plugin-opener`
  from `src-tauri/Cargo.toml`. `src-tauri/src/lib.rs` only ever initialized
  `tauri_plugin_autostart`, so the plugin was granted permission surface and
  compiled into the shipped binary without being used. Both lockfiles were
  regenerated (US1).
- The three orphaned scaffold SVGs — `src/assets/tauri.svg`,
  `src/assets/typescript.svg`, and `src/assets/vite.svg`. No source file or
  `index.html` referenced them, and Vite already excluded them from the bundle,
  so `dist/` is byte-for-byte unaffected. `src/assets/` is now empty and gone (US2).

### Changed

- The package and crate now identify themselves as Site Checker instead of the
  scaffold: `package.json` `name` is `site-checker`; `src-tauri/Cargo.toml` carries
  `name = "site-checker"`, a real one-line `description`, and
  `authors = ["Clint Parker <me@clintparker.com>"]` in place of `authors = ["you"]` (US3).
- `[lib] name` renamed `tauri_app_lib` → `site_checker_lib`, with the matching
  `src-tauri/src/main.rs` call site updated in the same change — the one
  build-breaking ripple in this feature (US3).
- The corrupt-file warning in `src-tauri/src/store.rs::load` now names its actual
  cause (the file is not valid JSON) instead of reading like the neighbouring
  I/O-error message. It still reassures the user the file was left on disk.
  This is the only user-visible change in the feature (US4).
- The `has_leading_scheme` doc comment in `src-tauri/src/model.rs` now states the
  character-class rule its body applies — the text before `://` must be entirely
  ASCII alphanumeric or one of `+`, `-`, `.` (US4).

### Technical Notes

- The bundle is unchanged where it counts: `src-tauri/tauri.conf.json` pins
  `productName: "Site Checker"` and `identifier: com.clintparker.site-checker`
  and never referenced the crate name, so `pnpm tauri build --bundles app` still
  emits `Site Checker.app` at the same 15 MB.
- No persisted or IPC field was renamed. `sites.json` keeps its shape, its path,
  and its load semantics — only the warning text changed.
- Sequencing was constrained by one cross-story conflict: US1 and US3 both edit
  `package.json` and `src-tauri/Cargo.toml`, so they were serialized. US2 and US4
  touch disjoint files.
- Quality gates after every story, not just at the end: `cargo test` (29 passing),
  `pnpm test` (12 passing), and `cargo clippy -- -D warnings` (clean).
