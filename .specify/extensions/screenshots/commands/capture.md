---
description: Capture before/after UI screenshots for the current feature and stage them on the branch for the pull request.
---

## User Input

```text
$ARGUMENTS
```

You **MUST** consider the user input before proceeding. It must name a mode: `before` (baseline, run prior to implementation) or `after` (run once implementation is complete). If neither word is present, stop and report a usage error.

## Purpose

Produce visual evidence that the app runs and the change looks right — a cheap end-to-end smoke test that doubles as PR documentation. Output layout, all under the current feature's directory (`FEATURE_DIR`):

```
FEATURE_DIR/screenshots/
  manifest.json                    # targets, viewports, baseline, app-specific state
  SKIPPED.md                       # written instead of images when the feature has no UI surface
  before/<target-slug>-<viewport>.png
  after/<target-slug>-<viewport>.png
```

Everything here except app state is committed to the feature branch so `speckit.ship.run` can embed the images in the PR description.

## The app profile

This command is generic. Everything that depends on *this* app — how to launch it, how to sign in, where its data lives, what counts as a "view", how a screenshot is actually taken — lives in the profile at:

```
.specify/extensions/screenshots/screenshots-config.yml
```

Read it before step 3 and follow it. Its sections are `ui_surface`, `launch`, `auth`, `data`, `targets`, `viewports`, `capture_method`, and `cleanup`.

**If the profile has `unconfigured: true`**, derive it yourself before continuing: read the repo README, build manifests (`package.json`, `*.csproj`, `Cargo.toml`, `pyproject.toml`, `go.mod`), the app entry point, and any existing e2e/browser config (Playwright, Cypress, Puppeteer, Tauri). Write the profile with your findings, set `unconfigured: false`, record `"profile": "auto-generated"` in the manifest's `notes`, and continue. Do not stop to ask — a repo-agnostic install must be runnable with zero manual steps, and the profile is reviewable after the fact. The `examples/` directory in this extension shows two filled-in profiles.

## Execution Steps

### 1. Locate the feature

Run `.specify/scripts/bash/check-prerequisites.sh --json` from repo root and parse `FEATURE_DIR`. All paths must be absolute.

### 2. Decide whether the feature is UI-relevant

- **Mode `before`**: read `FEATURE_DIR/spec.md` (and `plan.md` if present). The feature is UI-relevant iff it changes something a user sees, per the profile's `ui_surface`. If not UI-relevant, write `FEATURE_DIR/screenshots/SKIPPED.md` containing one line explaining why, commit it (`docs: screenshots skipped — <reason>`), and stop successfully.
- **Mode `after`**: if `SKIPPED.md` exists, verify the prediction with
  `git diff --name-only $(git merge-base HEAD <target>)..HEAD -- <ui_surface.paths>`,
  where `<target>` is the target branch named in `$ARGUMENTS` if given, else the profile's default, else the repo's default branch. If the diff is still empty, stop successfully. If implementation touched UI after all, delete `SKIPPED.md` and continue — there will be no baseline, so record `"baseline": "unavailable"` in the manifest and capture `after/` only.

### 3. Prepare data and launch the app

Follow the profile's `data` and `launch` sections, in whichever order the profile specifies — some apps must be seeded before launch, others after.

Two rules hold regardless of profile:

- **App state never lives inside the repo or worktree.** Auto-commit hooks would commit a database, a log, or a lockfile. Keep data directories and server logs on a path outside the checkout.
- **Real user data must survive every run, including a failed one.** If the profile's `data` section describes a backup/restore of a real file, treat the restore like a `trap`: perform it in both modes, on success and on failure, before reporting anything.

Mode `after` reuses the baseline state recorded in the manifest's `app` object so the before/after pair differs only by the UI change. If that state is gone, recreate it by replaying whatever the profile calls the seed procedure, then continue.

If the app fails to build or start, dump the log tail and stop with an error — a non-starting app is itself a finding worth reporting.

### 4. Authenticate

Follow the profile's `auth` section. If it says `none`, skip this step.

### 5. Choose targets

A "target" is whatever the profile's `targets` section says a capturable unit is — a page, a route, a window state, a view. Choose 1–4 from the spec: the ones the feature changes, plus the app's main screen if it is affected. Record each as `{ "slug": ..., "why": ... }`.

Mode `after` **must** reuse the manifest's target list, adding any targets the feature newly created.

### 6. Capture

For each target, capture at every viewport in the profile's `viewports` map. Use the profile's `capture_method`. Filenames: `<target-slug>-<viewport-label>.png` under `before/` or `after/` per mode.

Keep the total payload modest: PNG, viewport- or window-sized, 1–4 targets × the declared viewports.

### 7. Record, commit, clean up

Write/update `FEATURE_DIR/screenshots/manifest.json`:

```json
{
  "targets": [ { "slug": "dashboard", "why": "task list layout changed" } ],
  "viewports": { "mobile": "390x844", "desktop": "1280x900" },
  "baseline": "available",
  "notes": [],
  "app": {}
}
```

- `targets` — the captured units, each with a `slug` and a `why`.
- `viewports` — label → `WxH`, copied from the profile.
- `baseline` — `"available"` or `"unavailable"`.
- `notes` — free-form strings. Record failures here rather than dropping them.
- `app` — free-form, profile-specific state that mode `after` needs in order to
  reproduce mode `before`: a data directory path, a backup flag, seed steps, seed
  records. Its shape is the profile's business, not this command's.

Then clean up per the profile's `cleanup` section, and commit `FEATURE_DIR/screenshots/` with message `docs: <mode> screenshots for <feature>`. Never commit app data, server logs, or anything outside `FEATURE_DIR/screenshots/`.

## Constraints

- This command **never modifies application code**. If the app fails to build or start in mode `after`, that is an implementation defect: report it clearly and stop — do not patch around it.
- The data-protection rules in step 3 are not optional, and a crashed run must still restore real user data before reporting the failure.
- If the profile pins a port and it is occupied, pick another free port and use it consistently everywhere the profile references one (sign-in links and callback URLs are often stamped from it).
