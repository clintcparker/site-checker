# Quickstart: validating clickable URLs

How to prove this feature works end-to-end. Automated checks first — they cover
most of it and cost nothing. The manual pass covers only what cannot be
automated: that a real browser actually comes forward.

Shapes and rules referenced below live in
[contracts/open-url-command.md](./contracts/open-url-command.md),
[contracts/row-url-element.md](./contracts/row-url-element.md), and
[data-model.md](./data-model.md). They are not repeated here.

## Prerequisites

- macOS, with a default browser configured
- Rust toolchain, `pnpm` 10.30.3 (`packageManager` pins it)
- `pnpm install` has been run

## 1. Automated: the merge bar

These are the constitution's Quality Gates. All three must be green.

```bash
cd src-tauri && cargo test && cargo clippy -- -D warnings
cd .. && pnpm test
```

### What `cargo test` must prove

`openable_url`, in `src-tauri/src/model.rs` — pure, no `AppHandle`, no
filesystem. Every row of the table in
[contracts/open-url-command.md](./contracts/open-url-command.md#guard-openable_url)
is a test case. The two that matter most:

- **It returns the input byte-identical.** Not the `url::Url` re-serialization —
  `https://example.com` must not come back as `https://example.com/` (FR-006).
  This is the same trap `normalize_url` documents at length.
- **It refuses `example.com`.** A scheme-less string must be rejected, not
  repaired. If this test passes while `openable_url` delegates to
  `normalize_url`, the guard is wrong — see [research.md](./research.md) §4.

> No test may invoke the `open_url` command. `cargo test` must never launch a
> browser. The command body is thin shell by design (Constitution IV), the same
> allowance `engine.rs` takes.

### What `pnpm test` must prove

- `render.test.ts` — an unlabelled site renders its URL as a `<button>` carrying
  `data-open-url`; a labelled site renders the label as an inert `<span>` and
  the URL as the button; a non-http/https URL renders as a `<span>` with no
  `data-open-url` at all; the button's `data-open-url` is the full URL even when
  the text is long. Plus the reconciliation cases in the existing style:
  **element identity is preserved across a repaint with only `now` advanced**,
  and preserved when a status event lands.
- `open.test.ts` — the pure `shouldOpen` ledger rule (accept, suppress inside
  the window, accept again after it, per-URL independence, and that a
  *suppressed* activation does not extend the window). Then the delegated
  listener over a fixture `tbody`: a click on the button calls `openUrl` with
  the attribute's value; a click on a labelled row's label calls nothing; a
  rejected `openUrl` produces a banner message.
- `main.test.ts` — an open failure reaches `showBanner`, and the table is still
  rendering afterwards.

Follow the existing mocking convention: `vi.mock("./api", …)`, as `form.test.ts`
and `main.test.ts` already do. `open.ts` needs no Tauri backend behind it.

## 2. Manual: a real browser, a real click

Only US1, US2, and SC-003 need this. Everything else is covered above.

> **Back up your site list first.** The app has no config-directory override —
> a dev run reads and writes your real
> `~/Library/Application Support/com.clintparker.site-checker/sites.json`.
> Copy it somewhere outside the repo before starting, and confirm no earlier
> `pnpm tauri dev` is still running.

```bash
cp ~/Library/Application\ Support/com.clintparker.site-checker/sites.json /tmp/sites.json.bak
pnpm tauri dev
```

### US1 — Visit a site from its row (P1)

1. Add `https://example.com` with no label. Click its URL in the table.
   → The default browser comes forward on that address, **within two seconds**
   (SC-003). The dashboard window still shows the table, still ticking (FR-003,
   SC-005).
2. Edit the site to add the label `Example`. Click the URL beneath the label.
   → Same result. Click the label itself → nothing happens (FR-008).
3. Hover the URL without clicking.
   → Underline and pointer cursor. It reads as openable before any click (FR-004).

### US2 — Open a site without a mouse (P2)

1. Tab into the table until the URL takes focus.
   → A visible focus ring, and the URL is reached before that row's Edit button.
2. Press Enter. → The browser opens the same address (SC-004).
3. Tab back to the URL and hold focus through a status arriving and a few age
   ticks. → Focus stays on it (FR-011).

### US3 — A refusal explains itself (P3)

The honest failure — no handler for `http` — cannot be produced without changing
your system's default browser, which is not worth doing. Validate the path
instead:

1. **Automated** (`open.test.ts`): a rejected `openUrl` puts the backend's
   message in the banner and leaves the table usable (FR-009, SC-006).
2. **Manual**, for the guard: quit the app, and with a text editor add an entry
   to `sites.json` by hand with `"url": "ftp://example.com"` — keep the same
   snake_case shape as its neighbours. Relaunch.
   → The row appears, its URL is **plain text**: no underline, no pointer
   cursor, not reachable by Tab, and clicking it does nothing (FR-007, SC-002).
   It is not hidden and not flagged as invalid — spec Open Decision 3.
3. Restore your list: `cp /tmp/sites.json.bak ~/Library/Application\ Support/com.clintparker.site-checker/sites.json`

### FR-012 — rapid repeats

Double-click a URL fast. → One browser navigation, not two. Wait a couple of
seconds and click again → it opens again.

## 3. Nothing else changed

Before calling it done, confirm the feature stayed in its lane (FR-010,
Constitution II):

- `sites.json` is byte-identical to what it was before you clicked anything —
  no field added, no timestamp written.
- Adding, editing, and deleting still work; the status column keeps updating and
  the "last checked" ages keep counting.
- `git diff` touches no dependency manifest: not `Cargo.toml`, not
  `package.json`, not `Cargo.lock`, not `pnpm-lock.yaml`, and not
  `src-tauri/capabilities/default.json`. A change in any of them means the
  plugin route crept back in — see [research.md](./research.md) §1.
