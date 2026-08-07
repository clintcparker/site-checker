# Changelog

## 2.1.0 (2026-08-06)

### Added
- **`speckit.worktrees.create` is idempotent.** A new `## Outline` step 0 resolves which
  of three cases applies *before* deriving anything: already inside a linked worktree
  (no-op), primary checkout with a feature branch that already exists (use the branch
  name), or primary checkout on the base ref (the `before_specify` case, the only one
  that may pass `--from-description`). The hook and an explicit workflow step can now
  both run in the same pass without minting a spurious second feature number.

  This is what lets a workflow declare worktree isolation as a **step** instead of
  depending on the `before_specify` hook firing. A run that entered past
  `speckit.specify` — a resume, or a fix-up over an already-specified feature — never
  triggered the hook and executed entirely in the primary checkout.
- **A recovery path for the unrecoverable case.** When the feature branch is already
  checked out in the primary, no worktree can attach to it. Step 0 now recovers when the
  primary is *provably* clean — `git status --porcelain` and `git stash list` both empty
  — by moving the primary to the base ref and attaching the worktree, and reports
  `worktree_isolation=recovered` naming the base ref the primary now sits on. A dirty
  tree or any stash entry falls back to `worktree_isolation=failed`: run in place, report
  loudly, never force and never move the user's work.
- **A machine-readable isolation outcome** in the report — `created`, `already`,
  `entered`, `recovered`, or `failed` — so a workflow's ship step can put a non-clean
  outcome in the pull request description rather than let it die in a transcript.

### Changed
- The **never modify the primary checkout** rule now carries its one narrow exception
  (the recovery path above) instead of contradicting it.
- A non-zero script exit naming the primary repo is no longer "report it and stop"; it
  routes to step 0's failure path.

## 2.0.0 (2026-08-05)

Fork of [dango85/spec-kit-worktree-parallel](https://github.com/dango85/spec-kit-worktree-parallel)
**v1.0.0**, now published from
[clintcparker/speckit-addons](https://github.com/clintcparker/speckit-addons).

Upstream **1.3.2 is not merged**. It was read and rejected for this baseline: it
carries only partial `base_ref` support in the script, none of the other patches
below, plus tests and a post-install step that have never run here. Rebasing
these patches onto 1.3.2 remains possible later. The version is 2.0.0 purely so
catalog update logic moves forward from 1.3.2 — it does not imply 1.3.2 is
contained.

### Changed
- **The `speckit.worktrees.create` hook is declared at `before_specify`
  (priority 20)** instead of upstream's `after_specify`. A stock install now
  wires the worktree-first flow correctly, and `--force` reinstalls no longer
  revert a hand edit.

### Added (carried from local patches, not in upstream 1.0.0)
- `--from-description` — derive the branch name from the feature description by
  delegating to the git extension's `create-new-feature-branch.sh --dry-run`,
  so the `before_specify` hook can create the branch before a spec exists.
- `enter_worktree` config key — move the agent session into the new worktree so
  the spec and every later phase is written there rather than in the primary
  checkout.
- `base_ref` config key — the ref new feature branches fork from, auto-detected
  as `origin/main` → `main` → `origin/master` → `master` → `HEAD` when empty.
  Under the worktree-first flow the branch is created by `git worktree add -b`,
  so this — not the primary's HEAD — decides what a feature forks from.
- Worktree-first flow throughout the `speckit.worktrees.create` command.

## 1.0.0 (2026-04-13)

### Added
- `speckit.worktrees.create` command — spawn isolated worktrees with configurable layout
- `speckit.worktrees.list` command — dashboard of all active worktrees with spec-artifact and task progress
- `speckit.worktrees.clean` command — safe cleanup of merged, orphaned, or stale worktrees
- `after_specify` hook — auto-creates worktree after feature specification (configurable)
- Two layout modes: **sibling** (`../<repo>--<branch>`) and **nested** (`.worktrees/<branch>/`)
- Bash script `create-worktree.sh` for deterministic worktree creation with JSON output
- Per-repo configuration via `worktree-config.yml`
- `SPECIFY_WORKTREE_PATH` environment variable for path overrides
- `--in-place` / `--no-worktree` opt-out for single-agent flows
