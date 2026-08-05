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
  manifest.json                    # views, window sizes, seed sites, notes
  SKIPPED.md                       # written instead of images when the feature has no UI surface
  before/<view-slug>-<size>.png
  after/<view-slug>-<size>.png
```

Everything here except the user's real data file is committed to the feature branch so `speckit.ship.run` can embed the images in the PR description.

## Execution Steps

### 1. Locate the feature

Run `.specify/scripts/bash/check-prerequisites.sh --json` from repo root and parse `FEATURE_DIR`. All paths must be absolute.

### 2. Decide whether the feature is UI-relevant

- **Mode `before`**: read `FEATURE_DIR/spec.md` (and `plan.md` if present). The feature is UI-relevant iff it changes anything a user sees in the app window: the frontend (`src/`, `index.html`, CSS) or the window definition in `src-tauri/tauri.conf.json`. Backend-only Rust work (HTTP classifier internals, store, scheduling, config) is not — *unless* it changes user-visible output such as status `reason` strings or the shape of the `site-status` event. If not UI-relevant, write `FEATURE_DIR/screenshots/SKIPPED.md` containing one line explaining why, commit it (`docs: screenshots skipped — <reason>`), and stop successfully.
- **Mode `after`**: if `SKIPPED.md` exists, verify the prediction: `git diff --name-only $(git merge-base HEAD <target>)..HEAD -- src index.html src-tauri/tauri.conf.json`. If still empty, stop successfully. If implementation touched UI after all, delete `SKIPPED.md` and continue — there will be no baseline, so record `"baseline": "unavailable"` in the manifest and capture `after/` only.

### 3. Seed data (BEFORE launching)

The app reads `~/Library/Application Support/com.clintparker.site-checker/sites.json` (a bare JSON array of sites) at startup; there is no env override, so seeding means touching the user's real file. **Protect it**:

- If the file exists and no `sites.json.shots-backup` exists beside it, move it to `sites.json.shots-backup`. Record `"backup": true` in the manifest. Restoring this backup in step 7 is mandatory even if the run fails — treat it like a `trap`.
- Write the seed file. Use the manifest's `seed_sites` if it exists (mode `after` must reproduce the baseline exactly); otherwise pick 2–4 sites that exercise the states the feature touches and record them in the manifest. A dependable default:

```json
[
  { "id": "shots-up",   "url": "https://example.com",  "label": "Example",    "interval_secs": 60 },
  { "id": "shots-down", "url": "https://down.invalid", "label": "Never Up",   "interval_secs": 60 }
]
```

`.invalid` never resolves, so it renders the Down state without waiting on a real outage. Checks hit the real network; every site starts Pending on launch and results are never persisted.

### 4. Launch the app

- A fresh worktree has no `node_modules`: run `pnpm install` first if it is missing.
- Start `pnpm tauri dev` in the background, capturing stdout+stderr to a log file **outside the checkout** (the git auto-commit hooks would commit anything inside it). First cold cargo build can take several minutes — allow ~10 min before declaring failure; on failure, dump the log tail and stop with an error (a non-starting app is itself a finding worth reporting).
- Vite is pinned to port 1420 with `strictPort` — if a stale dev server holds it, kill that process first.
- The dev window belongs to process `site-checker` (bundled builds appear as `Site Checker`); its title is `Site Checker`. Poll System Events until it exists:
  `osascript -e 'tell application "System Events" to get position of window 1 of process "site-checker"'`
- Wait a few seconds after launch so Pending resolves to Up/Down before capturing — unless the feature is about the Pending state itself; use judgment and note the choice in the manifest.

### 5. Choose target views

This is a single-window app, so "pages" are views/states: the main site list, the empty state, the add-site form, an error banner — whatever the spec touches (1–4 views). Reach each one by driving the app (AppleScript keystrokes/clicks, or temporarily emptying the seed file for the empty state). Mode `after` must reuse the manifest's view list, adding any views the feature newly created.

### 6. Capture

For each view, capture the window at two sizes (the window is resizable, 480×320 minimum):

- `default`: 720×480 (the shipped size in `tauri.conf.json`)
- `narrow`: 480×320 (the floor — layout stress test)

Resize with System Events (`set size of window 1 of process "site-checker" to {W, H}`), then capture just the window: read `position` and `size` from System Events and run `screencapture -R x,y,w,h <file>` (`-l <windowid>` is fine too if you can get a CGWindowID). Filenames: `<view-slug>-<size>.png` under `before/` or `after/` per mode.

### 7. Record, commit, clean up

- Write/update `FEATURE_DIR/screenshots/manifest.json`:

```json
{
  "backup": true,
  "sizes": { "default": "720x480", "narrow": "480x320" },
  "seed_sites": [ { "id": "shots-up", "url": "https://example.com", "label": "Example", "interval_secs": 60 } ],
  "views": [ { "slug": "site-list", "why": "status row layout changed" } ],
  "notes": []
}
```

- Kill the `pnpm tauri dev` process **tree** (it spawns vite, cargo, and the app — kill the process group, then verify no `site-checker` process survives).
- Restore the user's data: if `sites.json.shots-backup` exists, move it back over `sites.json`; otherwise delete the seed `sites.json`. Do this in **both** modes, every run, success or failure — the manifest's `seed_sites` is what makes the `after` run reproducible, not leftover state.
- Commit `FEATURE_DIR/screenshots/` with message `docs: <mode> screenshots for <feature>`. Never commit the data file, dev-server logs, or anything outside `FEATURE_DIR/screenshots/`.

## Constraints

- This command never modifies application code. If the app fails to build or start in mode `after`, that is an implementation defect: report it clearly and stop — do not patch around it.
- Keep total image payload modest: PNG, window-sized, 1–4 views × 2 sizes.
- The user's real `sites.json` must survive every run — the backup/restore in steps 3 and 7 is not optional, and a crashed run must still restore it before reporting the failure.
- macOS screen-capture permission must already be granted to the terminal running the agent; if `screencapture` produces empty images, report that instead of retrying blindly.
