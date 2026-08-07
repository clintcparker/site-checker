---
description: "Spawn an isolated git worktree for a feature branch (default-on, configurable layout)"
---

# Create Worktree

Spawn an isolated git worktree for a feature branch so you can work on multiple features — or run multiple agents — in parallel without switching branches.

This command **owns feature-branch creation** in this project. A branch can be checked out in exactly one worktree, so if anything checks the branch out in the primary repo first (the git extension's `speckit.git.feature` hook used to), no worktree can be attached to it for as long as the primary stays there. That hook is therefore disabled in `.specify/extensions.yml`, and this command creates branch and worktree together with `git worktree add -b`. The primary checkout is not moved — except by the one recovery path in `## Outline` step 0, which needs a provably clean primary and reports the move.

The command is **idempotent**. Workflows that must not depend on `speckit.specify` running (a resumed run, or a fix-up over an already-specified feature) declare it as an explicit step *and* inherit the `before_specify` hook; step 0 makes the second of those a no-op.

## User Input

```text
$ARGUMENTS
```

You **MUST** consider the user input before proceeding (if not empty). The user may specify:

- A branch name (e.g., `005-user-auth`)
- `current` to create a worktree for the current branch
- `--in-place` or `--no-worktree` to skip worktree creation entirely

When this command runs as the **`before_specify` hook**, `$ARGUMENTS` is the *feature description* the user typed after `/speckit-specify`, not a branch name. In that mode the branch usually does not exist yet — pass the description through with `--from-description` and let the script derive the name. The same is true of an explicit `worktree` workflow step that forwards the workflow's spec input. Neither is a licence to assume the branch is absent: `## Outline` step 0 checks first, and `--from-description` is correct only in its case 3.

## Prerequisites

1. Verify the project is a git repository (`git rev-parse --show-toplevel`)
2. Verify `git worktree` is available (`git worktree list` succeeds)

## Configuration

Read configuration from `.specify/extensions/worktrees/worktree-config.yml` if it exists. Defaults apply when the file is absent.

| Key | Default | Description |
|-----|---------|-------------|
| `layout` | `sibling` | `sibling` — worktree at `../<repo>--<branch>` (IDE-friendly); `nested` — at `.worktrees/<branch>/` inside repo |
| `auto_create` | `true` | When `true`, the hook creates a worktree without prompting |
| `sibling_pattern` | `{{repo}}--{{branch}}` | Name pattern for sibling directories |
| `dotworktrees_dir` | `.worktrees` | Subdirectory name for nested layout |
| `base_ref` | *(empty)* | Ref new branches fork from. Empty = auto-detect `origin/main` → `main` → `origin/master` → `master` → `HEAD` |
| `enter_worktree` | `true` | Move the agent session into the worktree after creating it |

Environment variable `SPECIFY_WORKTREE_PATH` overrides the computed path entirely.

## Outline

0. **Detect the case before doing anything.** This command is idempotent and may be invoked both as
   the `before_specify` hook and as an explicit workflow step in the same run. Resolve which of three
   situations applies, and never fall through to `--from-description` unless it is case 3:

   - **Case 1 — already isolated.** `git rev-parse --git-common-dir` differs from
     `git rev-parse --git-dir`, i.e. the session is inside a linked worktree. **No-op.** Report
     `worktree_isolation=already` with the path and exit 0. Do not create, do not enter, do not
     derive a branch name.
   - **Case 2 — primary checkout, feature branch already exists.** `git branch --show-current`
     returns a name that is not the base ref. Use that **branch name**; never `--from-description`,
     which would mint a spurious next feature number. If `git worktree list --porcelain` already
     shows a worktree for it, enter that worktree and report `worktree_isolation=entered`. If none
     exists, the branch is checked out here and `git worktree add` for it cannot succeed — see the
     failure path below.

     A detached HEAD (`git branch --show-current` empty) is *not* case 3. Resolve the branch through
     step 1's fallbacks — `SPECIFY_FEATURE`, else the most recent feature branch — and only treat it
     as case 3 when neither names one.
   - **Case 3 — primary checkout, on the base ref, no feature branch yet.** This is the
     `before_specify` case. Pass `--from-description "<description>"` and let the script derive the
     name.

   **Failure path (case 2 with no worktree).** The branch is checked out in the primary, so no
   worktree can attach to it until the primary moves off it. Attempt recovery, but only when it is
   provably safe:

   - **Recover** when `git status --porcelain` **and** `git stash list` are *both* empty in the
     primary. Nothing can be lost, so: resolve the base ref to a *local* branch where one exists
     (`origin/main` → `main`) rather than leaving the user on a detached HEAD, run
     `git checkout <base>` in the primary, then `git worktree add <path> <branch>` and enter it.
     Report `worktree_isolation=recovered` with the branch, the worktree path, and — prominently —
     that **the primary checkout was moved off the feature branch to `<base>`**. That side effect is
     the price of recovery and must never be silent.
   - **Otherwise fall back.** A dirty tree or any stash entry means the primary is holding work that
     is not this run's to move. Do not force it, do not delete the branch, do not stash, do not move
     the primary. Report `worktree_isolation=failed`, the branch name, and the reason ("the branch is
     checked out in the primary with uncommitted work; a branch can live in exactly one worktree").
     Continue in place so an unattended run still completes, and make the report available to later
     steps.

1. **Determine the target branch** — step 0 has already settled which case applies; this is how each
   one names its branch:
   - Case 3, the `before_specify` hook call → do **not** invent a branch name. Pass the feature description to the script as `--from-description "<description>"`; it delegates to the git extension's `create-new-feature-branch.sh --dry-run`, which computes the next sequential number from `specs/`, local branches and remote refs *without* creating or checking out anything.
   - Case 2, a feature branch already exists → use that branch name, never `--from-description`.
   - User supplied a branch name → use it.
   - User said `current` → use `git branch --show-current`.
   - Otherwise (manual call, no argument) → use `SPECIFY_FEATURE` if set, else the most recent feature branch.

2. **Invoke the script**:
   Run the deterministic bash script shipped with this extension:

   ```bash
   bash .specify/extensions/worktrees/scripts/bash/create-worktree.sh \
     --json \
     [--from-description "<feature description>" | "$BRANCH_NAME"] \
     [--layout sibling|nested] \
     [--path <override>] \
     [--in-place] \
     [--dry-run] \
     [--base-ref <ref>]
   ```

   The script reads `worktree-config.yml` automatically and outputs JSON:

   ```json
   {"branch":"005-user-auth","worktree":true,"path":"/Users/me/code/MyProject--005-user-auth","layout":"sibling"}
   ```

   `"reused": true` means the branch already had a worktree and nothing was created — that is success, not an error. A non-zero exit with a message naming the *primary* repo means the branch is checked out there: hand it to step 0's failure path — recover if the primary is provably clean, otherwise report and continue in place. Never force it and never delete the branch.

   If the script is unavailable (e.g., non-bash environment), perform the equivalent operations directly:
   - Resolve the worktree path based on layout config
   - Check `git worktree list --porcelain` first — if the branch already appears under a worktree, report that path and stop
   - Run `git worktree add -b <branch> <path> <base-ref>` (new branch) or `git worktree add <path> <branch>` (existing branch checked out nowhere)
   - For nested layout, ensure `.worktrees/` is in `.gitignore`

3. **Move into the worktree** (unless `enter_worktree` is `false`, or the user passed `--in-place`/`--no-worktree`):

   Call the `EnterWorktree` tool with `path` set to the `path` from the script's JSON. The session's working directory becomes the worktree, which is what makes the rest of the feature land there: `specs/<dir>/spec.md`, `.specify/feature.json`, and every later phase (`/speckit-plan`, `/speckit-tasks`, `/speckit-implement`, `/speckit-ship-run`) resolve relative to the worktree with no absolute-path juggling. Continue the current command — including the `/speckit-specify` body that invoked this hook — from there.

   If no such tool exists (non-Claude integration), do not silently write the spec into the primary checkout. Instead:
   - export `SPECIFY_INIT_DIR=<worktree path>` for every `.specify/scripts/**` invocation, and
   - set `SPECIFY_FEATURE_DIRECTORY=<worktree path>/specs/<feature-dir>` before creating the spec.

   Both are first-priority overrides honored by `.specify/scripts/bash/common.sh`.

4. **Verify spec artifacts** — only when the worktree already existed, or on a manual call. Under `before_specify` the worktree is new and intentionally has no spec yet; skip this step. Otherwise check `specs/<feature-dir>/` in the worktree and list which artifacts are present (spec.md, plan.md, tasks.md).

5. **Report**: Output a summary:

   ```markdown
   ## Worktree Created

   | Field | Value |
   |-------|-------|
   | **Branch** | 005-user-auth |
   | **Base ref** | origin/main |
   | **Layout** | sibling |
   | **Worktree path** | /Users/me/code/MyProject--005-user-auth |
   | **Session** | moved into the worktree |
   | **Isolation** | created |

   **Next steps:**
   - Continue the current phase — you are already in the worktree
   - Run `/speckit-worktrees-list` to see all active worktrees
   ```

   **Isolation** carries step 0's outcome verbatim, and later steps of an unattended workflow read
   it: `created` (case 3), `already` (case 1 no-op), `entered` (case 2, worktree existed),
   `recovered` (the primary was moved off the branch — name the base ref it now sits on), or
   `failed` (running in the primary; name the branch and the reason). Anything other than the first
   three is a fact the run must carry to its end — a ship step should put it in the pull request
   description rather than let it die in a transcript.

## Rules

- **Idempotent** — safe to invoke twice in one run (the `before_specify` hook *and* an explicit workflow step). Step 0 decides the case before anything else happens; whichever call lands second finds the session already isolated and no-ops
- **Default behavior is to create a worktree** — only skip if the user explicitly passes `--in-place` or `--no-worktree`, or `auto_create` is `false` in config and this is a hook call
- **One worktree per branch** — never create a duplicate; report the existing path and exit successfully
- **Never modify the primary checkout, with one narrow exception** — no `checkout`, no `switch`, no stash in normal operation. The primary stays where the user left it, which is the whole point of creating the branch inside the worktree. The single exception is step 0's recovery path, where the feature branch is *already* checked out in the primary and the primary is provably clean (empty `git status --porcelain`, empty `git stash list`). There, and only there, `git checkout <base>` is permitted so the branch can be released to a worktree — and the move must be reported loudly
- **Never create the branch outside the worktree** — `git worktree add -b` does both atomically; a preceding `git checkout -b` makes the worktree impossible
- **Always update .gitignore for nested layout** — add the `dotworktrees_dir` value if not present
- **Branch names come from the git extension** — call it with `--dry-run` rather than reimplementing the numbering, so branch names stay consistent with `speckit.git.feature`
- **`--from-description` only in the `before_specify` hook** — it answers "what is the *next* feature number", so calling it a second time for a feature that already has a branch mints a spurious new one. Once the branch exists, always pass the branch name
