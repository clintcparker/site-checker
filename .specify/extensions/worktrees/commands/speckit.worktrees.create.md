---
description: "Spawn an isolated git worktree for a feature branch (default-on, configurable layout)"
---

# Create Worktree

Spawn an isolated git worktree for a feature branch so you can work on multiple features — or run multiple agents — in parallel without switching branches.

This command **owns feature-branch creation** in this project. A branch can be checked out in exactly one worktree, so if anything checks the branch out in the primary repo first (the git extension's `speckit.git.feature` hook used to), no worktree can ever be attached to it. That hook is therefore disabled in `.specify/extensions.yml`, and this command creates branch and worktree together with `git worktree add -b`. The primary checkout is never moved.

## User Input

```text
$ARGUMENTS
```

You **MUST** consider the user input before proceeding (if not empty). The user may specify:

- A branch name (e.g., `005-user-auth`)
- `current` to create a worktree for the current branch
- `--in-place` or `--no-worktree` to skip worktree creation entirely

When this command runs as the **`before_specify` hook**, `$ARGUMENTS` is the *feature description* the user typed after `/speckit-specify`, not a branch name. In that mode the branch does not exist yet — pass the description through with `--from-description` and let the script derive the name.

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

1. **Determine the target branch**:
   - `before_specify` hook call → do **not** invent a branch name. Pass the feature description to the script as `--from-description "<description>"`; it delegates to the git extension's `create-new-feature-branch.sh --dry-run`, which computes the next sequential number from `specs/`, local branches and remote refs *without* creating or checking out anything.
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

   `"reused": true` means the branch already had a worktree and nothing was created — that is success, not an error. A non-zero exit with a message naming the *primary* repo means the branch is checked out there; report it and stop, do not try to work around it.

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

   **Next steps:**
   - Continue the current phase — you are already in the worktree
   - Run `/speckit-worktrees-list` to see all active worktrees
   ```

## Rules

- **Default behavior is to create a worktree** — only skip if the user explicitly passes `--in-place` or `--no-worktree`, or `auto_create` is `false` in config and this is a hook call
- **One worktree per branch** — never create a duplicate; report the existing path and exit successfully
- **Never modify the primary checkout** — no `checkout`, no `switch`, no stash. The primary stays where the user left it, which is the whole point of creating the branch inside the worktree
- **Never create the branch outside the worktree** — `git worktree add -b` does both atomically; a preceding `git checkout -b` makes the worktree impossible
- **Always update .gitignore for nested layout** — add the `dotworktrees_dir` value if not present
- **Branch names come from the git extension** — call it with `--dry-run` rather than reimplementing the numbering, so branch names stay consistent with `speckit.git.feature`
- **`--from-description` only in the `before_specify` hook** — it answers "what is the *next* feature number", so calling it a second time for a feature that already has a branch mints a spurious new one. Once the branch exists, always pass the branch name
