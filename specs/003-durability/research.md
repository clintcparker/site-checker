# Phase 0 Research: Durability & Data Integrity

**Feature**: `specs/003-durability` | **Date**: 2026-08-06

The spec left no `[NEEDS CLARIFICATION]` markers — it deliberately states guarantees and
leaves mechanism to this phase. What follows are the mechanism decisions, each traced to the
requirement it satisfies. Two of them were settled by running the experiment rather than by
recalling how the syscall behaves; those are marked **verified**.

---

## R1 — How the write is made atomic

**Decision**: Write the serialized list to a sibling temp file in the same directory, then
`std::fs::rename` that temp file over `sites.json`. No new crate.

**Rationale**: `rename(2)` within one filesystem is atomic at the VFS layer — a reader either
sees the old inode or the new one, never a mixture, and there is no window in which the path
does not exist. That is exactly FR-001 and it is the standard POSIX idiom. `std::fs::rename`
maps straight onto it, so this costs no dependency and about five lines.

**Alternatives considered**:

- **`tempfile::NamedTempFile::persist`** — the obvious library answer, and rejected for two
  reasons. `tempfile` is a `[dev-dependencies]` entry today; using it in `save` promotes it to
  a runtime dependency for a five-line idiom. More importantly its temp names are randomized,
  so every interrupted save would leave a *differently named* orphan and the artifact count
  would grow with the number of crashes — a direct violation of FR-003 and SC-005.
- **A dedicated crate (`atomicwrites`, `atomic-write-file`)** — same dependency objection,
  same randomized-name objection, and both do more than this app needs.
- **Write in place, then `fsync`** — orders the durability but does nothing about atomicity.
  The truncate-then-write window the roadmap is complaining about is still wide open.
- **Copy `sites.json` to `sites.json.bak` first, then write in place** — this is a backup
  scheme, which the spec puts explicitly out of scope, and it still leaves the live file
  half-written during the crash it is supposed to protect against.

---

## R2 — What the staging artifact is called

**Decision**: A fixed name derived from the real path — `sites.json.tmp`, i.e.
`path.with_extension("json.tmp")` or equivalently the same file name with `.tmp` appended —
sitting in `sites.json`'s own directory.

**Rationale**: Three requirements land on this one choice.

- FR-003 "same directory as `sites.json`" — a rename across a filesystem boundary is not
  atomic and on most platforms simply fails with `EXDEV`. Deriving the temp path from
  `self.path`'s parent makes the boundary impossible to cross by construction, including in
  tests that point the store at a temp dir.
- FR-003 / SC-005 "MUST NOT accumulate" — a *fixed* name means the next save's staging step
  truncates and reuses the orphan rather than adding a second one. The artifact count is
  bounded at one no matter how many crashes precede it. This is the property randomized names
  cannot give.
- FR-003 "MUST NOT be readable as the site list" — `load()` reads exactly the path it was
  handed, which is always `sites.json`. A sibling named `sites.json.tmp` is never opened, is
  never a candidate, and does not shadow the real file.

**Alternatives considered**: a dot-prefixed `.sites.json.tmp` (hides it from Finder, but this
directory is not one users browse, and a visible artifact is a better breadcrumb when
something has gone wrong); a temp file in `std::env::temp_dir()` (crosses the volume boundary
— rejected outright).

---

## R3 — How far durability is pushed

**Decision**: `File::sync_all()` on the temp file before the rename. No `fsync` on the parent
directory.

**Rationale**: The spec's Assumptions scope this feature to *process* death, not media
failure — and `rename` alone already covers process death completely, since the kernel
performs it whether or not our process survives. The temp-file `sync_all` is a one-line,
essentially free upgrade for a file of this size that is written only when a user adds, edits,
or deletes a site: it means the bytes the rename publishes are on the platter and not just in
the page cache. Skipping the directory `fsync` is where the line is drawn — it would mean
opening the parent directory purely to sync it, which buys ordering guarantees only under
power loss, exactly the stronger claim the spec declines to make.

