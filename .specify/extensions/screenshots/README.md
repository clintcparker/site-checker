# Screenshots

Before/after UI screenshots for a Spec Kit feature, committed to the feature
branch so the ship step can embed them in the pull request.

It is a cheap end-to-end smoke test that doubles as PR documentation: if the app
starts and the pages render, the change is at least alive.

## The seam

`commands/capture.md` is **generic and never edited per repo**. It owns the
before/after mode contract, the UI-relevance decision, the `SKIPPED.md`
self-skip and its after-mode verification, the manifest, filename conventions,
commit rules, and the two constraints that matter most — never modify app code,
and real user data must survive every run including a failed one.

`screenshots-config.yml` owns everything that differs between apps: `ui_surface`,
`launch`, `auth`, `data`, `targets`, `viewports`, `capture_method`, `cleanup`.

Because the profile is a Spec Kit **config file**, it survives
`specify extension add screenshots --force`. Upgrading the extension never
clobbers a repo's adaptation, and because the command body never changes per
repo, the `.claude/skills` copy of it never goes stale either.

> The config file has to be named `screenshots-config.yml`. Spec Kit only
> preserves top-level `*-config.yml` / `*-config.local.yml` files across a
> `--force` reinstall; a differently-named target is dropped on install and
> destroyed on reinstall.

## Bootstrap

A fresh install ships the profile with `unconfigured: true`. On first run the
command derives the profile itself — reading the README, build manifests, entry
points and any existing e2e config — writes it back, flips the flag, and notes
in the manifest that the profile was auto-generated. A repo-agnostic install is
runnable with zero manual steps; review the profile afterwards.

Two filled-in profiles are in [`examples/`](examples/): an ASP.NET Razor Pages
app captured with Playwright, and a Tauri desktop app captured with AppleScript
and `screencapture`.

## Which feature it screenshots

Step 1 resolves `FEATURE_DIR` from `$SPECIFY_FEATURE_DIRECTORY`, then
`.specify/run-context.json`, then
`check-prerequisites.sh --paths-only --json` — pinned run state first, the
checkout last.

`--paths-only` matters. The plain `check-prerequisites.sh --json` form validates
before it reports and one of its gates is `plan.md`, so it exits 1 on any feature
that never ran `/speckit-plan`; a screenshot pass needs a feature *directory*, not
a plan. A feature with no `spec.md` either is a supported input — with nothing to
read, the command captures the baseline rather than skipping, because an
unnecessary baseline costs two PNGs and a missing one cannot be recreated after
the implementation lands.

The script's answer is also cross-checked against the pinned one. Unpinned, it
resolves from `.specify/feature.json`, which in a primary checkout right after a
merge names the feature that just shipped — and it exits 0 on it. A disagreement
stops the command, naming both paths, rather than screenshotting whichever tree
the session happens to be standing in.

[docs/core-helper-scripts.md](../../docs/core-helper-scripts.md) has the
upstream-side patch for the same two holes.

## Tracked, not force-added

The pull request body links these images by path, so they have to be *tracked* —
an image git considers ignored is never in the pushed head and its URL 404s for
every reviewer. Plenty of repos ignore `screenshots/` or `specs/*/screenshots/`
for unrelated reasons, and the workaround people reach for, `git add -f`, fixes
one commit and leaves the next step with the same conflict.

Step 2 of the command resolves it once instead: if `git check-ignore` reports a
rule against `FEATURE_DIR/screenshots/`, it appends `!screenshots/` to
`FEATURE_DIR/.gitignore` and commits that file. A `.gitignore` inside the feature
directory outranks the repo root's, so plain `git add` works from then on — for
the after pass, for `speckit.ship.run`, and in the reviewer's checkout. The
repo's own ignore rule is left exactly as it was.

## Manifest

`FEATURE_DIR/screenshots/manifest.json`:

| Key | Meaning |
|---|---|
| `targets` | Captured units, each `{ "slug", "why" }` |
| `viewports` | Label → `WxH`, copied from the profile |
| `baseline` | `"available"` or `"unavailable"` |
| `notes` | Free-form; failures are recorded here, not dropped |
| `app` | Free-form, profile-specific state `after` needs to reproduce `before` |

## Hooks

None, deliberately. Workflows that want screenshots add the steps explicitly —
`send-it` and `send-it-checked` both do. An `after_tasks` / `after_implement`
hook would make every Spec Kit flow pay the screenshot cost.

## Install

```bash
specify extension catalog add \
  https://raw.githubusercontent.com/clintcparker/speckit-addons/main/extensions/catalog.json \
  --name speckit-addons --install-allowed --priority 5

specify extension add screenshots
```

## License

MIT
