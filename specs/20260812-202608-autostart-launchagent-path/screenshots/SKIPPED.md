# Screenshots skipped — launch-at-login survives upgrades

No UI surface: this feature changes only where the launch-at-login registration points, and the spec states explicitly that the "Launch at login" checkbox, its label, its behaviour, and the state it reports are all unchanged (Assumptions; FR-009).

## Evidence

Judged in mode `before` on 2026-08-12 against `ui_surface.paths` in `.specify/extensions/screenshots/screenshots-config.yml` — `src`, `index.html`, `src-tauri/tauri.conf.json`, `assets`.

Per `plan.md` §Structure, the change touches:

| File | UI surface? |
|---|---|
| `src-tauri/src/autostart.rs` (new) | no — backend |
| `src-tauri/src/lib.rs`, `commands.rs`, `Cargo.toml` | no — backend |
| `README.md` | no — documentation |
| Homebrew formula caveats (`install/`) | no — documentation |
| `src/**`, `index.html` | **not modified** — plan marks the frontend UNCHANGED |

The profile's carve-out for backend changes with visible effects (status strings, event shape, warning banners, control state) does not apply: FR-009 requires the checkbox to report the *same* state before and after a repair, and FR-007 requires an unreadable registration to be handled with nothing reported to the user. There is deliberately nothing new to see.

## Notes

- The app profile was `unconfigured: true` at the start of this run and was derived and written back in the same commit, per the command's unattended-run rule. It is recorded here rather than in a manifest because the skip path writes no manifest. Worth a look in review: `.specify/extensions/screenshots/screenshots-config.yml`.
- The app was never built or launched, so no baseline exists and the user's real `sites.json` was never touched.
- `specs/*/screenshots/` is gitignored in this repo, so this file was committed with `git add -f`.

## For mode `after`

Verify this prediction with:

```sh
git diff --name-only $(git merge-base HEAD main)..HEAD -- src index.html src-tauri/tauri.conf.json assets
```

If that diff is non-empty, the implementation touched UI after all: delete this file, capture `after/` only, and record `"baseline": "unavailable"` in the manifest.
