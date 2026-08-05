# Changelog

All notable changes to Site Checker are recorded here.

## Scaffold Cleanup — 2026-08-05

Removes the residue `create-tauri-app` left behind and sharpens two imprecise
strings. No new code, no new dependencies, and no behavior change beyond the
wording of one warning banner.

Spec: [`specs/001-scaffold-cleanup/spec.md`](specs/001-scaffold-cleanup/spec.md) ·
Plan: [`specs/001-scaffold-cleanup/plan.md`](specs/001-scaffold-cleanup/plan.md) ·
Tasks: [`specs/001-scaffold-cleanup/tasks.md`](specs/001-scaffold-cleanup/tasks.md)

### Removed

- The unregistered `opener` plugin, in all three places it was declared:
  `"opener:default"` from `src-tauri/capabilities/default.json`,
  `@tauri-apps/plugin-opener` from `package.json`, and `tauri-plugin-opener`
  from `src-tauri/Cargo.toml`. `src-tauri/src/lib.rs` only ever initialized
  `tauri_plugin_autostart`, so the plugin was granted permission surface and
  compiled into the shipped binary without being used. Both lockfiles were
  regenerated (US1).
- The three orphaned scaffold SVGs — `src/assets/tauri.svg`,
  `src/assets/typescript.svg`, and `src/assets/vite.svg`. No source file or
  `index.html` referenced them, and Vite already excluded them from the bundle,
  so `dist/` is byte-for-byte unaffected. `src/assets/` is now empty and gone (US2).

### Changed

- The package and crate now identify themselves as Site Checker instead of the
  scaffold: `package.json` `name` is `site-checker`; `src-tauri/Cargo.toml` carries
  `name = "site-checker"`, a real one-line `description`, and
  `authors = ["Clint Parker <me@clintparker.com>"]` in place of `authors = ["you"]` (US3).
- `[lib] name` renamed `tauri_app_lib` → `site_checker_lib`, with the matching
  `src-tauri/src/main.rs` call site updated in the same change — the one
  build-breaking ripple in this feature (US3).
- The corrupt-file warning in `src-tauri/src/store.rs::load` now names its actual
  cause (the file is not valid JSON) instead of reading like the neighbouring
  I/O-error message. It still reassures the user the file was left on disk.
  This is the only user-visible change in the feature (US4).
- The `has_leading_scheme` doc comment in `src-tauri/src/model.rs` now states the
  character-class rule its body applies — the text before `://` must be entirely
  ASCII alphanumeric or one of `+`, `-`, `.` (US4).

### Technical Notes

- The bundle is unchanged where it counts: `src-tauri/tauri.conf.json` pins
  `productName: "Site Checker"` and `identifier: com.clintparker.site-checker`
  and never referenced the crate name, so `pnpm tauri build --bundles app` still
  emits `Site Checker.app` at the same 15 MB.
- No persisted or IPC field was renamed. `sites.json` keeps its shape, its path,
  and its load semantics — only the warning text changed.
- Sequencing was constrained by one cross-story conflict: US1 and US3 both edit
  `package.json` and `src-tauri/Cargo.toml`, so they were serialized. US2 and US4
  touch disjoint files.
- Quality gates after every story, not just at the end: `cargo test` (29 passing),
  `pnpm test` (12 passing), and `cargo clippy -- -D warnings` (clean).
