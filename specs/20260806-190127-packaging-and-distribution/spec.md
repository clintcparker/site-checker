# Feature Specification: Packaging & Distribution

**Feature Branch**: `20260806-190127-packaging-and-distribution`

**Created**: 2026-08-06

**Status**: Draft

**Input**: User description: "tackle section 2 of docs/ROADMAP.md"

**Source**: `docs/ROADMAP.md` §2 — "Build, packaging & distribution". This spec covers
every item in that section: the missing install path (the Homebrew item and all seven of
its sub-bullets), the bundle-size mismatch, the headless DMG hang, and the absence of CI.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Install Site Checker in one line (Priority: P1)

Someone who wants Site Checker on their Mac runs a single documented command and ends up
with a working app in `/Applications`. They never clone the repository, never install a
Rust or Node toolchain, and never have to right-click-open or otherwise talk their way
past a macOS security warning. When they later change their mind, one documented uninstall
command removes the app, and an opt-in variant of it also removes the site list the app
stored.

**Why this priority**: This is the entire point of the section. Today the only way to get
the app is to clone the repo and build it, which means Site Checker has no users other
than the person who wrote it. Every other story here exists to make this one repeatable.

**Independent Test**: On a Mac that has never built this project, run the documented
install command, launch the installed app, add a site, and confirm it checks. Then run the
documented uninstall-with-data command and confirm both the app and its stored list are
gone. No other story needs to exist for this to deliver value.

**Acceptance Scenarios**:

1. **Given** a Mac with Homebrew and no Site Checker, **When** the user runs
   `brew install clintcparker/tap/site-checker`, **Then** Site Checker is installed as an
   ordinary macOS application and appears where installed applications normally appear.
2. **Given** the freshly installed app, **When** the user opens it for the first time,
   **Then** it launches and is usable without the operating system reporting it as damaged,
   unverified, or from an unidentified developer, and without the user changing any
   security setting.
3. **Given** an installed copy that has been used (so a saved site list exists), **When**
   the user runs the documented uninstall command with its data-removal option, **Then**
   the application and the saved site list are both gone.
4. **Given** an installed copy that has been used, **When** the user runs the plain
   uninstall command, **Then** the application is gone and the saved site list is left
   intact, so re-installing restores the user's sites.
5. **Given** an Apple Silicon Mac and an Intel Mac, **When** each runs the same documented
   install command, **Then** each receives a copy that runs natively on that machine.

---

### User Story 2 - Publish a version by pushing one tag (Priority: P1)

The maintainer decides the current state of the default branch is a release. They push one
annotated version tag and do nothing else. Automation takes it from there: the version
recorded inside the app matches the tag, both architectures are built, a public release is
published with the built artifacts and generated notes attached, and the public install
channel is updated to serve exactly that version. There is no version number to edit in any
file before, during, or after.

**Why this priority**: Equal to US1 because an install path nobody can refresh is worse
than none — it strands users on the first version ever published. Making the tag the single
source of truth is also what prevents the version inside the app from drifting away from
the version users are told they installed.

**Independent Test**: Push a version tag at a known commit, wait for automation, then
confirm: a public release exists for that tag with an artifact per architecture, the
install channel advertises that exact version, and the version reported inside the app
matches the tag. Requires no manual editing step to be tested.

**Acceptance Scenarios**:

1. **Given** a commit on the default branch, **When** the maintainer pushes an annotated
   tag of the form `v<MAJOR>.<MINOR>.<PATCH>`, **Then** a public release for that version is
   published with one installable artifact per supported architecture and human-readable
   notes, with no further human action.
2. **Given** that release, **When** it finishes publishing, **Then** the public install
   channel is updated to point at those exact artifacts, verified against the artifacts
   actually attached to the release rather than against locally predicted names.
3. **Given** an installed copy of that release, **When** the user inspects the version the
   application reports, **Then** it matches the tag that produced it.
