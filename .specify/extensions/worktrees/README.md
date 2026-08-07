# spec-kit-worktree-parallel

> **This is a fork.** Published from
> [clintcparker/speckit-addons](https://github.com/clintcparker/speckit-addons)
> as `worktrees` v2.0.0 — dango85 v1.0.0 plus `--from-description`,
> `enter_worktree`, `base_ref`, the worktree-first flow, and a `before_specify`
> hook declaration. Upstream v1.3.2 is not merged; see
> [CHANGELOG.md](CHANGELOG.md). Original MIT license retained in
> [LICENSE](LICENSE).

A [Spec Kit](https://github.com/github/spec-kit) extension for **default-on** git worktree isolation — work on multiple features (or run parallel agents) without checkout switching.

## Why another worktree extension?

The community [spec-kit-worktree](https://github.com/Quratulain-bilal/spec-kit-worktree) extension is a good starting point. This extension differs in three ways:

1. **Default-on** — worktrees are created automatically after `/speckit.specify`. Opt *out* with `--in-place`, rather than opting in.
2. **Sibling-dir layout** — worktrees live at `../<repo>--<branch>` by default, so each feature gets its own top-level IDE window. Nested `.worktrees/` is available as an option.
3. **Deterministic bash script** — a real script (`create-worktree.sh`) with `--json` output, `--dry-run`, and `SPECIFY_WORKTREE_PATH` override, suitable for CI and scripted workflows.

## Installation

```bash
specify extension catalog add \
  https://raw.githubusercontent.com/clintcparker/speckit-addons/main/extensions/catalog.json \
  --name speckit-addons --install-allowed --priority 5

specify extension add worktrees
```

## Layout modes

### Sibling (default)

Each worktree is a sibling directory of the primary clone:

```
parent/
├── my-project/                  ← primary checkout (main)
├── my-project--005-user-auth/   ← worktree (005-user-auth branch)
├── my-project--006-chat/        ← worktree (006-chat branch)
```

Open each directory in its own IDE window. No `.gitignore` changes needed.

### Nested

Worktrees live inside the repo under `.worktrees/` (auto-gitignored):

```
my-project/
├── .worktrees/
│   ├── 005-user-auth/           ← worktree
│   ├── 006-chat/                ← worktree
├── specs/
├── src/
```

Switch with `layout: nested` in `worktree-config.yml`.

## Configuration

Create `.specify/extensions/worktrees/worktree-config.yml` to override defaults:

```yaml
layout: "sibling"           # sibling | nested
auto_create: true            # false to prompt instead of auto-creating
sibling_pattern: "{{repo}}--{{branch}}"
dotworktrees_dir: ".worktrees"
base_ref: ""                 # ref new branches are cut from; "" = auto-detect
                              # (origin/main → main → origin/master → master → HEAD)
enter_worktree: true          # move the session into the new worktree; false to
                               # just print the path and stay put
```

## Commands

| Command | Description | Modifies files? |
|---------|-------------|-----------------|
| `/speckit.worktrees.create` | Spawn a worktree for a feature branch | Yes |
| `/speckit.worktrees.list` | Dashboard: status, artifacts, tasks | No |
| `/speckit.worktrees.clean` | Remove merged/stale worktrees | Yes |

## Hook

**`before_specify`** (priority 20) — creates the feature branch inside a new worktree and moves the session there *before* the spec is written. Controlled by the `auto_create` config value.

This runs before rather than after `/speckit.specify` because a branch can live in exactly one worktree: if the spec were written first, `/speckit.specify` would create (and check out) the branch in the primary checkout, and `git worktree add` cannot claim that branch for as long as the primary stays there. Worktree-first avoids this failure mode by cutting the branch straight into its worktree, then writing the spec — and every later phase — there from the start.

### The hook is not enough on its own

It fires only when a run actually *starts* at `/speckit.specify`. A resumed run, or a fix-up over a feature that was already specified, enters at a later phase, never triggers the hook, and executes entirely in the primary checkout — silently. Any workflow that must not depend on where a run entered should declare `speckit.worktrees.create` as an **explicit first step** as well. The command is idempotent as of 2.1.0 (`## Outline` step 0), so whichever of the two lands second finds the session already isolated and no-ops; there is no double-creation and no spurious second feature number.

`speckit-addons`' own [`send-it`](../../workflows/send-it/), [`send-it-checked`](../../workflows/send-it-checked/) and [`yolo`](../../workflows/yolo/) workflows all do this.

### When the branch is already checked out in the primary

`git worktree add` cannot succeed until the primary moves off it. Step 0 recovers *only* when the primary is provably clean — `git status --porcelain` and `git stash list` both empty — by checking the primary out to the base ref, attaching the worktree, and reporting `worktree_isolation=recovered` along with the base ref the primary now sits on. A dirty tree or any stash entry falls back to `worktree_isolation=failed`: the run continues in place and reports loudly, and nothing of yours is moved, stashed, or forced.

## Script usage

The bash script can be called directly for automation:

```bash
# Create a sibling worktree for branch 005-user-auth
bash scripts/bash/create-worktree.sh --json 005-user-auth

# Nested layout
bash scripts/bash/create-worktree.sh --json --layout nested 005-user-auth

# Explicit path
bash scripts/bash/create-worktree.sh --json --path /tmp/my-worktree 005-user-auth

# Dry run (compute path without creating)
bash scripts/bash/create-worktree.sh --json --dry-run 005-user-auth

# Skip worktree (single-agent mode)
bash scripts/bash/create-worktree.sh --in-place 005-user-auth
```

## Environment variables

| Variable | Description |
|----------|-------------|
| `SPECIFY_WORKTREE_PATH` | Override computed worktree path entirely |
| `SPECIFY_FEATURE` | Current feature name (set by spec-kit) |

## Related

- [#61](https://github.com/github/spec-kit/issues/61) — Spawn worktree when creating new branch (36+ upvotes)
- [#1476](https://github.com/github/spec-kit/issues/1476) — Native worktree support for parallel agents
- [#1940](https://github.com/github/spec-kit/issues/1940) — Git operations extracted to extension (closed)

## Requirements

- Spec Kit >= 0.4.0
- Git >= 2.15.0 (worktree support)

## License

MIT