**Consequence to accept**: `sync_all` is a new failure point. It is handled identically to
every other — the `Err` propagates out of `save`, the previous `sites.json` is untouched
because the rename never ran, and `warn_on_write_failure` shows the existing banner. That is
FR-004 satisfied, not violated.

---

## R4 — How an interrupted save gets tested without killing a process

**Decision**: Split the write into two private steps inside `Store` — a staging step that
produces the temp file and returns its path, and `save`, which is the staging step followed by
the rename. Tests in `store.rs`'s own `mod tests` call the staging step directly to reproduce
"interrupted after the contents are staged, before they replace the live file".

**Rationale**: Acceptance scenario 1 needs a save that stops in the middle, and a unit test
cannot half-execute a function. The seam has to exist in the code. Splitting at exactly the
point the atomicity argument turns on — everything before the rename is invisible to a reader,
the rename is the instant of publication — makes the seam the same line as the design, not a
test-only contrivance. `mod tests` is a child module, so it reaches private items without
either step becoming part of the crate's public surface and without a `#[cfg(test)]`-only
method that clippy would have opinions about.

**Alternatives considered**:

- **Spawn a child process running a save and `SIGKILL` it mid-write** — genuinely tests the
  real thing, and rejected: it needs a separate test binary, the timing is a race, and a test
  that is flaky on a busy CI machine is worse than no test. The seam gives a deterministic
  proof of the same property.
- **Inject a failure via a trait or a closure passed into `save`** — more machinery than a
  two-function split, and it would put a generic parameter on `Store` for the sake of one test.

**Test-visible consequence**: the staging test also gets to assert the two facts the edge
cases care about — that `load()` on the live path still returns the previous list *with no
warning*, and that the orphan exists but is not part of the user's data.

---

## R5 — Behaviour when something other than a regular file is at the path (**verified**)

**Decision**: Accept the two behaviours below as they are, and correct the spec's edge-case
sentence rather than write code to force the old behaviour.

Both were confirmed by experiment on this machine (APFS, macOS 25.6) rather than asserted:

| Situation | `fs::write` (today) | `fs::rename` (after this change) |
|---|---|---|
| A **directory** at `sites.json` | fails | fails — `EISDIR`, verified |
| A **symlink** at `sites.json` | follows it, writes the target | **replaces the symlink itself**; the target file keeps its contents, verified |

**Rationale**: the directory case is unchanged — the save fails, is reported through the
existing banner, and nothing is destroyed, exactly as the spec's edge case says. The symlink
case is a real behavioural difference and is called out here rather than buried: after this
change, a symlink at `sites.json` is consumed and replaced by a regular file. Nothing is
destroyed — the symlink's target still holds every byte it held before — but the indirection
is gone. This is inherent to `rename` and cannot be avoided without re-opening the truncation
window this feature exists to close. The app never creates such a symlink; only a user who
hand-linked the file would see it, and they would still have their data.

**Action for implementation**: amend the spec's "A directory or symlink where `sites.json`
should be" edge case to state the two outcomes separately. This is a spec correction, not a
scope change.

---

## R6 — Lowercasing the scheme without reintroducing the trailing slash

**Decision**: Change `has_leading_scheme(&str) -> bool` into a function that returns the
*length* of the leading scheme (`Option<usize>`), and build `candidate` as the lowercased
scheme slice concatenated with the remainder of the user's input verbatim.

