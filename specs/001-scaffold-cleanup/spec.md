# Feature Specification: Scaffold Cleanup

**Feature Branch**: `001-scaffold-cleanup`

**Created**: 2026-08-05

**Status**: Draft

**Input**: Section 1 ("Cleanup — safe, cosmetic, do anytime") of `docs/ROADMAP.md`

## Overview

Site Checker v1 shipped from the `create-tauri-app` scaffold and never swept the
leftovers. The app carries an unregistered plugin, three unreferenced SVGs, and
manifest metadata that still says `tauri-app` / `"A Tauri App"` / `authors = ["you"]`.
Two comment/message strings are also imprecise enough to mislead a reader or a user.

None of this is user-visible today — the bundle identifier (`com.clintparker.site-checker`)
and window title (`Site Checker`) are already correct. This feature removes the dead
weight and sharpens the two strings so the repo reads like it was written for this app
rather than inherited from a template.

## Clarifications

Resolved from direct inspection of the tree on 2026-08-05:

- **The opener plugin is dead in three places, not two.** The roadmap names
  `src-tauri/capabilities/default.json` (`"opener:default"`) and `package.json`
  (`@tauri-apps/plugin-opener`). Inspection found a third: `src-tauri/Cargo.toml`
  carries `tauri-plugin-opener = "2"`. `src-tauri/src/lib.rs` registers only
  `tauri_plugin_autostart` — the opener plugin is never initialized, and no source
  file references it. All three declarations are removed together; leaving the Rust
  crate behind would keep compiling it into the bundle.
- **The SVGs are genuinely unreferenced.** `grep` over `src/` and `index.html` returns
  no hit for `tauri.svg`, `typescript.svg`, `vite.svg`, or any `assets/` path.
- **Renaming the Cargo package has one ripple.** `src-tauri/Cargo.toml` sets
  `[lib] name = "tauri_app_lib"`, and `src-tauri/src/main.rs` calls
  `tauri_app_lib::run()`. Renaming the package without renaming the lib and its
  caller breaks the build. `src-tauri/tauri.conf.json` does **not** reference the
  crate name — it uses `productName: "Site Checker"` — so the bundle output is
  unaffected.
- **Both store strings are in `load()`.** The I/O-error arm emits
  `"Could not read sites.json ({e}). Starting empty."`; the parse-error arm emits
  `"sites.json could not be read ({e}). Starting with an empty list; the existing
  file has been left alone."` A user seeing the second cannot tell it means "the
  contents are not valid JSON" rather than "the file could not be opened."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Remove the dead opener plugin (Priority: P1)

A maintainer opening `Cargo.toml`, `package.json`, or the capabilities file sees only
dependencies and permissions the app actually uses. The build does not compile or
declare a plugin that is never registered, and the app grants no capability it
does not exercise.

**Why this priority**: This is the only item with a functional dimension — an unused
capability in `default.json` is granted permission surface, and the Rust crate is
compiled into the shipped binary. Everything else in this feature is text.

**Independent Test**: Remove the three declarations, then run `cargo build` and
`pnpm build`; both succeed. `grep -rn "opener" src/ src-tauri/src/ src-tauri/capabilities/ package.json`
returns nothing. The app launches, lists sites, and checks them exactly as before.

**Acceptance Scenarios**:

1. **Given** `src-tauri/capabilities/default.json` lists `"opener:default"`, **When** that
   entry is removed leaving `"core:default"`, **Then** the app builds and runs and the
   capability file is valid against its schema.
2. **Given** `package.json` depends on `@tauri-apps/plugin-opener`, **When** the dependency
   is removed and the lockfile refreshed, **Then** `pnpm build` and `pnpm test` pass.
3. **Given** `src-tauri/Cargo.toml` depends on `tauri-plugin-opener`, **When** that line is
   removed, **Then** `cargo build` succeeds with no unresolved import, because no source
   file references the crate.

---

### User Story 2 - Delete orphaned scaffold assets (Priority: P2)

The `src/assets/` directory contains only assets the app actually uses. A reader
browsing the tree is not misled into thinking the Tauri/Vite/TypeScript logos are
part of the UI.

**Why this priority**: Zero risk and zero build impact (Vite already tree-shakes
them out of the bundle), but it is independent of every other story.

**Independent Test**: Delete the three files, run `pnpm build`, and confirm the build
succeeds and the emitted `dist/` contains the same assets as before. Launch the app
and confirm the UI is unchanged.

**Acceptance Scenarios**:

1. **Given** `src/assets/` holds `tauri.svg`, `typescript.svg`, and `vite.svg` with no
   referencing `<img>` or import, **When** all three are deleted, **Then** `pnpm build`
   succeeds and the rendered UI is visually identical.

---

### User Story 3 - Fix scaffold identity metadata (Priority: P2)

The npm package and Cargo crate identify themselves as Site Checker, authored by
this project's author, with a description of what the app actually does.

**Why this priority**: Purely cosmetic — the user-visible identifiers are already
correct — but it carries the one build-breaking ripple in this feature (the
`tauri_app_lib` rename), so it must not be done casually alongside other edits.

**Independent Test**: Rename the package, lib, and caller together, then run
`cargo build`, `cargo test`, `cargo clippy -- -D warnings`, and `pnpm build`. All pass.
The produced `.app` is still named `Site Checker`.

**Acceptance Scenarios**:

1. **Given** `package.json` has `"name": "tauri-app"`, **When** it is renamed to
   `"site-checker"`, **Then** `pnpm install`, `pnpm build`, and `pnpm test` all succeed.