4. **Given** a tag that does not match the required version form, **When** it is pushed,
   **Then** no release is produced and nothing is published.
5. **Given** a release that failed partway through, **When** the maintainer re-runs it for
   the same version, **Then** the result converges rather than duplicating: the existing
   release is updated in place, and the install channel is left untouched when the content
   it would be given is unchanged.

---

### User Story 3 - Be told what one-time setup is missing, immediately (Priority: P2)

The first time a release is attempted — and any time a credential expires or is rotated
away — the run stops within seconds and names the specific missing piece, instead of
building for several minutes and then failing at the publish step or, worse, publishing
something broken.

**Why this priority**: One notch below the first two because it does not add capability,
it protects the maintainer's time and prevents half-published releases. It matters most
exactly once per credential, but that once is the moment when everything is unfamiliar.

**Independent Test**: Temporarily remove one required credential, push a version tag to a
scratch tag name, and confirm the run fails within seconds naming that credential, having
built nothing.

**Acceptance Scenarios**:

1. **Given** required release credentials are not configured, **When** a release is
   triggered, **Then** it fails in seconds with a message naming which piece is missing,
   and no build, release, or channel update occurs.
2. **Given** all credentials are configured, **When** a release is triggered, **Then** the
   pre-flight check passes and the run proceeds.
3. **Given** a documented one-time setup checklist, **When** a new maintainer follows it
   end to end, **Then** the pre-flight check passes on the first attempt.

---

### User Story 4 - Catch a stale or broken install channel automatically (Priority: P2)

Without anyone looking, the project notices when what it advertises and what it actually
serves have diverged — a release whose artifacts 404, an install channel still pointing at
the previous version, or an installed copy that the operating system refuses to run.

**Why this priority**: The failure this catches is silent and user-facing: the maintainer
sees a green release, the user sees a broken install. It is deliberately below the release
pipeline itself, because it verifies that pipeline rather than replacing it.

**Independent Test**: Run the verification on demand against the current published version
and confirm it passes; point it at a deliberately stale channel value and confirm it fails.

**Acceptance Scenarios**:

1. **Given** a published release, **When** verification runs, **Then** every artifact the
   install channel references is confirmed downloadable and the channel is confirmed to
   advertise the latest released version.
2. **Given** an install channel left on an older version, **When** verification runs,
   **Then** it fails and identifies the lagging channel.
3. **Given** a published release, **When** verification runs its end-to-end step, **Then**
   it installs the app the same way a user would on a clean machine and confirms the
   operating system accepts the installed application for execution.
4. **Given** no release has happened recently, **When** the scheduled verification runs,
   **Then** it still checks the currently published version, so a channel that rots between
   releases is caught within a day.

---

### User Story 5 - Every push is checked automatically (Priority: P3)

Changes pushed to the repository, and every pull request, are checked by automation:
the Rust test suite, the Rust lint gate with warnings treated as errors, the frontend test
suite, and the frontend build. Nobody has to remember to run them, and a red result is
visible before a merge rather than after.

**Why this priority**: Below the release path because the discipline it replaces is
currently working — three ship runs verified all four gates by hand and recorded the
results. It is a durability improvement, not a missing capability, and the release pipeline
in US2 assumes such a suite already exists.

**Independent Test**: Open a pull request containing a deliberately failing test and
confirm the checks report failure without any local action; remove the failure and confirm
they report success.

**Acceptance Scenarios**:

1. **Given** a pull request, **When** it is opened or updated, **Then** the Rust tests,
   the Rust lint gate with warnings as errors, the frontend tests, and the frontend build
   all run automatically and their pass/fail result is attached to the pull request.
2. **Given** a change that breaks any one of those four gates, **When** the checks run,
   **Then** the result is a visible failure naming the failing gate.
3. **Given** the checks run in an environment with no interactive desktop session,
   **When** they run, **Then** they complete without hanging.

---

### User Story 6 - The release procedure is written down (Priority: P3)

