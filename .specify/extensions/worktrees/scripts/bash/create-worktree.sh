#!/usr/bin/env bash
# spec-kit-worktree-parallel: create-worktree.sh
# Deterministic worktree creation for parallel agents/features.
# Called by the speckit.worktrees.create command or after_specify hook.
#
# Usage:
#   create-worktree.sh [options] <branch-name>
#   create-worktree.sh [options] --from-description "<feature description>"
#
# Options:
#   --layout sibling|nested   Override config layout (default: sibling)
#   --path <dir>              Explicit worktree path (overrides layout)
#   --in-place                Skip worktree creation; no-op exit 0
#   --json                    Output JSON instead of key=value
#   --dry-run                 Compute paths without creating anything
#   --base-ref <ref>          Base ref for new branch (default: auto-detect)
#   --from-description <text> Derive the branch name from a feature description
#   --repo-root <dir>         Repository root (default: git rev-parse --show-toplevel)
#   --config <file>           Path to worktree-config.yml (default: auto-detect)
#   --help                    Show this help
#
# Worktree-first invariant: this script is the *only* thing that creates a feature
# branch. The git extension's before_specify hook is disabled precisely so the
# branch is never checked out in the primary repo — a branch can live in exactly one
# worktree, so a `git checkout -b` in the primary makes `git worktree add` impossible
# for that same branch forever after.

set -euo pipefail

# --- defaults ---
LAYOUT="sibling"
WORKTREE_PATH_OVERRIDE=""
IN_PLACE=false
JSON_MODE=false
DRY_RUN=false
BASE_REF=""
REPO_ROOT=""
CONFIG_FILE=""
BRANCH_NAME=""
FEATURE_DESCRIPTION=""

# --- parse args ---
while [[ $# -gt 0 ]]; do
  case "$1" in
    --layout)      LAYOUT="$2"; shift 2 ;;
    --path)        WORKTREE_PATH_OVERRIDE="$2"; shift 2 ;;
    --in-place)    IN_PLACE=true; shift ;;
    --json)        JSON_MODE=true; shift ;;
    --dry-run)     DRY_RUN=true; shift ;;
    --base-ref)    BASE_REF="$2"; shift 2 ;;
    --from-description) FEATURE_DESCRIPTION="$2"; shift 2 ;;
    --repo-root)   REPO_ROOT="$2"; shift 2 ;;
    --config)      CONFIG_FILE="$2"; shift 2 ;;
    --help|-h)
      echo "Usage: $0 [options] <branch-name>"
      echo "       $0 [options] --from-description \"<feature description>\""
      echo ""
      echo "Options:"
      echo "  --layout sibling|nested   Worktree location strategy (default: sibling)"
      echo "  --path <dir>              Explicit worktree path (overrides layout)"
      echo "  --in-place                Skip worktree creation (no-op exit 0)"
      echo "  --json                    Output JSON instead of key=value"
      echo "  --dry-run                 Compute paths without creating anything"
      echo "  --base-ref <ref>          Base ref for new branch (default: auto-detect)"
      echo "  --from-description <text> Derive branch name from a feature description"
      echo "  --repo-root <dir>         Repository root (default: git rev-parse)"
      echo "  --config <file>           Path to worktree-config.yml"
      echo "  --help                    Show this help"
      exit 0
      ;;
    -*)
      echo "Error: unknown option $1" >&2; exit 1 ;;
    *)
      if [[ -z "$BRANCH_NAME" ]]; then
        BRANCH_NAME="$1"
      else
        echo "Error: unexpected argument '$1' (branch already set to '$BRANCH_NAME')" >&2; exit 1
      fi
      shift ;;
  esac
done

if [[ -z "$BRANCH_NAME" && -z "$FEATURE_DESCRIPTION" ]]; then
  echo "Error: branch name is required (or pass --from-description \"<text>\")" >&2
  echo "Usage: $0 [options] <branch-name>" >&2
  exit 1
fi

if [[ -n "$BRANCH_NAME" && -n "$FEATURE_DESCRIPTION" ]]; then
  echo "Error: pass either a branch name or --from-description, not both" >&2
  exit 1
fi

