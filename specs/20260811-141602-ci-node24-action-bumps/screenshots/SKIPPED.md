# Screenshots skipped — no UI surface

This feature only bumps GitHub Actions pins to Node 24 majors and adds `.github/dependabot.yml`; it changes nothing a user sees in the app window.

## How this was decided (unattended run, mode `before`, target `main`)

There is no `spec.md` or `plan.md` — this feature bypassed `/speckit-specify` and `/speckit-plan` — so UI-relevance was judged from `tasks.md` instead. Every file path named across T001–T020 is one of:

- `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/workflows/verify-install-channels.yml`
- `.github/dependabot.yml`
- `docs/how-to/release.md`

None of `src/`, `index.html`, or `src-tauri/tauri.conf.json` is touched, and no task changes user-visible output such as status `reason` strings or the shape of the `site-status` event. `tasks.md` states this directly under "Path Conventions": *"This feature touches CI configuration only... No `src/`, `src-tauri/`, or `tests/` code changes."*

Because the decision was reachable without launching the app, no `pnpm tauri dev` run and no `sites.json` seeding took place — the user's real data file was never touched, so no backup/restore was required.

## Prediction for the `after` run

`git diff --name-only $(git merge-base HEAD main)..HEAD -- src index.html src-tauri/tauri.conf.json` should be empty. If it is not, this file gets deleted, and the `after` capture records `"baseline": "unavailable"` since no `before/` images exist.

## Verification (unattended run, mode `after`, target `main`)

Prediction held — the `after` run captured nothing and this file stands.

The first `after` pass ran while the feature's work was still uncommitted: the merge-base against `main` (`bd7a559`) was also `HEAD`, so the committed diff was empty *trivially* and proved nothing on its own. That pass fell back to checking the working tree. Later passes ran against real commits. This re-verification is at `HEAD` = `418a9e9`, which includes the review fixes that landed after the earlier passes (`9eb0cc6` and its successors), so the committed diff is meaningful:

- `git diff --name-only bd7a559..HEAD -- src index.html src-tauri/tauri.conf.json` → empty
- `git diff --name-only bd7a559..HEAD -- src src-tauri index.html` → empty (widened to *all* of `src-tauri/`, not just `tauri.conf.json`)
- `git status --porcelain -- src src-tauri index.html` → empty (no uncommitted UI work either)

The full committed diff is `.github/dependabot.yml`, `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `CHANGELOG.md`, `docs/how-to/release.md`, plus this feature's own `specs/` artifacts. No UI surface, so no `before/` baseline was needed and no `after/` images exist. The only untracked file is a QA record under `specs/`.

No `manifest.json` exists to reuse: the `before` pass skipped rather than capturing, so it never wrote one. Nothing about the app's state needed to be reproduced.

As in the `before` pass, the app was never launched and `sites.json` was never seeded, backed up, or restored — the decision needed no running app, so the user's real data file was never at risk.

## Re-verification (unattended run, mode `before`, target `main`, post-merge)

This `before` run was invoked after the feature had already merged — PR #27 landed at `88ad5c6`, and `main` has since advanced to `457f572` (PR #28, an unrelated Dependabot Actions bump). A baseline is therefore not merely unnecessary here but unobtainable from the checkout as it stands: `main`'s tip *is* the after state, so any capture taken now would document the post-implementation UI, not a pre-implementation one. The skip stands on its own merits regardless, so no throwaway worktree was built at the pre-implementation SHA.

The decision was re-checked against the full merged range rather than the working tree. The merge-base of PR #27 is `b64c0e2`:

- `git diff --name-only b64c0e2..903d524 -- src index.html src-tauri/tauri.conf.json` → empty
- `git diff --name-only b64c0e2..457f572 -- src src-tauri index.html` → empty (widened to all of `src-tauri/`, and extended through the current `main` tip)
- `git status --porcelain -- src src-tauri index.html` → empty
- Full non-`specs/` diff for the range: `.github/dependabot.yml`, `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `CHANGELOG.md`, `docs/how-to/release.md`

Two process notes for whoever reads this next:

- `.specify/scripts/bash/check-prerequisites.sh --json` hard-fails on this feature (`ERROR: plan.md not found`), because the feature deliberately bypassed `/speckit-specify` and `/speckit-plan`. `FEATURE_DIR` was read from `.specify/feature.json` instead, and UI-relevance judged from `tasks.md` as in the original `before` pass.
- Commit `c51c1aa` on branch `20260811-141602-ci-node24-action-bumps` carries an earlier draft of this same note and is **not** an ancestor of `main`; it never merged. This section supersedes it, on `main`, where the rest of the feature's artifacts live.

No app launch, no `sites.json` seeding, no backup, no restore — same as both prior passes.

## Re-verification (unattended run, mode `after`, target `main`, post-merge, uncommitted round)

This `after` run was invoked on `main` at `59db8a8` — the commit that recorded the
previous `before` re-verification — with a further round of this feature's work
still uncommitted in the working tree (the `dependabot.yml` ignore-syntax fix, the
`download-artifact@v8` `digest-mismatch: warn` pin, and the accompanying
`CHANGELOG.md` / `docs/how-to/release.md` / `ci.yml` / `release.yml` prose). The
skip stands: none of it touches a UI surface.

Because branch == target == `main`, `git merge-base HEAD main` resolves to `HEAD`
itself, so the committed diff is empty *trivially* and proves nothing on its own —
the same trap the first `after` pass hit. Both the meaningful committed range and
the working tree were therefore checked directly:

- `git diff --name-only $(git merge-base HEAD main)..HEAD -- src index.html src-tauri/tauri.conf.json` → empty (trivially; recorded for completeness)
- `git diff --name-only b64c0e2..HEAD -- src src-tauri index.html` → empty (feature merge-base through current tip, widened to all of `src-tauri/`)
- `git status --porcelain -- src src-tauri index.html` → empty (no uncommitted UI work)

The full uncommitted set is `.github/dependabot.yml`, `.github/workflows/ci.yml`,
`.github/workflows/release.yml`, `CHANGELOG.md`, `docs/how-to/release.md`, plus
this feature's own `specs/` artifacts. The `release.yml` change adds a
`digest-mismatch: warn` input to a workflow step; it changes CI behavior, not the
app's status `reason` strings or the shape of the `site-status` event, so it does
not meet the "user-visible output" exception either.

### On reusing the `before` manifest

The run request asked that this pass reuse the manifest the `before` pass wrote,
so the pair would be comparable. There is no such manifest, and there never was:
every `before` pass on this feature skipped rather than captured, so none of them
wrote `manifest.json` or seeded any app state. There is nothing to reproduce and
no baseline to pair against — `SKIPPED.md` is the whole record. Had implementation
turned out to touch UI after all, this pass would have deleted this file and
captured `after/` only with `"baseline": "unavailable"`.

The app was not launched and did not need to be, so its build/start health is
untested by this run and this file should not be read as evidence either way.
`sites.json` was never seeded, backed up, or restored — the user's real data file
was never at risk, in this pass or any prior one.