A maintainer — including the original one, a year later — can find one document that says
how to release, what one-time setup exists, and what to do when a release fails partway
through.

**Why this priority**: Lowest because it documents behaviour the other stories create.
It is still in scope: the per-feature ship records this repository already keeps say what
happened on a given run, not how to perform a release.

**Independent Test**: Hand the document to someone who has never released this project and
have them perform a release using only it.

**Acceptance Scenarios**:

1. **Given** the documentation, **When** a maintainer reads it, **Then** it states the
   release procedure, the one-time setup checklist, and the recovery procedure for a
   failed or partially completed release.
2. **Given** a failed release, **When** the maintainer follows the recovery section,
   **Then** the same version can be re-released without manual cleanup of the previously
   published state.

---

### Edge Cases

- **Re-running a release for a version that already published.** Must converge, not
  duplicate: the existing release is updated in place and the install channel is left
  untouched when the content it would receive is byte-identical.
- **A release artifact is missing when the install channel is updated.** The channel must
  be built from the artifacts actually attached to the published release, so a missing or
  renamed artifact fails the run instead of publishing a channel entry that 404s.
- **Placeholder drift between the channel template kept in this repository and the
  automation that renders it.** Both directions must fail the release: a placeholder the
  automation expects but the template no longer contains, and a placeholder that survives
  unrendered into the published output.
- **Building where no interactive desktop session exists.** The disk-image packaging step
  drives the Finder through GUI automation and blocks waiting for permission that a
  headless machine can never grant, even though the compiled application itself completes.
  Release and CI builds must not hang on this.
- **A version recorded in a build-configuration file competing with the tag.** The
  application's build configuration currently carries its own version number; if it is not
  derived from the tag at build time, two sources of truth exist and will diverge.
- **A user on the architecture that was not built.** Every published version must serve
  both currently supported Mac architectures, or the install must fail loudly rather than
  installing something that cannot run.
- **A signing or notarization credential that expires between releases.** Must surface as
  a fast, named pre-flight failure rather than as a release that publishes an app users
  cannot open.
- **An install-channel credential that only permits writing to the channel.** The
  credential used to publish the install channel must not be able to write anything else.
- **A user who installed by hand, from a locally built bundle, before this existed.** The
  documented install must not silently collide with or overwrite that copy's stored site
  list.

## Requirements *(mandatory)*

### Functional Requirements

**Install & uninstall (US1)**

- **FR-001**: Site Checker MUST be installable on macOS with the single documented command
  `brew install clintcparker/tap/site-checker`, requiring no source checkout and no
  language toolchain.
- **FR-002**: The published install entry MUST install the macOS **application bundle** —
  not a command-line binary — while keeping the advertised install command above unchanged.
  **Amended 2026-08-11.** This previously required a *cask* specifically. That is now
  unsatisfiable at acceptable cost: casks quarantine unconditionally (see `research.md` R2's
  decision record), so a cask forces the $99/yr membership FR-006 no longer assumes. The
  requirement's intent — ship the app, not a CLI, behind one unchanged command — is
  preserved; the mechanism is a formula.
- **FR-003**: The install entry MUST support uninstalling the application, and the project
  MUST document how to remove the application's stored data directory
  (`~/Library/Application Support/com.clintparker.site-checker`, which contains
  `sites.json`).
  **Amended 2026-08-11.** This previously required the install entry itself to *offer* a
  data-removing uninstall. `zap` is a cask stanza with no formula equivalent —
  `brew uninstall --zap` on a formula exits 0 having done nothing — so the capability does
  not exist to deliver. It degrades to documentation, in the README and in the formula's
  `caveats`. Recorded as a deliberate loss, not an oversight.
- **FR-004**: The plain uninstall MUST leave the stored site list in place, so a
  re-install restores the user's sites. **Now satisfied by construction:** a formula never
  touches the data directory under any flag.
- **FR-005**: Every published version MUST be installable and natively runnable on both
  Apple Silicon and Intel Macs.