**Rationale**: The spec's Clarifications already diagnosed this precisely — `normalize_url`
returns `candidate` (the user's own text) rather than `parsed`'s serialization, specifically so
that `example.com` yields `https://example.com` and not `https://example.com/`. The fix has to
stay inside `candidate`. Returning the scheme's byte offset from the function that already
computes it hands the lowercasing exactly the slice it needs and nothing more, with no
`unwrap` and no second scan of the string. `to_ascii_lowercase` is the correct operation, not
`to_lowercase` — the existing character filter already guarantees the scheme is ASCII, and
URL schemes are ASCII by definition.

The slice boundary is safe without a check: `find("://")` returns a byte index at a character
boundary, and the guard proves every byte before it is ASCII.

**Alternatives considered**:

- **Return `parsed`'s own serialization** — `url::Url` lowercases the scheme for free, which is
  why the bug is invisible to the scheme check. Rejected in the spec itself: it re-adds the
  trailing slash the current code exists to avoid, breaking SC-004.
- **`trimmed.to_lowercase()` on the whole input** — would lowercase paths and query strings,
  which are case-sensitive. Directly contradicts FR-008.
- **Lowercase the host too** — hosts are case-insensitive, so it would be *correct*, and it is
  ruled out by the spec's Assumptions as more rewriting of the user's text than was asked for.
  Not revisited here.

**Preserved by construction**: scenario 4 (`example.com?next=HTTP://x.dev`) still finds its
first `://` inside the query, where the preceding characters include `?` and `=`, fails the
scheme-character guard, and takes the prepend branch — query returned verbatim. Scenario 5
(`FTP://`) lowercases to `ftp` and is then rejected by the unchanged scheme check.

---

## R7 — What a refused duplicate add does to the shell

**Decision**: `Store::add` returns `Err` *before* mutating `self.sites` and before calling
`save`. `commands.rs` is left untouched.

**Rationale**: FR-009 is a statement about the store, and refusing before the push satisfies it
exactly — the in-memory list is unchanged, `save` never runs, so the file is unchanged too.
Leaving the shell alone keeps this feature inside the pure, tested layer, which is what
Constitution IV asks for and what makes all three stories unit-testable.

**The inconsistency this leaves, stated plainly**: `add_site` funnels every `Store::add` error
into `warn_on_write_failure` and then returns `Ok(site)` regardless, so a refusal would surface
as a write-failure banner while the UI still adds a row and the engine still starts a timer for
a site that is not in the store. Distinguishing a refusal from a write failure means giving
`Store` a typed error and teaching the shell to branch on it — real work, in the shell,
for a branch that the shipped app cannot reach, since `add_site` mints a fresh v4 UUID for
every call. **The cost of getting this wrong today is zero and the cost of building it is
not**, so it stays as-is and gets one line in `docs/ROADMAP.md` under section 2
(concurrency/robustness hardening) so it is deferred rather than dropped — the same discipline
the roadmap's own preamble describes.

---

## R8 — Whether `load()` should clean up an orphaned staging file

**Decision**: No. `load()` is unchanged in every respect.

**Rationale**: three reasons converge.

- The non-accumulation requirement is already met by R2's fixed name; cleanup would be
  belt-and-braces on a bound that is structural.
- Deleting on load means mutating the filesystem during startup, which this app deliberately
  does not do — and doing it inside the one function the constitution describes as "loading the
  store never fails" is the worst possible place to introduce a new failure mode.
- FR-005 requires load behaviour be preserved *exactly*, and the existing
  `corrupt_file_yields_an_empty_list_a_warning_and_is_left_on_disk` test is the pin. Touching
  `load` at all puts that at risk for no gain.

**Alternative considered and rejected**: recovering the user's lost edit from the orphan when
it parses as a valid list. That is version history for `sites.json`, which the spec names in
Out of Scope. It would also mean deciding whether an orphan is newer than the live file — a
judgement the app has no basis to make.

---

## R9 — Error message shape

**Decision**: Keep the existing message shape — user-facing, naming `sites.json`, one line.
The new failure points (staging write, `sync_all`, rename) each get their own message but all
follow `Could not …: {e}`.

**Rationale**: FR-004 requires the *existing* write-failure warning, and FR-011 forbids UI
change. The banner is the reporting channel and it renders whatever string `save` returns, so
message shape is the whole of the user-visible contract here. Distinct messages per stage cost
nothing and are the only diagnostic available when a save fails on someone's machine.

---

## Open questions

None. Every FR has a mechanism above, and the one place the implementation diverges from the
spec's prose (R5, symlinks) is recorded with the amendment it requires.
