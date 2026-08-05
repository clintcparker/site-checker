# Contract — Store Warning Messages

The user-facing text surfaced in the startup banner when `sites.json` cannot be loaded.
This is a contract because FR-006 constrains the *relationship between* the two strings,
not just each one alone — a later well-meaning edit to one of them can break the pair.

Producer: `store::load` → `LoadOutcome.warning: Option<String>`
Transport: `get_warning` IPC command (startup) and the `store-warning` event (later failures)
Consumer: `showBanner` in `src/main.ts` — renders the string verbatim, no parsing

---

## Message A — the file could not be opened

**Branch**: `std::fs::read_to_string` returns `Err` with a kind other than `NotFound`
(permissions, is-a-directory, hardware, …).

**Before**

```
Could not read sites.json ({e}). Starting empty.
```

**After**

```
Could not open sites.json ({e}). Starting with an empty list.
```

Requirements:

- MUST indicate the file could not be **opened / accessed**.
- MUST NOT describe the contents as damaged, corrupt, or unreadable-as-data — the contents
  were never seen (FR-006).
- MUST NOT claim the file was left alone. Nothing was written, but saying so here invites
  the reader to look for a recovery step that does not apply to a permissions problem.
- MUST include the underlying OS error `{e}` — it is what distinguishes "denied" from
  "disk gone".

## Message B — the file could not be understood

**Branch**: the file was read successfully but `serde_json::from_str::<Vec<Site>>` fails.

**Before**

```
sites.json could not be read ({e}). Starting with an empty list; the existing file has been left alone.
```

**After**

```
sites.json is damaged and could not be understood ({e}). Starting with an empty list; the existing file has been left alone so you can recover it.
```

Requirements:

- MUST indicate the **contents** could not be understood (FR-006).
- MUST state the existing file has been left in place (FR-007, Principle II). This is a
  real guarantee — nothing writes until the user's next save — and it is the sentence that
  tells the reader recovery by hand is possible.
- MUST include the parse error `{e}`.

---

## The pairwise property

The point of the change. Both must hold:

1. **No shared opening phrase.** Message A begins `Could not open sites.json`; message B
   begins `sites.json is damaged`. They diverge at the first word.
2. **No cross-contamination.** A does not say "damaged"/"corrupt"/"understood"; B does not
   say "could not be opened".

Restated as the acceptance bar (SC-004): shown only these two strings, a reader who has
never seen the code picks which one means "the file is damaged" and which means "the file
could not be opened" — correctly, both times.

## Test obligation

No existing test pins either string (the current corrupt-file test asserts only
`warning.is_some()`), so nothing breaks — but the distinction must not be left unpinned.
Add a test in `store.rs`'s `mod tests` that:

- produces the open-failure branch by calling `load` on a path that is a **directory** —
  a read error that is neither `NotFound` nor dependent on running as a non-root user, so
  it works in any environment;
- produces the parse-failure branch with invalid JSON, as the existing test does;
- asserts both warnings are `Some`, and asserts the pairwise property above rather than
  the full literal text — pin the distinction, not the prose, so wording can still be
  improved without a test edit.

Per SC-006 this raises the test count; no test is deleted or weakened.
