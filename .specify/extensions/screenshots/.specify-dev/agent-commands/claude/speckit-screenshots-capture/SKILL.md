---
name: speckit-screenshots-capture
description: Capture before/after UI screenshots for the current feature and stage
  them on the branch for the pull request.
compatibility: Requires spec-kit project structure with .specify/ directory
metadata:
  author: github-spec-kit
  source: screenshots:commands/capture.md
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
  manifest.json                    # pages, viewports, seed steps, data-dir path, notes
  SKIPPED.md                       # written instead of images when the feature has no UI surface
  before/<page-slug>-<viewport>.png
  after/<page-slug>-<viewport>.png
```

Everything here except the app data directory is committed to the feature branch so `speckit.ship.run` can embed the images in the PR description.

## Execution Steps

### 1. Locate the feature

Run `.specify/scripts/bash/check-prerequisites.sh --json` from repo root and parse `FEATURE_DIR`. All paths must be absolute.

### 2. Decide whether the feature is UI-relevant

- **Mode `before`**: read `FEATURE_DIR/spec.md` (and `plan.md` if present). The feature is UI-relevant iff it changes anything a user sees in a browser: Razor pages under `src/HomeApp/Pages/`, `src/HomeApp/wwwroot/` (CSS, images), the action/invite confirm pages, or layout/shared partials. Backend-only work (feed `.ics` internals, lifecycle rules, migrations, config) is not. If not UI-relevant, write `FEATURE_DIR/screenshots/SKIPPED.md` containing one line explaining why, commit it (`docs: screenshots skipped — <reason>`), and stop successfully.
- **Mode `after`**: if `SKIPPED.md` exists, verify the prediction: `git diff --name-only $(git merge-base HEAD <target>)..HEAD -- src/HomeApp/Pages src/HomeApp/wwwroot`. If still empty, stop successfully. If implementation touched UI after all, delete `SKIPPED.md` and continue — there will be no baseline, so record `"baseline": "unavailable"` in the manifest and capture `after/` only.

### 3. Launch the app

- **Data directory**: NEVER inside the repo or worktree — the git auto-commit hooks would commit a SQLite database. Use a temp path outside the checkout.
  - Mode `before`: create a fresh directory (e.g. `mktemp -d -t homeapp-shots`), record its absolute path in the manifest as `data_dir`.
  - Mode `after`: reuse `data_dir` from the manifest if it still exists (startup migrations will upgrade the schema, and reusing it keeps the data identical so the before/after diff shows only the UI change). If it is gone, create a fresh one and replay the manifest's `seed_steps`.
- Provision the screenshot user (idempotent — skip if mode `after` and the data dir survived):
  `dotnet run --project src/HomeApp -- provision shots@example.test --name "Screenshot Bot"`
- Start the server in the background with `DATA_DIR=<data_dir> APP_BASE_URL=http://localhost:8123 ASPNETCORE_URLS=http://localhost:8123 dotnet run --project src/HomeApp`, capturing stdout to a log file. Poll `GET http://localhost:8123/healthz` until healthy (timeout ~60s; on failure, dump the log tail and stop with an error — a non-starting app is itself a finding worth reporting).

### 4. Sign in via the magic-link flow (Playwright)

With no ACS/SMTP configured the app's console email fallback logs the sign-in link to stdout. Using the Playwright browser tools:

1. Navigate to `http://localhost:8123/login`, submit `shots@example.test`.
2. Grep the server log for the most recent sign-in URL and navigate to it. The resulting cookie lasts the whole session.

### 5. Seed representative data

- Mode `before`: through the UI, create the minimum data that makes the target pages meaningful (typically: one home, one entity such as an appliance, one maintenance task so the dashboard and occurrence pages are non-empty). Record each step tersely in the manifest's `seed_steps` array so mode `after` can replay them if the data dir was lost.
- Mode `after`: only replay `seed_steps` if the data dir had to be recreated; then additionally create whatever new data the feature itself introduces (a new field, a new page's records) so the change is visible.

### 6. Capture

- Choose 1–4 target pages from the spec — the pages the feature changes, plus the dashboard if it is affected. Mode `after` must reuse the manifest's page list (adding any pages the feature newly created).
- For each page, capture at two viewports (the app is mobile-first with one breakpoint at 768px):
  - `mobile`: 390×844
  - `desktop`: 1280×900
- Viewport screenshots (not full-page) unless the change is below the fold. Filenames: `<page-slug>-<viewport>.png` under `before/` or `after/` per mode.

### 7. Record, commit, clean up

- Write/update `FEATURE_DIR/screenshots/manifest.json`:

```json
{
  "data_dir": "/abs/path",
  "viewports": { "mobile": "390x844", "desktop": "1280x900" },
  "seed_steps": ["create home 'Test House'", "add entity 'Dishwasher' with photo", "..."],
  "pages": [ { "slug": "dashboard", "path": "/", "why": "task list layout changed" } ],
  "notes": []
}
```

- Kill the `dotnet run` process and close the browser tab. Leave the data dir in place after mode `before` (mode `after` wants it); after mode `after`, delete it.
- Commit `FEATURE_DIR/screenshots/` with message `docs: <mode> screenshots for <feature>`. Never commit the data dir, server logs, or anything outside `FEATURE_DIR/screenshots/`.

## Constraints

- This command never modifies application code. If the app fails to build or start in mode `after`, that is an implementation defect: report it clearly and stop — do not patch around it.
- Keep total image payload modest: PNG, viewport-sized, 1–4 pages × 2 viewports.
- Port 8123 is chosen to avoid the dev-default 8080; if it is occupied, pick another free port and use it consistently for `APP_BASE_URL` too (sign-in links are stamped from it).