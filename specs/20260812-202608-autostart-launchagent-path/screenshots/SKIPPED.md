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

## Mode `after` — prediction verified, skip confirmed

Run on 2026-08-12 against target branch `main`, in worktree
`site-checker--20260812-202608-autostart-launchagent-path` at `a2f95cf`
(merge-base with `main`: `1a711b7`).

```sh
git diff --name-only $(git merge-base HEAD main)..HEAD -- src index.html src-tauri/tauri.conf.json assets
# (no output)
```

Empty — the implementation did not touch the UI surface, so per the command's
mode-`after` rule the skip stands and nothing was captured. The app was not
built or launched in this pass either, so the user's real `sites.json` was
again never touched.

What the implementation actually landed, checked against the profile's
backend carve-out rather than the path list alone:

| Change | Carve-out applies? |
|---|---|
| `src-tauri/src/autostart.rs` (new) — path resolution, plist repair | no — never surfaces in the window |
| `lib.rs` setup — manager is now optional, repair pass added | no — nominal path renders identically |
| `commands.rs` — `get_autostart` / `set_autostart` route through `autolaunch()` | see below |
| `README.md`, `install/homebrew/site-checker.rb` | no — documentation |
| `src/**`, `index.html` | **not modified**, as planned |

One new user-visible string exists — `"Site Checker could not determine its own
location, so it cannot manage the login item."`, which the unchanged frontend
renders as a banner with the checkbox disabled. It is unreachable in a capture:
it fires only when `canonicalize(current_exe())` fails, i.e. when the running
binary's own path has been deleted underneath it. Producing that state means
tampering with the running app, and this command never modifies application
code. There is also no `before/` baseline to pair it against. Its behaviour is
covered instead by the QA evidence under `../qa/` (see
`qa/responses/current-exe-canonicalize.txt` and the harness run).

In the nominal state a reviewer would see — app launched normally, login item
readable — the window is pixel-identical to `main`, which is exactly what
FR-009 requires.
