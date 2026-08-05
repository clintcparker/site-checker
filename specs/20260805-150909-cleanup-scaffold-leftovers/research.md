# Phase 0 Research — v1 Cleanup

The spec carried no `NEEDS CLARIFICATION` markers, but three items had unverified
assumptions behind them ("cosmetic", "unused", "no test pins this"). Each was checked
against the actual repository and the actual installed app rather than assumed. One of the
three turned out to be wrong in a way that changes the plan.

---

## R1 — Does renaming the Cargo package change the shipped product? **Yes.**

**Question**: The roadmap calls the identity rename "Cosmetic — the app identifier and
window title are already correct, so this is not user-visible." Is that true?

**Investigation** — against the existing build and the live machine:

```
src-tauri/target/release/bundle/macos/Site Checker.app/Contents/MacOS/tauri-app
```

```
CFBundleDisplayName => "Site Checker"
CFBundleName        => "Site Checker"
CFBundleIdentifier  => "com.clintparker.site-checker"
CFBundleExecutable  => "tauri-app"          ← derived from Cargo [package] name
```

`productName` and `identifier` come from `tauri.conf.json` and are already correct, as the
roadmap says. But `CFBundleExecutable` is **not** in `tauri.conf.json` — it defaults to
cargo's output binary name, i.e. `[package] name`. Renaming the package to `site-checker`
renames the executable inside the bundle.

That matters because the installed LaunchAgent hardcodes the full path:

```xml
<!-- ~/Library/LaunchAgents/Site Checker.plist -->
<key>ProgramArguments</key>
<array><string>/Applications/Site Checker.app/Contents/MacOS/tauri-app</string></array>
```

And `~/Library/Application Support/com.clintparker.site-checker/autostart.initialized`
already exists, so `lib.rs`'s first-run block will **not** re-register the login item. The
failure mode is therefore: rename → reinstall → launch-at-login silently stops working →
the checkbox still reads "on" → nothing self-heals. That is an observable behavior change,
which FR-010 forbids and which FR-004 names explicitly.

**Decision**: Rename `[package] name` to `site-checker` **and** add
`"mainBinaryName": "tauri-app"` to `tauri.conf.json` in the same commit.

**Rationale**: This is precisely what the spec's edge case prescribes — "If a rename would
change any of those, the shipped values must be pinned explicitly so they stay put." The
key is supported by the pinned toolchain: `tauri-utils 2.9.3` defines
`main_binary_name: Option<String>`, documented as "Overrides app's main binary filename…
we will rename that binary in `tauri-cli`'s `tauri build` command, and target
`tauri bundle` to it." It applies at build/bundle time only, so `tauri dev` is unaffected.

**Alternatives considered**:

- *Rename and accept the new executable name.* Cleanest metadata, but breaks launch-at-login
  for the one person who runs this, with no self-repair. Rejected on FR-004/FR-010.
- *Rename and also fix the LaunchAgent.* Out of scope — touching the login item is a
  behavior change, and the spec's Out of Scope section is explicit that unrelated work gets
  appended to the roadmap rather than fixed in place.
- *Leave `[package] name` alone; fix only description/authors/`[lib] name`.* Zero risk, but
  leaves the most visible placeholder in place and fails FR-003/SC-002.
- *Set `[[bin]] name = "tauri-app"` instead of `mainBinaryName`.* Equivalent effect (the
  Tauri docs suggest it as the alternative) but expresses the pin in cargo rather than in
  the Tauri config where the other shipped-identity values already live. Rejected for
  cohesion, not correctness.

> **⚠️ Flag for the user — the one judgment call in this feature.** Pinning
> `mainBinaryName` means the literal string `tauri-app` survives, in
> `tauri.conf.json`, after a feature whose SC-002 says "zero scaffold placeholder identity
> values remain." The pin is a deliberate compatibility anchor with a documented reason,
> not a missed placeholder — but it *is* the same string, and reasonable people would call
> that a partial miss.
>
> The alternative is one sentence of manual work: accept `CFBundleExecutable =
> site-checker`, and after installing the new build, untick and re-tick **Launch at login**
> once to rewrite the plist. If you'd rather have genuinely zero `tauri-app` strings and
> don't mind that one-time toggle, say so and the pin comes out. **Planned as: keep the
> pin** (it is what the spec as written asks for).

---

## R2 — Is the internal library rename safe?

**Question**: Spec Assumption says `[lib] name = "tauri_app_lib"` is renamed too, "if
renaming it proves to carry any risk to the produced bundle." Does it?

**Investigation**: `tauri_app_lib` is referenced from exactly one place —
`src-tauri/src/main.rs:5`, `tauri_app_lib::run()`. The `[lib]` target's `staticlib`/
`cdylib`/`rlib` outputs land in `target/` and are not copied into the `.app` bundle; only
the bin target is. The lib name exists to avoid a bin/lib collision on Windows (per the
comment already in `Cargo.toml`), which `site_checker_lib` satisfies equally.

**Decision**: Rename to `site_checker_lib` and update the single call site in `main.rs`.
No risk to the bundle; the assumption's escape hatch is not needed.

**Alternatives considered**: Leave it — rejected, it is the same placeholder and a
half-finished rename is worse than either end state.

---

## R3 — Is the opener plugin genuinely dead, and where does it live?

**Question**: The roadmap names two locations. Are there more, and is the plugin truly
unreferenced?

**Investigation**: A repo-wide grep (excluding `node_modules` and `target`) returns exactly
three declaration sites plus two lockfile records:

