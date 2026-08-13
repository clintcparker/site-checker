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
