# Changelog

## 0.1.0 (2026-08-05)

### Added
- `speckit.screenshots.capture` command — before/after UI screenshots for the
  current feature, committed to the branch for embedding in the pull request.
- Generic command + per-repo app profile split: `commands/capture.md` is never
  edited per repo; `screenshots-config.yml` holds everything app-specific and
  survives `specify extension add --force`.
- Bootstrap mode — a profile marked `unconfigured: true` is derived by the agent
  on first run, so a fresh install is runnable with no manual steps.
- Example profiles for ASP.NET + Playwright and Tauri + AppleScript.
- Generalized manifest schema: `targets`, `viewports`, `baseline`, `notes`, and
  a free-form `app` object for profile-specific state.

### Notes
- Extracted from the hand-written `capture.md` variants in `homeapp1` and
  `site-checker`, which shared the entire command skeleton and differed only in
  app mechanics.