2. **Given** `src-tauri/Cargo.toml` has `name = "tauri-app"`, `description = "A Tauri App"`,
   and `authors = ["you"]`, **When** these are set to the real values, **Then**
   `cargo build` succeeds.
3. **Given** `[lib] name = "tauri_app_lib"` and `main.rs` calls `tauri_app_lib::run()`,
   **When** the lib is renamed, **Then** `main.rs` is updated in the same change and
   `cargo build` succeeds with no unresolved-crate error.
4. **Given** the rename is complete, **When** `pnpm tauri build` runs, **Then** the
   bundle is still `Site Checker.app` with identifier `com.clintparker.site-checker`.

---

### User Story 4 - Make two strings say what they mean (Priority: P3)

A reader of `has_leading_scheme` learns which characters count as a scheme without
reading the body, and a user hitting the corrupt-file banner can tell it apart from
a permissions problem.

**Why this priority**: Text-only, no build impact, no behavior change. Last because
it is the least consequential, though the store message has the most real-world
value of the two.

**Independent Test**: `cargo test` and `cargo clippy -- -D warnings` stay green. The
existing `corrupt_file_yields_an_empty_list_a_warning_and_is_left_on_disk` test still
passes (it must not assert on exact wording; if it does, update the assertion).

**Acceptance Scenarios**:

1. **Given** the `has_leading_scheme` doc comment explains only the leading-`://` rule,
   **When** it is extended, **Then** it also states that the prefix must be ASCII
   alphanumeric or `+`, `-`, `.` — matching the `matches!` arm in the body.
2. **Given** the I/O-error and parse-error warnings in `store.rs::load` both read as
   "could not be read", **When** the parse-error message is reworded, **Then** it
   names the actual cause (the file is not valid JSON) and remains distinguishable
   from the I/O message at a glance.
3. **Given** the reworded message, **When** `cargo test` runs, **Then** the corrupt-file
   test still passes.

---

### Edge Cases

- **Lockfiles**: removing `@tauri-apps/plugin-opener` requires `pnpm-lock.yaml` to be
  regenerated, and removing `tauri-plugin-opener` requires `src-tauri/Cargo.lock` to be
  regenerated. Both must be committed, or the next clean checkout installs a dependency
  the manifest no longer declares.
- **Renaming the Cargo package** changes the compiled binary's name. `tauri.conf.json`
  does not pin it, so the bundle is unaffected — but a stale `src-tauri/target/` may
  hold artifacts under the old name. These are ignorable build residue, not a defect.
- **Capability schema**: `default.json` references `../gen/schemas/desktop-schema.json`,
  which is generated at build time. Editing the permissions array does not require
  regenerating it.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The build MUST NOT declare, compile, or grant any capability for the
  opener plugin in `package.json`, `src-tauri/Cargo.toml`, or
  `src-tauri/capabilities/default.json`.
- **FR-002**: `src/assets/` MUST NOT contain files that no source file references.
- **FR-003**: `package.json` `name` and `src-tauri/Cargo.toml` `name`, `description`,
  and `authors` MUST describe Site Checker, not the scaffold template.
- **FR-004**: Renaming the Cargo package MUST leave the build green, which requires
  updating `[lib] name` and its caller in `src-tauri/src/main.rs` in the same change.
- **FR-005**: The `has_leading_scheme` doc comment MUST state the character-class rule
  its body applies.
- **FR-006**: The corrupt-file and unreadable-file warnings in `src-tauri/src/store.rs`
  MUST be distinguishable, each naming its actual cause.
- **FR-007**: No change in this feature may alter runtime behavior other than the
  wording of the corrupt-file warning banner.

### Key Entities

None. This feature touches no data structure, no persisted field, and no IPC contract.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `cargo test`, `pnpm test`, and `cargo clippy -- -D warnings` are green after
  every story, not just at the end.
- **SC-002**: `grep -rn "opener" package.json src-tauri/Cargo.toml src-tauri/capabilities/ src/ src-tauri/src/`
  returns no matches.
- **SC-003**: `grep -rn 'tauri-app\|tauri_app\|A Tauri App' package.json src-tauri/Cargo.toml src-tauri/src/`
  returns no matches.
- **SC-004**: `src/assets/` contains no unreferenced file.
- **SC-005**: The built app is still named `Site Checker` with identifier
  `com.clintparker.site-checker`, and its behavior is unchanged.
- **SC-006**: `docs/ROADMAP.md` section 1 is empty or removed, with every item either
  done or explicitly re-deferred with a reason.

## Constitution Alignment

- **I. One Mac, One Person** — no scope change; this removes surface rather than adding it.
- **II. Results Are Ephemeral, Config Is Sacred** — `sites.json`'s shape, location, and
  load semantics are untouched. Only the *wording* of the corrupt-file warning changes;
  the corrupt file is still left on disk.
- **III. Be a Polite Client** — no request behavior is touched.
- **IV. Testable Core, Thin Shell** — no logic moves; existing tests are the regression net.
- **V. The Rust/TS Contract Is snake_case, As-Is** — no serialized field name changes.
- **Quality Gates** — all three gates (`cargo test`, `pnpm test`, clippy) are enforced per
  story, and the roadmap is updated rather than silently drained.

## Out of Scope

Sections 2–6 of `docs/ROADMAP.md`: robustness fixes, atomic writes, concurrency
hardening, bundle-size/TLS-backend work, and the test-coverage gaps. Each is tracked
separately and none is a prerequisite for this feature.