# --- resolve repo root ---
if [[ -z "$REPO_ROOT" ]]; then
  REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" || {
    echo "Error: not inside a git repository" >&2; exit 1
  }
fi

# When invoked from a worktree, $REPO_ROOT is that worktree, not the repo the
# worktrees hang off. Every branch/worktree query below must run against the main
# worktree, otherwise a sibling path would be computed relative to a sibling
# (homeapp1--008-x--008-x) and `git worktree list` would be read from the wrong root.
MAIN_ROOT="$(git -C "$REPO_ROOT" rev-parse --path-format=absolute --git-common-dir 2>/dev/null)" || MAIN_ROOT=""
if [[ -n "$MAIN_ROOT" ]]; then
  MAIN_ROOT="$(dirname -- "$MAIN_ROOT")"
else
  MAIN_ROOT="$REPO_ROOT"
fi
# A bare/unusual layout can leave MAIN_ROOT pointing somewhere without a worktree;
# fall back rather than guess.
[[ -d "$MAIN_ROOT" ]] || MAIN_ROOT="$REPO_ROOT"
REPO_ROOT="$MAIN_ROOT"

# --- derive branch name from a feature description (before_specify hook path) ---
# Delegates to the git extension's numbering logic in --dry-run mode: it computes
# the next sequential number from specs/, local branches and `git ls-remote` without
# creating or checking out anything. Duplicating that numbering here would drift.
if [[ -z "$BRANCH_NAME" ]]; then
  feature_script="$REPO_ROOT/.specify/extensions/git/scripts/bash/create-new-feature-branch.sh"
  if [[ ! -f "$feature_script" ]]; then
    echo "Error: --from-description requires the git extension at $feature_script" >&2
    exit 1
  fi
  if ! branch_json=$(bash "$feature_script" --dry-run --json "$FEATURE_DESCRIPTION" 2>&1); then
    echo "Error: could not derive a branch name from the feature description." >&2
    printf '%s\n' "$branch_json" >&2
    exit 1
  fi
  if command -v jq >/dev/null 2>&1; then
    BRANCH_NAME=$(printf '%s' "$branch_json" | jq -r '.BRANCH_NAME // empty')
  else
    BRANCH_NAME=$(printf '%s' "$branch_json" \
      | sed -n 's/.*"BRANCH_NAME"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)
  fi
  if [[ -z "$BRANCH_NAME" ]]; then
    echo "Error: no BRANCH_NAME in the branch script output:" >&2
    printf '%s\n' "$branch_json" >&2
    exit 1
  fi
fi

# --- load config ---
load_config_value() {
  local key="$1" default="$2" file="$CONFIG_FILE"
  if [[ -z "$file" ]]; then
    # auto-detect: extension config in .specify
    for candidate in \
      "$REPO_ROOT/.specify/extensions/worktrees/worktree-config.yml" \
      "$REPO_ROOT/.specify/extensions/worktrees/config.yml"; do
      if [[ -f "$candidate" ]]; then file="$candidate"; break; fi
    done
  fi
  if [[ -n "$file" ]] && [[ -f "$file" ]]; then
    local val
    val=$(grep -E "^${key}:" "$file" 2>/dev/null | head -1 | sed 's/^[^:]*: *//; s/ *#.*//; s/^"//; s/"$//' || true)
    if [[ -n "$val" ]]; then echo "$val"; return; fi
  fi
  echo "$default"
}

if [[ "$IN_PLACE" == true ]]; then
  # --in-place: no worktree, just report and exit
  if $JSON_MODE; then
    printf '{"branch":"%s","worktree":false,"path":""}\n' "$BRANCH_NAME"
  else
    echo "WORKTREE=false"
    echo "BRANCH=$BRANCH_NAME"
  fi
  exit 0
fi

# Override layout from env or config
if [[ -z "$WORKTREE_PATH_OVERRIDE" ]]; then
  LAYOUT=$(load_config_value "layout" "$LAYOUT")
fi
AUTO_CREATE=$(load_config_value "auto_create" "true")
SIBLING_PATTERN=$(load_config_value "sibling_pattern" '{{repo}}--{{branch}}')
DOTWORKTREES_DIR=$(load_config_value "dotworktrees_dir" ".worktrees")

# env override
if [[ -n "${SPECIFY_WORKTREE_PATH:-}" ]]; then
  WORKTREE_PATH_OVERRIDE="$SPECIFY_WORKTREE_PATH"
fi

# --- guard: is this branch already checked out somewhere? ---
# A branch lives in exactly one worktree. Answer that question *before* computing a
# path, so re-running the command (or running it from inside the worktree it already
# made) reports the existing checkout instead of failing deep inside `git worktree add`.
# `git worktree list --porcelain` emits stanzas: worktree <path> / HEAD <sha> /
# branch <ref>, blank-line separated. The first stanza is always the main worktree.
worktree_holding_branch() {
  git -C "$REPO_ROOT" worktree list --porcelain | awk -v ref="refs/heads/$BRANCH_NAME" '
    /^worktree /  { path = substr($0, 10) }
    /^branch /    { if (substr($0, 8) == ref) { print path; exit } }
  '
}

EXISTING_WT="$(worktree_holding_branch)"
if [[ -n "$EXISTING_WT" ]]; then
  if [[ "$EXISTING_WT" == "$REPO_ROOT" ]]; then
    # The primary checkout owns the branch, so no worktree can ever be attached to it.
    # This is what a `git checkout -b` in the primary (the old before_specify git hook)
    # leaves behind; say so plainly instead of letting `git worktree add` fail opaquely.
    echo "Error: branch '$BRANCH_NAME' is checked out in the primary repo: $EXISTING_WT" >&2
    echo "A branch can only be checked out in one worktree, so no worktree can be" >&2
    echo "attached to it while the primary holds it. Move the primary to another branch:" >&2
    echo "  git -C \"$EXISTING_WT\" switch <other-branch>" >&2
    echo "then re-run this command." >&2
    exit 1
  fi
  # Already has its own worktree — idempotent success, per the one-worktree-per-branch rule.
  echo "[worktrees] Reusing existing worktree: $EXISTING_WT (branch $BRANCH_NAME)" >&2
  if $JSON_MODE; then
    printf '{"branch":"%s","worktree":true,"path":"%s","layout":"%s","reused":true}\n' \
      "$BRANCH_NAME" "$EXISTING_WT" "$LAYOUT"
  else
    echo "WORKTREE=true"
    echo "BRANCH=$BRANCH_NAME"
    echo "PATH=$EXISTING_WT"
    echo "LAYOUT=$LAYOUT"
    echo "REUSED=true"
  fi
  exit 0
fi

# --- resolve worktree target path ---
resolve_worktree_path() {
  if [[ -n "$WORKTREE_PATH_OVERRIDE" ]]; then
    if [[ "$WORKTREE_PATH_OVERRIDE" = /* ]]; then
      echo "$WORKTREE_PATH_OVERRIDE"
    else
      local _d _f
      _d=$(dirname "$WORKTREE_PATH_OVERRIDE")
      _f=$(basename "$WORKTREE_PATH_OVERRIDE")
      echo "$(cd "$REPO_ROOT" && cd "$_d" 2>/dev/null && pwd)/$_f"
    fi
    return
  fi

  local safe_branch
  safe_branch="$(echo "$BRANCH_NAME" | tr '/ ' '--')"

  case "$LAYOUT" in
    sibling)
      local parent base
      parent="$(dirname -- "$REPO_ROOT")"
      base="$(basename -- "$REPO_ROOT")"
      local name="$SIBLING_PATTERN"
      name="${name//\{\{repo\}\}/$base}"
      name="${name//\{\{branch\}\}/$safe_branch}"
      echo "${parent}/${name}"
      ;;
    nested)
      echo "${REPO_ROOT}/${DOTWORKTREES_DIR}/${safe_branch}"
      ;;
    *)
      echo "Error: unknown layout '$LAYOUT' (expected: sibling, nested)" >&2
      exit 1
      ;;
  esac
}

WT_TARGET=$(resolve_worktree_path)

# --- resolve base ref ---
resolve_base_ref() {
  if [[ -n "$BASE_REF" ]]; then echo "$BASE_REF"; return; fi
  local configured
  configured=$(load_config_value "base_ref" "")
  if [[ -n "$configured" ]]; then echo "$configured"; return; fi
  if git -C "$REPO_ROOT" rev-parse --verify origin/main >/dev/null 2>&1; then echo "origin/main"
  elif git -C "$REPO_ROOT" rev-parse --verify main >/dev/null 2>&1; then echo "main"
  elif git -C "$REPO_ROOT" rev-parse --verify origin/master >/dev/null 2>&1; then echo "origin/master"
  elif git -C "$REPO_ROOT" rev-parse --verify master >/dev/null 2>&1; then echo "master"
  else echo "HEAD"
  fi
}

# --- dry-run ---
if [[ "$DRY_RUN" == true ]]; then
  # Report the base ref too: under the worktree-first flow it — not the primary's
  # current HEAD — decides what the feature forks from, so it is worth seeing up front.
  DRY_BASE=$(resolve_base_ref)
  if $JSON_MODE; then
    printf '{"branch":"%s","worktree":true,"path":"%s","layout":"%s","base_ref":"%s","dry_run":true}\n' \
      "$BRANCH_NAME" "$WT_TARGET" "$LAYOUT" "$DRY_BASE"
  else
    echo "WORKTREE=true"
    echo "BRANCH=$BRANCH_NAME"
    echo "PATH=$WT_TARGET"
    echo "LAYOUT=$LAYOUT"
    echo "BASE_REF=$DRY_BASE"
    echo "DRY_RUN=true"
  fi
  exit 0
fi

# --- guard: target must not exist ---
if [[ -e "$WT_TARGET" ]]; then
  echo "Error: worktree path already exists: $WT_TARGET" >&2
  echo "Remove it, set SPECIFY_WORKTREE_PATH, or pass --path to another directory." >&2
  exit 1
fi

# --- ensure .worktrees/ is gitignored for nested layout ---
if [[ "$LAYOUT" == "nested" ]]; then
  local_gitignore="$REPO_ROOT/.gitignore"
  if ! grep -qxF "$DOTWORKTREES_DIR/" "$local_gitignore" 2>/dev/null; then
    echo "$DOTWORKTREES_DIR/" >> "$local_gitignore"
  fi
fi

# --- create worktree ---
RESOLVED_BASE=$(resolve_base_ref)

# git's own stderr is the only useful diagnostic here — never discard it.
if git -C "$REPO_ROOT" show-ref --verify --quiet "refs/heads/$BRANCH_NAME"; then
  # Branch exists locally but is checked out nowhere (the guard above proved that) —
  # attach a worktree to it.
  if ! git_err=$(git -C "$REPO_ROOT" worktree add "$WT_TARGET" "$BRANCH_NAME" 2>&1); then
    echo "Error: git worktree add failed for existing branch '$BRANCH_NAME' at '$WT_TARGET'." >&2
    printf '%s\n' "$git_err" >&2
    exit 1
  fi
else
  # Create new branch + worktree from base ref. This is the normal path under the
  # worktree-first flow: the branch is born in the worktree and never touches the primary.
  if ! git_err=$(git -C "$REPO_ROOT" worktree add -b "$BRANCH_NAME" "$WT_TARGET" "$RESOLVED_BASE" 2>&1); then
    echo "Error: git worktree add -b '$BRANCH_NAME' at '$WT_TARGET' from '$RESOLVED_BASE' failed." >&2
    printf '%s\n' "$git_err" >&2
    echo "Run 'git fetch' or use --in-place if worktrees are not available." >&2
    exit 1
  fi
fi

echo "[worktrees] Created: $WT_TARGET (branch $BRANCH_NAME)" >&2

# --- output ---
if $JSON_MODE; then
  printf '{"branch":"%s","worktree":true,"path":"%s","layout":"%s","base_ref":"%s"}\n' \
    "$BRANCH_NAME" "$WT_TARGET" "$LAYOUT" "$RESOLVED_BASE"
else
  echo "WORKTREE=true"
  echo "BRANCH=$BRANCH_NAME"
  echo "PATH=$WT_TARGET"
  echo "LAYOUT=$LAYOUT"
  echo "BASE_REF=$RESOLVED_BASE"
fi
