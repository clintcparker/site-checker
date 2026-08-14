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
FEATURE_DIR/
  .gitignore                       # written only when the repo ignores screenshots/ — see step 2
  screenshots/
    manifest.json                  # targets, viewports, baseline, app-specific state
    SKIPPED.md                     # written instead of images when the feature has no UI surface
    before/<target-slug>-<viewport>.png
    after/<target-slug>-<viewport>.png
```

Everything here except app state is committed to the feature branch so `speckit.ship.run` can embed the images in the PR description. **Committed means tracked** — `git add -f` is never the answer here, because the PR body links these paths and a link to an ignored path 404s.

## The app profile

This command is generic. Everything that depends on *this* app — how to launch it, how to sign in, where its data lives, what counts as a "view", how a screenshot is actually taken — lives in the profile at:

```
.specify/extensions/screenshots/screenshots-config.yml
```

Read it before step 4 and follow it. Its sections are `ui_surface`, `launch`, `auth`, `data`, `targets`, `viewports`, `capture_method`, and `cleanup`.

**If the profile has `unconfigured: true`**, derive it yourself before continuing: read the repo README, build manifests (`package.json`, `*.csproj`, `Cargo.toml`, `pyproject.toml`, `go.mod`), the app entry point, and any existing e2e/browser config (Playwright, Cypress, Puppeteer, Tauri). Write the profile with your findings, set `unconfigured: false`, record `"profile": "auto-generated"` in the manifest's `notes`, and continue. Do not stop to ask — a repo-agnostic install must be runnable with zero manual steps, and the profile is reviewable after the fact. The `examples/` directory in this extension shows two filled-in profiles.

## Execution Steps

### 1. Locate the feature

`FEATURE_DIR` comes from three sources, listed in priority order. The highest one that answers is the value the rest of the command uses:

1. **`$SPECIFY_FEATURE_DIRECTORY`** — the explicit override. A workflow that pinned this run's feature exports it, and it outranks anything the checkout says.
2. **`.specify/run-context.json`** → its `feature_dir`. Look for that file under `$SPECIFY_INIT_DIR` first, then in the current directory, then in the primary checkout.
3. **`.specify/scripts/bash/check-prerequisites.sh --paths-only --json`** → parse `FEATURE_DIR`.

All paths must be absolute; normalize a relative one against the repo root before using it.

**Use `--paths-only`.** Plain `check-prerequisites.sh --json` validates before it reports, and one of its gates is `plan.md`: it exits 1 with `ERROR: plan.md not found` for every feature that never ran `/speckit-plan`. A screenshot pass needs a feature *directory*, not a plan. `--paths-only` performs the same resolution with no validation, and — unlike the plain form — does not persist the override into `.specify/feature.json` on the way past.

**Source 3 is a fallback and also a cross-check.** When 1 or 2 answered, run it anyway and compare. If it names a different feature it has resolved the wrong one: with no override in the environment it answers from `.specify/feature.json`, and in the primary checkout right after a merge that names the feature which shipped last. **Stop and report the disagreement, naming both paths.** Do not adopt the script's answer, and do not silently prefer the pinned one either — a mismatch means the override is not reaching `.specify/scripts/**`, and every later step of the run inherits that. A script exiting 0 is not evidence it found the right feature.

If source 3 fails outright while 1 or 2 answered, continue with the pinned value and record the failure in the manifest's `notes`. If nothing answers, stop with an error naming all three sources.

### 2. Make the screenshots directory trackable

Everything this command writes has to be **tracked**, because `speckit.ship.run` links it from the pull request body — an image git considers ignored is never in the pushed head, and its URL 404s for every reviewer. Repos routinely ignore `screenshots/` (or `specs/*/screenshots/`) for unrelated reasons, so check before writing anything:

```bash
git check-ignore -v -- "$FEATURE_DIR/screenshots/"
```

The directory does not have to exist yet — `check-ignore` matches pathnames, not files. Exit status 1 with no output means *not ignored*; that is the good case, not a failure. Continue.

If it reports a rule, do **not** work around it with `git add -f` — that stages the files once and leaves the after pass, the ship step and the reviewer's checkout facing the same conflict. Un-ignore the directory once, in a file that is itself committed:

```bash
printf '!screenshots/\n' >> "$FEATURE_DIR/.gitignore"
```

A `.gitignore` inside the feature directory outranks the repo root's, so that one line makes plain `git add` work here and in every later step. Re-run `git check-ignore` to confirm, and commit the `.gitignore` with the artifacts. Skip the append when the line is already there.

If `FEATURE_DIR` **itself** is the ignored path, its own `.gitignore` is never read — git does not descend into an excluded directory. Append the negation for the feature directory to the repo-root `.gitignore` first, then handle `screenshots/` as above.

### 3. Decide whether the feature is UI-relevant

- **Mode `before`**: read `FEATURE_DIR/spec.md` and `FEATURE_DIR/plan.md`, whichever exist. The feature is UI-relevant iff it changes something a user sees, per the profile's `ui_surface`. If not UI-relevant, write `FEATURE_DIR/screenshots/SKIPPED.md` containing one line explaining why, commit it (`docs: screenshots skipped — <reason>`), and stop successfully.

  **A feature with neither document is not an error here.** A branch started by hand, or a run that bypassed `/speckit-specify`, reaches this command with an empty feature directory — step 1 resolves it regardless. With nothing to read, judge from what there is: `$ARGUMENTS`, the branch name, and the profile's `ui_surface`. When that is not enough to decide, **capture the baseline anyway**. A baseline that turns out unnecessary costs two PNGs; a missing one cannot be recreated once the implementation has landed. Record `"spec": "unavailable"` in the manifest's `notes` either way.
- **Mode `after`**: if `SKIPPED.md` exists, verify the prediction with
  `git diff --name-only $(git merge-base HEAD <target>)..HEAD -- <ui_surface.paths>`,
  where `<target>` is the target branch named in `$ARGUMENTS` if given, else the profile's default, else the repo's default branch. If the diff is still empty, stop successfully. If implementation touched UI after all, delete `SKIPPED.md` and continue — there will be no baseline, so record `"baseline": "unavailable"` in the manifest and capture `after/` only.

### 4. Prepare data and launch the app

Follow the profile's `data` and `launch` sections, in whichever order the profile specifies — some apps must be seeded before launch, others after.

Two rules hold regardless of profile:

- **App state never lives inside the repo or worktree.** Auto-commit hooks would commit a database, a log, or a lockfile. Keep data directories and server logs on a path outside the checkout.
- **Real user data must survive every run, including a failed one.** If the profile's `data` section describes a backup/restore of a real file, treat the restore like a `trap`: perform it in both modes, on success and on failure, before reporting anything.

Mode `after` reuses the baseline state recorded in the manifest's `app` object so the before/after pair differs only by the UI change. If that state is gone, recreate it by replaying whatever the profile calls the seed procedure, then continue.

If the app fails to build or start, dump the log tail and stop with an error — a non-starting app is itself a finding worth reporting.

### 5. Authenticate

Follow the profile's `auth` section. If it says `none`, skip this step.

### 6. Choose targets

A "target" is whatever the profile's `targets` section says a capturable unit is — a page, a route, a window state, a view. Choose 1–4 from the spec: the ones the feature changes, plus the app's main screen if it is affected. Record each as `{ "slug": ..., "why": ... }`.

Mode `after` **must** reuse the manifest's target list, adding any targets the feature newly created.

### 7. Capture

For each target, capture at every viewport in the profile's `viewports` map. Use the profile's `capture_method`. Filenames: `<target-slug>-<viewport-label>.png` under `before/` or `after/` per mode.

Keep the total payload modest: PNG, viewport- or window-sized, 1–4 targets × the declared viewports.

### 8. Record, commit, clean up

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

Then clean up per the profile's `cleanup` section, and commit `FEATURE_DIR/screenshots/` — plus `FEATURE_DIR/.gitignore` if step 2 wrote one — with message `docs: <mode> screenshots for <feature>`. Use a plain `git add`, never `git add -f`: step 2 is what makes the plain form work, and a force-add hides that it did not. Never commit app data, server logs, or anything outside `FEATURE_DIR/`.

## Constraints

- This command **never modifies application code**. If the app fails to build or start in mode `after`, that is an implementation defect: report it clearly and stop — do not patch around it.
- The data-protection rules in step 4 are not optional, and a crashed run must still restore real user data before reporting the failure.
- If the profile pins a port and it is occupied, pick another free port and use it consistently everywhere the profile references one (sign-in links and callback URLs are often stamped from it).
- **A spec-less feature is a supported input, not a failure.** Nothing in this command requires `spec.md` or `plan.md` to exist; the only thing it needs is the feature *directory*. Resolve it per step 1 and never gate the run on a document a helper script happens to validate.
- **Never adopt a feature the run's own context does not name.** A helper script that resolves a different one has found the wrong one — report both and stop, rather than screenshotting whatever the checkout is standing on.
- **`git add -f` is not a fix for an ignored screenshots directory.** It stages the files for one commit and leaves the after pass, the ship step and the reviewer's checkout facing the same conflict. Step 2 resolves it once, in a committed file.
- The `.gitignore` step 2 may write is the **only** file this command creates outside `FEATURE_DIR/screenshots/`, and it is one line. It never edits an existing ignore rule; it only appends a negation.
