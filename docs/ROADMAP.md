# Site Checker — Roadmap & Deferred Work

Everything here is **deferred, not broken.** v1 shipped merge-ready: the automated
suite is green (Rust 29, frontend 12), `cargo clippy -- -D warnings` is clean, and
the final whole-branch review found no Critical or Important issues. This file
collects the Minor findings, hardening ideas, and out-of-scope features surfaced
during implementation and review, so none of them get silently lost.

Each item notes rough **effort** (S / M / L) and where it lives. Nothing here
blocks use of v1.

---

## 1. Cleanup (safe, cosmetic, do anytime)

- **Remove the unused opener plugin.** `src-tauri/capabilities/default.json` still
  grants `"opener:default"` and `package.json` still depends on
  `@tauri-apps/plugin-opener`, but the plugin is never registered in `lib.rs` and
  the spec never uses it. Scaffold leftover. — S
- **Delete orphaned scaffold assets.** `src/assets/{tauri,typescript,vite}.svg`
  are no longer referenced by any `<img>` (Vite already drops them from the
  bundle; this is tidiness only). — S
- **Fix scaffold identity metadata.** `package.json` `"name"` is still
  `"tauri-app"`; `src-tauri/Cargo.toml` still has `name = "tauri-app"`,
  `description = "A Tauri App"`, `authors = ["you"]`. Cosmetic — the app
  identifier (`com.clintparker.site-checker`) and window title (`Site Checker`)
  are already correct, so this is not user-visible. — S
- **`has_leading_scheme` doc comment** (`src-tauri/src/model.rs`) doesn't mention
  the character-class rule it applies (alphanumeric, `+`, `-`, `.`). — S
- **Two near-identical store error strings** (`src-tauri/src/store.rs`): the
  "could not read" I/O-error message and the corrupt-file message read almost
  the same, so a corrupt file can look like a permissions problem to the user. — S

## 2. Robustness (small correctness wins)

- **Register event listeners before the awaited startup IPC calls.**
  `src/main.ts` attaches `onSiteStatus` / `onStoreWarning` only *after*
  `await mountAutostart()` and `await getWarning()`. A `site-status` event
  emitted during that window is dropped (Tauri events have no replay). Rare in
  practice — the first check is delayed 0–10 s by jitter while the IPC calls take
  milliseconds, and a dropped event self-heals on the next interval — but moving
  the listener registration to the top of `main()` closes it. *This is the one
  item the final review flagged as a real, if rare, robustness bug.* — S
- **Reset a row to Pending when its URL is edited.** `upsertSite` in
  `src/main.ts` updates the site but not the `statuses` map, so changing a URL
  from a good one to a bad one keeps showing the old dot until the next check
  lands. Dropping the stale status on a URL change would make the "last
  confirmed" claim honest immediately. — S
- **Guard against double-submit / double-delete.** The add/edit form and the
  per-row Delete have no in-flight guard (`src/form.ts`); a fast double-click can
  create two identical sites or fire two deletes. Low impact for a single-user
  tool; disabling the submit button during the awaited call would close it. — S
- **Upper bound on the interval field.** `index.html` / `src/form.ts` have no
  `max`; a pasted very large number could exceed `u64` at the IPC boundary.
  Today this fails gracefully (the command rejects, `form.ts`'s catch shows the
  error inline — no crash), but a `max` attribute would prevent it at the
  source. — S
- **Null-guard the `#autostart` lookup.** `src/main.ts`'s
  `querySelector("#autostart")!` is a non-null assertion; if the element ever
  went missing, the `catch` block itself dereferences the checkbox and would
  throw, halting the rest of `main()`. Latent only — the element is static in
  `index.html`. — S

## 3. Durability & data integrity