- **FR-006**: A freshly installed copy MUST open on a machine other than the one that built
  it without the operating system reporting it as damaged or blocking it.
  **Resolved 2026-08-11** (see `research.md` R2's decision record), reversing the plan-time
  answer. There is no Apple Developer Program membership and no notarization. The
  requirement is met by shipping a **formula**, which never sets `com.apple.quarantine` —
  so Gatekeeper never evaluates the bundle and never reports it as damaged. The plan-time
  answer assumed the alternative was `quarantine: false` in a cask; that stanza does not
  exist in current Homebrew, which is what forced the channel change rather than the
  cheaper fix.

**Release by tag (US2)**

- **FR-007**: Publishing a version MUST require exactly one maintainer action: pushing one
  annotated tag of the form `v<MAJOR>.<MINOR>.<PATCH>`.
- **FR-008**: The version MUST be derived from that tag (`v1.2.3` → `1.2.3`) and MUST be
  written into the application's build configuration at build time, so no file in the
  repository holds a version number that a human maintains.
- **FR-009**: A pushed tag that does not match the required form MUST NOT trigger a
  release.
- **FR-010**: A release MUST publish, for the tagged version, one installable artifact per
  supported architecture plus automatically generated release notes.
- **FR-011**: The install channel MUST be updated from the artifacts actually attached to
  the published release — including their real checksums — rather than from locally
  predicted names or checksums.
- **FR-012**: The install-channel entry MUST be generated from a template kept in this
  repository. The template MUST be marked as the canonical source and as never-edit-by-hand
  at its destination.
- **FR-013**: Rendering MUST fail the release both when the template is missing a
  placeholder the automation expects, and when any placeholder survives unrendered into the
  published output.
- **FR-014**: Re-running a release for an already-published version MUST converge: the
  existing release is updated in place, and the install channel is not rewritten when the
  rendered content is unchanged.
- **FR-015**: The credential used to publish the install channel MUST be scoped to write
  only the install-channel repository (`clintcparker/homebrew-tap`) and nothing else.
- **FR-016**: The release MUST NOT hang in an environment without an interactive desktop
  session; any packaging step that depends on desktop GUI automation MUST be avoided,
  skipped, or replaced there.

**Pre-flight (US3)**

- **FR-017**: Before building anything, a release MUST verify that every required one-time
  credential is present, and MUST fail within seconds naming the missing piece when one is
  not.
- **FR-018**: The pre-flight failure MUST occur before any build, release publication, or
  install-channel write, so a partially published state cannot result from missing setup.

**Channel verification (US4)**

- **FR-019**: A verification pass MUST run on a schedule, automatically after each release,
  and on demand.
- **FR-020**: Verification MUST resolve the latest published version, confirm every
  artifact the install channel references is downloadable, confirm the install channel
  advertises that same version, **and confirm the channel's recorded checksums match the
  bytes actually published** — failing when any channel lags or diverges. (The checksum
  clause was added 2026-08-11, closing review finding R001: version and URL can both be
  correct while the checksums are stale, which is exactly the state a partial re-run
  leaves, and `brew install` aborts on it while verification stayed green.)
- **FR-021**: Verification MUST include an end-to-end step that installs the application on
  a clean macOS environment the same way a user would, and confirms the installed bundle is
  **not quarantined** and that its **signature verifies**.
  **Amended 2026-08-11.** This previously required confirming "the operating system accepts
  the installed application for execution" — in practice `spctl -a -t exec`, which cannot
  pass without a Developer ID and would therefore only ever fail. The two clauses above are
  what remains checkable: the quarantine assertion is the premise of choosing a formula,
  made machine-checked rather than assumed, and `codesign --verify --strict` catches the
  install path modifying a bundle that CI sealed. It is a real check of a strictly weaker
  property, and the loss of provenance is recorded in `docs/ROADMAP.md`.

**Continuous integration (US5)**

- **FR-022**: The Rust test suite, the Rust lint gate with warnings treated as errors, the
  frontend test suite, and the frontend build MUST run automatically on pushes to the
  default branch and on every pull request.
- **FR-023**: These checks MUST be separate from the release pipeline; the release pipeline
  MAY assume such a suite exists and run its own test gate before building.

**Documentation (US6)**

- **FR-024**: A how-to document MUST describe the release procedure, the one-time setup
  checklist, and how to re-run a failed or partially completed release.
- **FR-025**: `docs/ROADMAP.md` §2 MUST be updated to record what shipped and what, if
  anything, was deliberately left, consistent with how §§1–3 record previously completed
  sections.

**Bundle size (roadmap §2, standalone item)**

- **FR-026**: The mismatch between the shipped bundle's actual size (~15 MB) and the v1
  specification's stated "single-digit MB" expectation MUST be resolved so that no
  project document states a size the product does not meet.
  **Resolved at plan time** (see `research.md` R7): amend the stated expectation. The one
  document that states a size is `docs/superpowers/plans/2026-07-23-site-checker.md:2554`,
  whose line exists to prompt an investigation if the number came in high; the
  investigation happened and attributed the size to `aws-lc-rs` (confirmed by `cargo
  tree`). Changing the TLS backend is rejected — it alters the code path behind every
  check the product makes, which this feature's Out of Scope forbids.

**Scope of this feature's completion**

- **FR-027**: This feature is complete only once a real `v1.0.0` tag has been pushed and
  `brew install clintcparker/tap/site-checker` verifiably works on a Mac that did not build
  it. **Resolved at plan time** (see `research.md` R10): every success criterion here is a
  statement about a published artifact rather than a merged file — SC-001 times a real
  install, SC-002 asserts what a real user does not see, SC-004 claims something about
  100% of published versions — and the failure class this feature exists to prevent
  (green release, broken install) is invisible until the first real install.

### Constraints discovered at plan time

Two blockers not visible when this spec was written. Both are recorded here because they
change what "shipped" requires, not merely how it is built:

- **The repository is private, and a Homebrew cask cannot install from a private
  repository.** `brew` downloads a cask's `url` unauthenticated, so every install would
  404. FR-001 is unreachable until `clintcparker/site-checker` is public. Making it public
  is effectively irreversible and exposes the full history — a disclosure decision for the
  user, not the plan. See `research.md` R1, which also covers the GitHub Actions cost
  argument (macOS minutes bill at 10× on private repositories) and the fallback scope if
  the answer is no.
- **`docs/` is gitignored** (`.gitignore:25`), so FR-024's `docs/how-to/release.md` would
  never be committed and SC-011 would be unsatisfiable. Resolved by narrowing the ignore to
  `docs/*` plus `!docs/how-to/`; git cannot re-include a file whose parent directory is
  excluded, so the negation only works in that shape. See `research.md` R8. FR-025's and
  FR-026's edits stay inside the ignored area and must be applied in the primary checkout
  rather than the feature worktree.

### Out of Scope

- **Windows and Linux distribution.** Site Checker is a macOS application; no other
  platform is served by this feature.
- **Any change to what the application does.** This feature changes how the application is
  built, published, and installed — not its behaviour, its stored data format, or its
  interface. The constitution's "One Mac, One Person" scope is unchanged.
- **Auto-update from inside the application.** Updating is the install channel's job here.
- **Publishing to any channel other than the existing `clintcparker/homebrew-tap`.**
- **The roadmap's §3 test-coverage gaps and §4 v2 features**, which are separate sections.

### Key Entities

- **Version tag**: An annotated `v<MAJOR>.<MINOR>.<PATCH>` tag on the default branch. The
  single source of truth for the version of a given release.
- **Release**: The published, immutable-by-convention record of one version, carrying one
  installable artifact per architecture plus generated notes.
- **Install-channel template**: The canonical, human-edited description of how the install
  channel should look, kept in this repository, carrying version and per-architecture
  checksum placeholders.
- **Install-channel entry**: The rendered result of that template, written only by
  automation into the public tap repository, never edited there.
- **One-time setup credentials**: The named secrets and external accounts a release
  requires — the install-channel write token, plus whatever FR-006's answer requires.
- **Stored data directory**: `~/Library/Application Support/com.clintparker.site-checker`,
  the only location the application owns; relevant here because uninstall must be able to
  remove it deliberately and must not remove it accidentally.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A person on a Mac who has never built this project can go from "nothing
  installed" to "app running and checking a site" using one documented install command and
  no source checkout, in under 5 minutes on a typical connection.
- **SC-002**: Installing and launching requires zero security-override steps by the user —
  no right-click-open, no settings change, no manual approval dialog to dismiss.
- **SC-003**: Publishing a new version requires exactly one maintainer action and zero
  file edits; the number of files in the repository containing a hand-maintained version
  number is zero.
- **SC-004**: 100% of published versions install and run natively on both Apple Silicon and
  Intel Macs.
- **SC-005**: When required one-time setup is missing, the release run reports which piece
  is missing in under 60 seconds and produces no published artifacts.
- **SC-006**: A published version whose install channel lags, or whose artifacts are
  unreachable, is detected automatically within 24 hours and immediately after any release.
- **SC-007**: Re-running a release for the same version produces exactly one release for
  that version and no duplicate or conflicting install-channel entries.
- **SC-008**: 100% of pull requests get an automated pass/fail result for all four quality
  gates named in FR-022, with no local action by the author.
- **SC-009**: No automated build hangs waiting for desktop interaction; every release and
  CI run reaches a definite pass or fail without human intervention.
- **SC-010**: Uninstalling leaves the user's site list fully intact and restorable by
  re-installing, and the documented data-removal command leaves no application data on disk.
  **Amended 2026-08-11**, following FR-003: there is no data-removal *option* on a formula
  to measure, so the first clause is now unconditional and the second is measured against
  the documented `rm -rf`. Review finding R009 — that the cask's `zap trash:` moved data to
  the Trash rather than deleting it, so "leaves no application data on disk" was not
  literally met until the Trash was emptied — is moot: nothing moves anything to the Trash
  any more.
- **SC-011**: A maintainer who has never released this project can complete a release using
  only the written procedure.
- **SC-012**: No project document states a bundle size, or any other published expectation,
  that the delivered product does not meet.

## Assumptions

- **The public tap repository already exists.** `clintcparker/homebrew-tap` is public and
  already serves `name-on`; Site Checker is a second entry in it, not new infrastructure.
- **Conventions are mirrored from `clintcparker/name-on`**, which has already proven this
  shape: a template in the source repository, a tag-triggered release pipeline whose
  pre-flight fails fast, a separate channel-verification pass, and a how-to document. The
  one deliberate divergence is cask-vs-formula (FR-002), because Site Checker ships an
  application bundle rather than a command-line binary.
- **Per-architecture artifacts, not a universal binary.** The default is one artifact per
  architecture, matching the existing per-platform asset convention. A single universal
  artifact would collapse the build matrix but diverge from that convention; if the plan
  finds the matrix disproportionately expensive, it may revisit this, but the user-facing
  outcome (FR-005) is identical either way.
- **The first tag will be `v1.0.0`.** v1 has shipped and the build configuration's stale
  `0.1.0` is exactly the hand-maintained version number FR-008 eliminates.
- **Users have Homebrew installed.** Bootstrapping Homebrew itself is the user's business
  and is documented by Homebrew.
- **Secrets do not cross repositories**, so any token needed to write the tap must be
  configured on this repository as well as existing elsewhere.
- **Release automation runs on hosted runners**, including a macOS runner for building and
  for the end-to-end install check — which is why FR-016's headless constraint exists.
- **No existing user base is being migrated.** Nobody has installed Site Checker through a
  published channel yet, so there is no upgrade path to preserve — only the hand-built
  local copies described in the last edge case.
