# Changelog

## 1.1.0 (2026-08-05)

Fork of the `git` extension bundled with
[github/spec-kit](https://github.com/github/spec-kit) at v1.0.0, published from
[clintcparker/speckit-addons](https://github.com/clintcparker/speckit-addons).

### Changed
- `create-new-feature-branch.{sh,ps1,py}` now honor `branch_numbering: timestamp`
  from `git-config.yml` when the caller passes no explicit flag. The worktrees
  hook calls the script without `--timestamp`, so the config value was
  previously dead. An explicit `--number N` still forces sequential numbering.
- `config-template.yml` ships `branch_numbering: timestamp` by default. Parallel
  worktrees each compute "the next sequential number" independently from the same
  `specs/` directory and refs, so two features specified at once collide.
- The `before_specify` → `speckit.git.feature` hook is **no longer declared**.
  The `worktrees` extension creates the feature branch inside the worktree; with
  both declared the branch is created twice and the second attempt lands in the
  primary checkout. Previously this had to be disabled by hand in
  `.specify/extensions.yml` after every reinstall.

### Unchanged
- All other hooks (auto-commit before/after each phase, `before_constitution`
  repository initialization) keep upstream behavior.
- All five commands, and the `speckit.git.feature` command itself — it is still
  installed and still callable directly, just not hooked.
