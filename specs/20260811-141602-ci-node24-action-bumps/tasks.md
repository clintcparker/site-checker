---
description: "Task list for retiring the Node.js 20 deprecation warning in GitHub Actions"
---

# Tasks: Retire the Node.js 20 Deprecation Warning in CI

**Input**: No design documents exist for this feature. Tasks were derived directly from the
CI annotation on [run 31536128187](https://github.com/clintcparker/site-checker/actions/runs/31536128187)
plus a live audit of every action pin in `.github/workflows/`.

**Prerequisites**: None. This feature bypassed `/speckit-specify` and `/speckit-plan`, so there is
no `spec.md` or `plan.md` in this directory. See "Provenance" below.

**Tests**: No automated test tasks. The verification surface for this change is the GitHub Actions
annotation list itself, which cannot be asserted locally — verification tasks call for reading the
annotations off a real run.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

This feature touches CI configuration only. All edited paths are under `.github/` at the repository
root. No `src/`, `src-tauri/`, or `tests/` code changes.

---

## Provenance

The annotation on the linked run reads:

> Node.js 20 is deprecated. The following actions target Node.js 20 but are being forced to run on
> Node.js 24: `actions/checkout@v4`. For more information see:
> https://github.blog/changelog/2025-09-19-deprecation-of-node-20-on-github-actions-runners/

The `frontend` job on the same run carries the same warning naming three actions:
`actions/checkout@v4`, `actions/setup-node@v4`, `pnpm/action-setup@v4`.

**The run itself succeeded.** This is a deprecation warning, not a failure — the runner is already
force-upgrading these actions to Node 24. The work is to make the pins say what is already
happening, before GitHub stops force-upgrading.

### Audit baseline (verified 2026-08-11)

| Action | Pinned | Runtime at pin | Target | Why this target |
|---|---|---|---|---|
| `actions/checkout` | v4 | node20 | **v5** | Pure node24 bump, no behavior change. v6 changes credential persistence; v7 adds ESM + a fork-PR checkout block — neither is needed here. |
| `actions/setup-node` | v4 | node20 | **v6** | node24. v5 added automatic caching from `packageManager`; v6 narrows that to npm only. Both workflows already pass `cache: pnpm` explicitly, so neither change reaches this repo. |
| `pnpm/action-setup` | v4 | node20 | **v6** | v5 is the node24 bump; v6 adds pnpm 11 support. |
| `actions/upload-artifact` | v4 | node20 | **v6** | v5 is still node20 — it only had *preliminary* Node 24 support. v6 is the first default-node24 major. |
| `actions/download-artifact` | v4 | node20 | **v7** | v5 and v6 are both still node20. v8 makes digest mismatch a hard failure and changes decompression behavior — avoid on the release path. |
| `softprops/action-gh-release` | v2 | node20 | **v3** | v3 is the node24 bump; v2 stays on the Node 20 line. |
| `Swatinem/rust-cache` | v2 | **node24** | — | Already compliant. Do not touch. |
| `dtolnay/rust-toolchain` | stable | composite | — | Not a JS action. No runtime to bump. |

`.github/workflows/verify-install-channels.yml` contains **no** `uses:` steps — it is pure shell.
It needs no changes and is out of scope for every story below.

All target majors require Actions runner ≥ 2.327.1. Every job in this repo runs on
`ubuntu-latest` or `macos-latest` (GitHub-hosted), which are well past that.

---

## Phase 1: Setup

**Purpose**: Freeze the audit so the edits below can be checked against something.

- [X] T001 Re-run the pin inventory with `grep -rn "uses:" .github/workflows/` and confirm it still matches the audit baseline table in `specs/20260811-141602-ci-node24-action-bumps/tasks.md` (8 distinct actions, 20 `uses:` lines across `ci.yml` and `release.yml`) — the count read "7 distinct actions" until 2026-08-12; the baseline table above it has always listed 8, and `grep -rn "uses:" .github/workflows/` confirms 8. The 20-line count was always correct.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Confirm the one assumption every story depends on.

- [X] T002 Confirm no self-hosted runners are configured for this repository via `gh api repos/clintcparker/site-checker/actions/runners`, since every target major requires runner ≥ 2.327.1 and only GitHub-hosted runners are guaranteed to satisfy it

**Checkpoint**: Runner compatibility established — US1 and US2 may proceed in parallel.

---

## Phase 3: User Story 1 - CI runs warning-free (Priority: P1) 🎯 MVP

**Goal**: The `CI` workflow — the one the user linked — produces zero Node.js 20 deprecation
annotations on both its `rust` and `frontend` jobs.

**Independent test**: Push the branch, open a PR, and read the annotations for both jobs of the
resulting `CI` run. Both must be empty of "Node.js 20 is deprecated" messages, and both jobs must
still conclude `success`.

**Why this is the MVP**: It closes exactly the run the user pointed at. `release.yml` (US2) only
emits its warnings on tag pushes, so it is invisible until a release and can ship separately.

> **Line numbers below shifted after T018.** These tasks were written against the pre-T018
> `ci.yml`, where the pins sat at lines 28, 71, 73, and 75. T018 inserted the pinning-convention
> comment block above them, so the current locations are **47, 90, 92, and 94**. Both sets are
> given so a future reader diffing the task list against the file does not conclude a task hit
> the wrong line. `release.yml`'s citations in US2 are unaffected and still exact.

- [X] T003 [US1] Bump `actions/checkout@v4` → `@v5` on both lines 28 and 71 (now 47 and 90) of `.github/workflows/ci.yml` (the `rust` and `frontend` jobs)
- [X] T004 [US1] Bump `pnpm/action-setup@v4` → `@v6` on line 73 (now 92) of `.github/workflows/ci.yml`
- [X] T005 [US1] Bump `actions/setup-node@v4` → `@v6` on line 75 (now 94) of `.github/workflows/ci.yml`, leaving the `node-version: 22` and `cache: pnpm` inputs exactly as they are — the explicit `cache` input is what makes the v5/v6 auto-caching change a no-op here
- [X] T006 [US1] Validate the edited `.github/workflows/ci.yml` parses as YAML (e.g. `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml'))"`) and confirm no `@v4` pin remains except intentionally unchanged actions
- [X] T007 [US1] Push the branch and confirm via `gh api repos/clintcparker/site-checker/check-runs/<id>/annotations` that both `CI` jobs return `[]` and both conclude `success` — **done during ship**: PR #27 triggered run [31539366957](https://github.com/clintcparker/site-checker/actions/runs/31539366957); `rust` → `success` with `annotations: 0`, `frontend` → `success` with `annotations: 0`. The Node 20 deprecation warning is gone from `CI`. US1's success criterion is met on a live runner, not by inspection.

**Checkpoint**: The linked run's warning is gone. This is independently shippable.

---

## Phase 4: User Story 2 - Release runs warning-free (Priority: P2)

**Goal**: The `Release` workflow emits no Node.js 20 deprecation annotations across all five of its
jobs, so the next tag push is clean.

**Independent test**: Inspect the annotations of the next `Release` run (or a `workflow_dispatch`
dry run) for "Node.js 20 is deprecated". None of `preflight`, `test`, `build`, `release`, or
`homebrew` may carry one.

**Independent of US1**: Different file. `release.yml` can be edited and reviewed without any US1
task being complete, and vice versa.

**Note**: `release.yml` carries three actions that `ci.yml` does not — `upload-artifact`,
`download-artifact`, and `action-gh-release` — and these are *not* named in the linked run's
annotation because `CI` never invokes them. They are the same defect on a path that only runs at
release time.

- [X] T008 [P] [US2] Bump `actions/checkout@v4` → `@v5` on lines 93, 139, and 264 of `.github/workflows/release.yml` (the `test`, `build`, and `homebrew` jobs)
- [X] T009 [US2] Bump `pnpm/action-setup@v4` → `@v6` on lines 103 and 150 of `.github/workflows/release.yml`
- [X] T010 [US2] Bump `actions/setup-node@v4` → `@v6` on lines 105 and 152 of `.github/workflows/release.yml`, preserving the `node-version: 22` and `cache: pnpm` inputs on both
- [X] T011 [US2] Bump `actions/upload-artifact@v4` → `@v6` on line 218 of `.github/workflows/release.yml`, preserving `name`, `path`, and `if-no-files-found: error`
- [X] T012 [US2] Bump `actions/download-artifact@v4` → `@v7` on line 231 of `.github/workflows/release.yml`, preserving `path: artifacts` — and confirm v7 still unzips by default, since the `release` job's `files:` globs depend on artifacts landing unpacked at `artifacts/site-checker-<arch>/site-checker-<arch>-apple-darwin.zip`
- [X] T013 [US2] Bump `softprops/action-gh-release@v2` → `@v3` on line 238 of `.github/workflows/release.yml`, preserving `generate_release_notes`, `fail_on_unmatched_files: true`, and the two-line `files:` list
- [X] T014 [US2] Validate the edited `.github/workflows/release.yml` parses as YAML and confirm the `Swatinem/rust-cache@v2` pins on lines 99 and 145 were left untouched (v2 is already node24)
- [ ] T015 [US2] Verify the release path end to end by pushing a throwaway prerelease tag, then confirm zero Node 20 annotations, that both arch artifacts attach to the release, and that the `homebrew` job's `gh release download` still resolves the asset names

**Checkpoint**: Both workflows are Node 24 clean.

---

## Phase 5: User Story 3 - The warning does not silently return (Priority: P3)

**Goal**: A future Node runtime deprecation surfaces as a dependency PR rather than as an annotation
nobody reads.

**Independent test**: Confirm Dependabot opens (or would open) a PR for a stale action pin — visible
under the repository's Dependabot "Last checked" status for the `github-actions` ecosystem.

**Independent of US1 and US2**: New file, no overlap. It is worth doing even if the bumps above are
deferred.

- [X] T016 [P] [US3] Create `.github/dependabot.yml` with a `package-ecosystem: "github-actions"` entry at `directory: "/"` on a `weekly` schedule, so action majors are proposed as PRs instead of accumulating silently
- [X] T017 [US3] Confirm Dependabot accepted the config via `gh api repos/clintcparker/site-checker/dependabot/alerts` or the repository's Insights → Dependency graph → Dependabot tab, and that no parse error is reported against the new file — **closed with a defect, 2026-08-12**. Dependabot accepted the file and reported no parse error, and that turned out to be the problem rather than the reassurance: the `ignore:` block's `8.x`-style values parse as *versions*, not ranges, in the github-actions ecosystem's Gem requirement grammar, so all four entries compiled to equalities against versions that will never exist and matched nothing. Proven in production — Dependabot opened [PR #28](https://github.com/clintcparker/site-checker/pull/28) 64 seconds after the config landed on `main`, proposing all four majors the file declined, and it merged. The values are now bare integers, which match the bare major tag Dependabot reports. See [review-20260812-110430.md](reviews/review-20260812-110430.md) R001. Re-verification that the corrected form holds requires one more weekly Dependabot cycle on `main` and is the one thing still outstanding on US3.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T018 [P] Record the "pin actions to majors that run on the current Node runtime" convention alongside the existing CI reasoning comments at the top of `.github/workflows/ci.yml`, matching the file's existing explain-the-why comment style
- [X] T019 [P] Note in `docs/how-to/release.md` that `release.yml` action pins are now Node 24 and that a runner-version floor of 2.327.1 applies, so a future self-hosted runner is not introduced unknowingly — **first confirm this file is tracked**, since `docs/` is gitignored except `how-to/`
- [X] T020 Re-read the annotations on the newest `CI` run on `main` after merge and confirm an empty annotation list, closing out the report that started this feature — **confirmed twice**: run [31544064523](https://github.com/clintcparker/site-checker/actions/runs/31544064523) at `88ad5c6` (the merge itself) and run [31544339526](https://github.com/clintcparker/site-checker/actions/runs/31544339526) at `457f572` (after PR #28 moved four pins) both concluded `success` on `rust` and `frontend` with both annotation lists returning `[]`. The warning that started this feature is gone from `main` and stayed gone across the pin change.

---

## Dependencies

```
T001 (setup)
  └─> T002 (foundational: runner check)
        ├─> US1: T003 → T004 → T005 → T006 → T007      [ci.yml]
        ├─> US2: T008 → T009 → T010 → T011 → T012 → T013 → T014 → T015   [release.yml]
        └─> US3: T016 → T017                            [dependabot.yml]

Polish: T018 (after US1) · T019 (after US2) · T020 (after US1 merges)
```

**Story completion order**: US1 → US2 → US3 by priority, but all three are independent and may be
done in any order or concurrently.

**Within-story ordering is sequential, not incidental**: T003–T005 and T008–T013 all edit a single
file each, so they must not be parallelized against one another despite being distinct actions.

## Parallel Execution Opportunities

- **Across stories**: US1 (`ci.yml`), US2 (`release.yml`), and US3 (`dependabot.yml`) touch three
  disjoint files and can be executed by three agents concurrently once T002 clears.
- **T008** is marked `[P]` because it is the entry point of the US2 chain and conflicts only with
  later US2 tasks, not with US1 or US3.
- **T016** is marked `[P]` because it creates a new file that nothing else touches.
- **T018 and T019** target different files and can run together.
- **Not parallelizable**: every task within US1 after T003, and within US2 after T008 — same-file
  edits.

## Implementation Strategy

**MVP = User Story 1 alone.** Five tasks, one file, and it closes the exact run that was reported.
Ship it as its own PR.

**Increment 2 = User Story 2.** Larger and riskier than US1 because T015 requires an actual tag
push to verify the artifact round-trip. Worth a separate PR so a release-path regression is
bisectable to one commit.

**Increment 3 = User Story 3.** Preventive, not corrective. Can trail the other two indefinitely
without blocking anything.

## Deliberately Out of Scope

> **Four of these were crossed anyway — updated 2026-08-12.** The entries below are this feature's
> scope decision and remain the record of what was evaluated and declined. They are **not** a
> description of `main`. [PR #28](https://github.com/clintcparker/site-checker/pull/28), a Dependabot
> group bump opened 64 seconds after PR #27 merged and merged three minutes later, moved the tree onto
> `checkout@v7`, `setup-node@v7`, `upload-artifact@v7`, and `download-artifact@v8` — every major the
> last two bullets decline. It was only proposed because `.github/dependabot.yml`'s `ignore:` syntax
> was inert (see T017). The bump was kept rather than reverted. `ci.yml`'s comment block now carries
> the per-major evaluation that should have preceded it, and the one real behavior change —
> `download-artifact@v8` defaulting `digest-mismatch` to `error` — is pinned back to `warn` on the
> `release` job. Read the bullets below as history, and `.github/workflows/` as the current state.

- `Swatinem/rust-cache@v2` and `dtolnay/rust-toolchain@stable` — already compliant. *(Still true.)*
- `.github/workflows/verify-install-channels.yml` — contains no actions. *(Still true.)*
- `actions/download-artifact@v8` and `actions/upload-artifact@v7` — the newest majors, skipped
  because v8 turns digest mismatches into hard failures and changes decompression. Revisit only if
  direct-upload behavior is wanted. — **Superseded.** Both are on `main`. v8's decompression change
  turned out to be narrower than this bullet implies: it skips unzipping only for content whose
  `Content-Type` is not a zip, and `skip-decompress` defaults to `false`, so the `release` job's
  `artifacts/site-checker-<arch>/…` globs still resolve. The digest default is real and is pinned
  back to `warn` at `release.yml`. `upload-artifact@v7`'s direct upload is opt-in (`archive` defaults
  to `true`).
- `actions/checkout@v6`/`@v7` and `actions/setup-node@v7` — newer than the chosen targets. Chosen
  targets already resolve the warning; jumping further adds behavior changes this feature has no
  reason to absorb. — **Superseded.** Both are on `main`. Evaluated after the fact and inert here:
  `checkout@v7`'s fork-PR block only fires on `pull_request_target`/`workflow_run`, which no workflow
  in this repo uses, and the `homebrew` job pushes with `TAP_PUSH_TOKEN` rather than a persisted
  checkout credential; `setup-node@v7` inherits v6's npm-only caching narrowing, which all three call
  sites bypass by passing `cache: pnpm` explicitly.

---

## Verification status (recorded 2026-08-11)

### Audit baseline re-confirmed, not assumed

The target column of the audit table was re-verified against the live tags before any edit, by
reading each action's `action.yml` at the target ref rather than trusting the table:

| Action | Target | `runs.using` at that tag |
|---|---|---|
| `actions/checkout` | v5 | `node24` |
| `actions/setup-node` | v6 | `node24` |
| `pnpm/action-setup` | v6 | `node24` |
| `actions/upload-artifact` | v6 | `node24` |
| `actions/download-artifact` | v7 | `node24` |
| `softprops/action-gh-release` | v3 | `node24` |
| `Swatinem/rust-cache` | v2 | `node24` — already compliant, left untouched |

Input compatibility was checked the same way, at the target ref:

- `actions/setup-node@v6` still accepts `node-version` and `cache`. Both workflows pass `cache: pnpm`
  explicitly, which is what makes the v5/v6 auto-caching change a no-op here (T005, T010).
- `pnpm/action-setup@v6` leaves `version` optional and falls back to `packageManager` in
  `package.json`, which this repo pins to `pnpm@10.30.3`. No `version` input needed (T004, T009).
- `actions/upload-artifact@v6` still accepts `name`, `path`, and `if-no-files-found` (T011).
- `actions/download-artifact@v7` still defaults `merge-multiple: false`, documented as "extracted
  into individual named directories within the specified path". So artifacts still land unpacked at
  `artifacts/site-checker-<arch>/site-checker-<arch>-apple-darwin.zip` and the `release` job's
  `files:` globs still resolve. This was T012's open question — resolved, no glob change needed.
- `softprops/action-gh-release@v3` still accepts `generate_release_notes`,
  `fail_on_unmatched_files`, and `files` (T013).

`gh api repos/clintcparker/site-checker/actions/runners` returns `total_count: 0` — no self-hosted
runners, so the ≥ 2.327.1 runner floor is satisfied by every job (T002).

### Why four tasks remained open (updated 2026-08-12 — one still is)

Recorded before merge; three of the four have since closed. **T007** closed during ship on PR #27's
first `CI` run. **T020** closed post-merge, confirmed on two separate `main` runs. **T017** closed
with a defect — it found that the `ignore:` block was inert, which is exactly the failure it existed
to catch, except that it ran a day late and Dependabot found it first. **T015 is the one that is
still genuinely open**, and PR #28 raised its stakes: the release path is now two majors past the
set that was evaluated for it, and `download-artifact@v8`'s hard-fail digest default has been pinned
back to `warn` by inspection, not by a run. See
[review-20260812-110430.md](reviews/review-20260812-110430.md).

The original note follows.

These four cannot be closed from a local checkout. They are not skipped — they are blocked on
something that has to happen on GitHub first.

- **T007** — `ci.yml` triggers on `push` to `main` and on `pull_request` only. Pushing the feature
  branch by itself starts no run, so there are no annotations to read until a PR exists. Closes on
  the first `CI` run of the PR.
- **T015** — requires pushing a real tag, which publishes a real GitHub Release and writes to
  `clintcparker/homebrew-tap`. That is a release action, not an implementation step, and was left
  for an explicit decision rather than taken unilaterally. Until it runs, the `release.yml` changes
  are verified by inspection (table above) but not end to end.
- **T017** — Dependabot only parses `.github/dependabot.yml` after it lands on the default branch.
  Closes after merge.
- **T020** — by definition post-merge.