- **Atomic writes for `sites.json`.** `Store::save` (`src-tauri/src/store.rs`)
  uses a plain `fs::write`, not write-to-temp-then-rename. A crash mid-write can
  truncate the file; the next launch treats that as corrupt and shows the banner
  (graceful — not silent data loss), but the last edit is gone. Write-temp +
  atomic rename would make saves crash-safe. *Highest-value item in this
  section.* — M
- **Lowercase the URL scheme in `normalize_url`** (`src-tauri/src/model.rs`):
  `HTTPS://example.com` currently persists verbatim rather than normalizing to
  `https://`. Cosmetic — the URL still works. — S
- **Dedupe ids in `Store::add`.** No check today; unreachable in practice because
  ids are v4 UUIDs generated in `add_site`. Belt-and-braces only. — S

## 4. Concurrency hardening

Judged against a single-user desktop tool, none of these are live bugs today.

- **Mutex-poison recovery.** `engine.rs` and `commands.rs` use
  `Mutex::lock().unwrap()` throughout; a panic while holding a lock would poison
  it and cascade panics to every later command. No critical section under either
  lock realistically panics (no indexing, no `unwrap` on fallible ops inside
  `save`/`get`/`update`), so this is latent. `PoisonError::into_inner()` recovery
  would harden it. — M
- **`update_site` read-modify-write race.** `src-tauri/src/commands.rs` takes the
  store lock twice — once to read `existing.method_override`, once to write the
  new `Site`. Two concurrent updates for the same id could each decide from the
  same stale snapshot, and the second write wins, discarding the first's fields.
  Reachable only by overlapping saves on one row (a fast double-submit). Closing
  it means one lock scope for read-decide-write. — M

## 5. Build & packaging

- **Bundle size: 15 MB `.app` vs the spec's "single-digit MB" expectation.**
  Attributable to rustls pulling in the `aws-lc-rs` crypto backend, which
  compiles native C. Either accept it and amend the spec's size line, or switch
  reqwest's TLS feature to the `ring` backend to shrink the bundle. This is a
  spec-expectation mismatch, not a code defect. — M
- **DMG build step hangs in a headless environment.** `pnpm tauri build`'s
  `bundle_dmg.sh` calls `osascript` to lay out the DMG Finder window, which
  blocks on GUI-automation permission when no interactive session is present.
  The compile, `.app`, and `.dmg` all complete regardless; only the cosmetic
  re-layout hangs. Relevant to any future CI: run the build where Finder
  automation is approved, or skip the DMG target in CI. — S

## 6. Test-coverage gaps

All of these paths are correct by inspection today; the note is only that no
automated test pins them.

- `check.rs`: no test for `used_get_fallback == true` when the GET retry itself
  fails at the **transport** layer (only the "GET returns 500" case is covered);
  no test for a 405 → GET → 405 loop or a > 10-hop redirect chain.
- `store.rs`: no test for `update`'s no-op branch (a missing id still saves and
  returns `Ok`).
- `render.ts`: no test for `renderTable(tbody, [], now)` against a populated
  tbody (delete-the-last-site), nor for front-insertion.
- `form.ts`: no unit tests for its branch/clamp logic (consistent with the
  spec's exclusion of UI end-to-end tests; a focused DOM unit test like the one
  added for `render.ts` could cover the clamp and Add/Edit dispatch).

## 7. Out of scope for v1 (feature roadmap)

Recorded in the plan's "Deferred to v2" and reaffirmed here — each is a
deliberate v1 exclusion, not an oversight:

- Desktop notifications on up→down / down→up transitions
- Status history, sparklines, uptime percentages
- A manual "check now" button
- A menu-bar icon
- Continuing to run after the window is closed (v1 quits on window close)
- Per-URL expected-status configuration
- Auth headers / private endpoints

---

## Environment note (not a code item)

The Rust toolchain fix from Task 1 edited `~/.config/fish/config.fish`, whose
real file lives in the `~/.dotfiles` repo. That change is **uncommitted in the
dotfiles repo** — commit it there so it isn't lost on a later `stash`/`pull`/`reset`.
