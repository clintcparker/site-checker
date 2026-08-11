# Screenshots skipped — no UI surface

This feature only bumps GitHub Actions pins to Node 24 majors and adds `.github/dependabot.yml`; it changes nothing a user sees in the app window.

## How this was decided (unattended run, mode `before`, target `main`)

There is no `spec.md` or `plan.md` — this feature bypassed `/speckit-specify` and `/speckit-plan` — so UI-relevance was judged from `tasks.md` instead. Every file path named across T001–T020 is one of:

- `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/workflows/verify-install-channels.yml`
- `.github/dependabot.yml`
- `docs/how-to/release.md`

None of `src/`, `index.html`, or `src-tauri/tauri.conf.json` is touched, and no task changes user-visible output such as status `reason` strings or the shape of the `site-status` event. `tasks.md` states this directly under "Path Conventions": *"This feature touches CI configuration only... No `src/`, `src-tauri/`, or `tests/` code changes."*

Because the decision was reachable without launching the app, no `pnpm tauri dev` run and no `sites.json` seeding took place — the user's real data file was never touched, so no backup/restore was required.

## Prediction for the `after` run

`git diff --name-only $(git merge-base HEAD main)..HEAD -- src index.html src-tauri/tauri.conf.json` should be empty. If it is not, this file gets deleted, and the `after` capture records `"baseline": "unavailable"` since no `before/` images exist.