| Location | Line |
|---|---|
| `src-tauri/capabilities/default.json` | `"opener:default"` |
| `src-tauri/Cargo.toml` | `tauri-plugin-opener = "2"` |
| `package.json` | `"@tauri-apps/plugin-opener": "^2"` |
| `src-tauri/Cargo.lock` | resolved `tauri-plugin-opener` entry |
| `pnpm-lock.yaml` | resolved `@tauri-apps/plugin-opener@2.5.4` |

`lib.rs` registers only `tauri_plugin_autostart` — there is no `.plugin(tauri_plugin_opener::init())`
call. No TypeScript file imports `@tauri-apps/plugin-opener`. The plugin is dead in both
halves, confirming the spec's assumption that the backend declaration is in scope too.

**Decision**: Remove all three declarations; regenerate both lockfiles. Remove the
capability entry and the Cargo dependency as **one atomic edit** — `tauri-build` resolves
capability permissions against installed plugins at compile time, so a state with
`"opener:default"` granted but the crate absent fails to build.

**Alternatives considered**: Removing only the two locations the roadmap names — rejected
by the spec's own assumption ("the intent governs"); it would leave the crate compiling
into the binary.

---

## R4 — What wording distinguishes the two store warnings, and what does it break?

**Question**: How should the messages read, and does any existing test pin the current text?

**Investigation**: `store.rs` currently emits:

- I/O error: `"Could not read sites.json ({e}). Starting empty."`
- Parse error: `"sites.json could not be read ({e}). Starting with an empty list; the existing file has been left alone."`

Both hinge on "could not be read" — the exact collision the roadmap flagged. The existing
test `corrupt_file_yields_an_empty_list_a_warning_and_is_left_on_disk` asserts only
`warning.is_some()`, so **no test pins the current wording** and none will break.

**Decision**: Reword so the two differ from their first word onward, and add a test that
pins the distinction. Exact strings are the contract in
[contracts/warning-messages.md](./contracts/warning-messages.md).

- Open failure: `Could not open sites.json ({e}). Starting with an empty list.`
- Parse failure: `sites.json is damaged and could not be understood ({e}). Starting with an empty list; the existing file has been left alone so you can recover it.`

The parse message keeps the "left alone" promise (FR-007, Principle II). Neither message
shares an opening phrase (FR-006).

**On the new test**: SC-006 requires the test count to hold or rise, and an unpinned
distinction is exactly the kind of thing a later edit re-collides by accident. The test
needs a non-`NotFound` I/O error, which is awkward to produce portably — `chmod 000`
depends on not running as root. Pointing `load()` at a **directory** yields a read error
that is neither `NotFound` nor permission-dependent, and works without special privileges.

**Alternatives considered**:

- *Reword only the I/O message.* Sufficient for FR-006 in the narrow sense, but leaves the
  corrupt-file message opening with a phrase about reading, which is the ambiguity being
  fixed. Rejected.
- *Add an error enum instead of message text.* Would satisfy the distinction structurally,
  but the warning crosses the IPC boundary as a `String` (Principle V) and changing that is
  a contract change well beyond a cosmetic-cleanup feature. Rejected as scope creep.
- *No new test.* Rejected — SC-006, and the collision would silently return.

---

## R5 — What does `has_leading_scheme` actually accept?

**Question**: What rule must the doc comment state?

**Investigation**: `src-tauri/src/model.rs:48-55`. Given `s.find("://")`: `None` → false;
`Some(0)` → false (a `://` at position zero is not a scheme); otherwise every char before
the separator must satisfy `is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')`.

**Decision**: Extend the existing comment to state both halves — the ASCII
alphanumeric/`+`/`-`/`.` character class, and that a separator at position 0 does not count.
Keep the existing sentence explaining *why* the naive `contains("://")` was rejected; the
gap is *what*, not *why*. Doc-comment-only; no code change, so no test change.

**Alternatives considered**: Restating the rule in prose without naming the characters —
rejected, FR-009 requires the character rule and SC-004's sibling test ("predict correctly
from the note alone") depends on it.

---

## R6 — How is "observably identical" verified without a UI end-to-end harness?

**Question**: SC-005 wants a clean-build comparison and SC-007 wants a functional pass, but
the project deliberately excludes UI end-to-end tests (Principle IV).

**Investigation**: The pre-change bundle already exists on disk, so its identity values can
be recorded as a baseline before touching anything. Two build-environment hazards apply:

- `cargo clean` is required — the spec's edge case is right that an incremental build can
  resolve a stale `tauri_app_lib` from cache and hide a broken rename.
- `pnpm tauri build`'s DMG step calls `osascript` and hangs without an interactive session
  (roadmap §5). `--bundles app` produces the `.app` — which is all the identity check
  needs — and skips the DMG entirely.

**Decision**: Verify in three tiers, per [quickstart.md](./quickstart.md): (1) automated
gate after every item; (2) a one-time clean `--bundles app` build compared against the
recorded baseline via `plutil -p`; (3) a manual launch exercising add/edit/status/delete
and the autostart toggle. This matches the spec's assumption that behavior is verified "by
the existing automated suite plus a manual launch."

**Alternatives considered**: Adding a UI end-to-end harness — rejected, contradicts
Principle IV and the spec's own assumption, for a feature that changes no UI logic.

---

## R7 — Are the three SVGs really unreferenced?

**Question**: The roadmap says Vite already drops them; confirm nothing references them.

**Investigation**: Grep across `*.ts`, `*.html`, `*.css`, `*.json` for `tauri.svg`,
`typescript.svg`, `vite.svg`, and `assets/` returns **zero** matches outside
`node_modules`. `src/assets/` contains only those three files.

**Decision**: Delete all three, then remove the empty `src/assets/` directory.

**Alternatives considered**: Keeping the directory with a `.gitkeep` — rejected, nothing
is planned to go in it and an empty placeholder is the same kind of scaffold residue this
feature removes.
