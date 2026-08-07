# Screenshots skipped — Packaging & Distribution

Skipped because this feature changes how the app is built, published, and installed — never
what the app window shows, so a before/after image pair would be two identical pictures.

## Evidence (mode `before`, unattended run, 2026-08-06)

Judgement made against `spec.md`, `plan.md`, and `tasks.md`:

- **`spec.md` § Out of Scope** — "Any change to what the application does. This feature changes
  how the application is built, published, and installed — not its behaviour, its stored data
  format, or its interface."
- **`plan.md:144`** — "Application code under `src/` and `src-tauri/src/` is untouched."
- **`tasks.md:355`** — "**Zero changes to `src/` or `src-tauri/src/`.** If a task appears to
  require one, stop — it is out of scope per the spec."
- **`plan.md:129-136`** (full edit list) names no `src/` file, no `index.html`, and no CSS.
- **`src-tauri/tauri.conf.json` is edited, but not its window definition.** T009 deletes the
  top-level `"version"` key; T010 changes `bundle.targets` from `["app","dmg"]` to `["app"]`.
  The `app.windows` block — `title`, `width` 720, `height` 480, `minWidth` 480, `minHeight` 320,
  `resizable` — is untouched, so both capture sizes this skill uses stay valid.
- **No user-visible output strings change.** `plan.md:84` — "No persisted or event field name is
  touched." No status `reason` string and no `site-status` event shape is in scope, which is the
  one backend-only case that would still have made this UI-relevant.

## The one thing that looked UI-relevant and is not

US2 acceptance scenario 3 says the user "inspects the version the application reports". That
version is the bundle's `CFBundleShortVersionString`, read out of `Info.plist` — T014 validates it
with `/usr/libexec/PlistBuddy`, not by looking at the window. No version is rendered in the UI
before or after this feature.

## Consequence for the `after` run

`speckit-screenshots-capture` in mode `after` must re-check this prediction:

```
git diff --name-only $(git merge-base HEAD main)..HEAD -- src index.html src-tauri/tauri.conf.json
```

`tauri.conf.json` **is expected to appear** in that diff (T009/T010 above). That alone does not
overturn this skip — confirm the diff does not touch `app.windows` before concluding the feature
gained a UI surface. If any `src/` or `index.html` file appears, delete this file, capture `after/`
only, and record `"baseline": "unavailable"` in the manifest.

## Run notes

No app launch and no data seeding were performed, so the user's real
`~/Library/Application Support/com.clintparker.site-checker/sites.json` was never touched
(verified intact, 803 bytes, with no leftover `sites.json.shots-backup` from earlier runs).

## Verification (mode `after`, unattended run, 2026-08-06)

**The skip holds. No screenshots captured, in either direction.**

The re-check the `before` pass demanded was run against `main` as the target branch:

```
$ git diff --name-only $(git merge-base HEAD main)..HEAD -- src index.html src-tauri/tauri.conf.json
src-tauri/tauri.conf.json
```

`src/` and `index.html` are absent, so the feature gained no UI surface and no `after/`-only
capture is warranted. `tauri.conf.json` appears exactly as predicted, and its diff is the two
anticipated hunks and nothing else:

- **T009** — the top-level `"version": "0.1.0"` key is deleted.
- **T010** — `bundle.targets` goes from `["app", "dmg"]` to `["app"]`.

`app.windows` is byte-for-byte unchanged — still `title: "Site Checker"`, `width` 720,
`height` 480, `minWidth` 480, `minHeight` 320, `resizable: true` — which was the stated condition
for the skip to survive a `tauri.conf.json` edit. The window the two capture sizes target is the
same window it was before the feature.

No baseline exists and none is needed, so no `manifest.json` was written; this file remains the
whole record. The user's data file was again never touched — re-verified intact at 803 bytes,
5 real sites, no `shots-*` seed entries, and no `sites.json.shots-backup` beside it. No app
launch was attempted, so this run makes no claim about whether the app still builds; that is
`ship`'s CI gate and the `qa` run to establish, not this one's.
