# Phase 1 Data Model: Packaging & Distribution

This feature stores no data and changes no persisted format. "Entities" here are the artifacts
and identifiers a release moves between, and the rules that govern them. The application's own
data model — `Site`, `StatusEvent`, `sites.json` — is untouched by this feature and is not
restated.

Field-level formats live in [contracts/](./contracts/); this file records what each entity *is*,
what owns it, and what must be true of it.

---

## Entities

### Version Tag

The single source of truth for a release's version.

| Attribute | Value |
|---|---|
| Form | `v<MAJOR>.<MINOR>.<PATCH>` — e.g. `v1.0.0` |
| Kind | Annotated git tag |
| Location | `clintcparker/site-checker`, on a commit reachable from `main` |
| Created by | The maintainer, by hand. The only manual step in a release. |
| Derived value | `version = tag without the leading "v"` (`v1.2.3` → `1.2.3`) |

**Rules**

- Pushing one is the *entire* release procedure (FR-007).
- A tag not matching the form triggers nothing (FR-009). The workflow filter is `v*`; the
  version-derivation step rejects anything that is not `v` followed by three dot-separated
  numeric components, so a stray `v-old` or `vnext` fails in seconds rather than publishing a
  release named after it.
- The first tag is `v1.0.0` — v1 has shipped, and `0.1.0` was never anything but scaffold
  default.

---

### Build Version Sentinel

The inert placeholder that occupies every version field in the tree, so no committed file carries
a number a human maintains (FR-008).

| Attribute | Value |
|---|---|
| Value | `0.0.0` |
| Locations | `src-tauri/Cargo.toml` `[package].version`, `package.json` `version` |
| Not a location | `src-tauri/tauri.conf.json` — the `version` key is **deleted**, so Tauri falls back to `Cargo.toml` |

**Rules**

- `release.yml` overwrites `Cargo.toml`'s sentinel with the tag-derived version **before**
  building. The edit lives only in the runner's working copy and is never committed.
- `ci.yml` asserts both sentinels are still `0.0.0` on every PR and push. A hand-bump is a red
  check, not a silent second source of truth.
- `package.json`'s version never reaches the bundle; it is stamped for consistency and guarded so
  it cannot drift back into looking authoritative.

**State**

```text
committed: 0.0.0  ──(release.yml, runner only)──▶  1.2.3  ──▶  baked into Site Checker.app
     ▲                                                             │
     └──────────── never written back; CI asserts 0.0.0 ───────────┘
```

---

### Release Artifact

One installable bundle per supported architecture.

| Attribute | Value |
|---|---|
| Name | `site-checker-<arch>-apple-darwin.zip`, `<arch>` ∈ `aarch64`, `x86_64` |
| Contents | Exactly one top-level entry: `Site Checker.app` |
| Produced by | `pnpm tauri build --bundles app --target <arch>-apple-darwin`, then `ditto -c -k --sequesterRsrc --keepParent` |
| Signed | Developer ID Application, hardened runtime |
| Notarized | Yes, ticket stapled to the `.app` **before** zipping |
| Cardinality | Exactly 2 per release (FR-005, SC-004) |

**Rules**

- Never a `.tar.gz` — tar mangles a signed bundle's extended attributes (research R3).
- Never a `.dmg` — producing one hangs headless (FR-016, research R4).
- The stapling order matters: staple the `.app`, then zip. A ticket stapled to a zip does not
  survive extraction.

---

### Release

The published record of one version.

| Attribute | Value |
|---|---|
| Identity | The version tag |
| Holds | Both release artifacts + generated release notes |
| Created by | `softprops/action-gh-release@v2` with `generate_release_notes: true` |
| Visibility | Public — required for the install channel to resolve (research R1) |

**Rules**

- Re-running for an existing tag **updates in place**; it never creates a second release for the
  same version (FR-014, SC-007).
- It is the authority for checksums: the `homebrew` job downloads the assets *from the published
  release* and hashes those bytes, rather than hashing what the build job happened to leave on
  disk (FR-011).

---

### Install-Channel Template

The canonical, human-edited description of the cask.

| Attribute | Value |
|---|---|
| Path | `install/homebrew/site-checker.rb` (this repository) |
| Header | "TEMPLATE — do not copy to the tap by hand", naming the workflow that renders it |
| Placeholders | `VERSION`, `SHA256_ARM64`, `SHA256_X86_64` |
| Edited by | Humans. This is the only file in the pair anyone edits. |

**Rules**

- Rendering fails the release if a placeholder the renderer expects is **missing here** (template
  drifted ahead of the workflow) — FR-013, first direction.
- Rendering fails the release if any placeholder **survives** into the output (workflow drifted
  ahead of the template) — FR-013, second direction.

---

### Install-Channel Entry

The rendered result, and the thing `brew` actually reads.

| Attribute | Value |
|---|---|
| Path | `Casks/site-checker.rb` in `clintcparker/homebrew-tap` |
| Written by | `release.yml`'s `homebrew` job only. Never by hand. |
| Contains | No placeholders; a literal version and two literal checksums |

**Rules**

- Committed only when its content differs from what the tap already holds (FR-014).
- Its `version "<L>"` line is what `verify-install-channels.yml` greps to detect a lagging
  channel (FR-020).
- A cask, not a formula (FR-002) — but `brew install clintcparker/tap/site-checker` resolves
  against both, so the advertised command is unchanged.

---

### One-Time Setup Credentials

Named secrets on `clintcparker/site-checker`. Present-or-absent is all pre-flight can see
(research R9).

| Secret | Purpose | Scope |
|---|---|---|
| `TAP_PUSH_TOKEN` | Push `Casks/site-checker.rb` to the tap | Fine-grained PAT, Contents:RW, **`clintcparker/homebrew-tap` only** (FR-015) |
| `APPLE_CERTIFICATE` | Developer ID Application `.p12`, base64 | Signing |
| `APPLE_CERTIFICATE_PASSWORD` | `.p12` password | Signing |
| `APPLE_SIGNING_IDENTITY` | Certificate common name | Signing |
| `APPLE_API_ISSUER` | App Store Connect issuer ID | Notarization |
| `APPLE_API_KEY_ID` | App Store Connect key ID | Notarization |
| `APPLE_API_KEY` | App Store Connect `.p8`, base64 | Notarization |

Each is one-time. None recurs per release. All are conditional on research R2's answer except
`TAP_PUSH_TOKEN`.

---

### Stored Data Directory

`~/Library/Application Support/com.clintparker.site-checker`, holding `sites.json`.

Relevant here **only** because uninstall must be able to remove it deliberately and must never
remove it accidentally.

| Operation | Effect on this directory |
|---|---|
| `brew install` | none |
| `brew upgrade` | none |
| `brew uninstall` | **none** (FR-004) |
| `brew uninstall --zap` | moved to Trash (FR-003, SC-010) |

This is the only new mechanism in the project's history that can delete the user's site list, and
it is opt-in. Constitution II is satisfied by the `zap`/`uninstall` split, not by discipline.

---

## The Release State Machine

```text
  push tag v1.2.3
        │
        ▼
  ┌───────────┐   missing secret   ┌──────────────────────────────┐
  │ preflight │──────────────────▶ │ FAIL (<60s, names the secret)│
  └───────────┘                    │ nothing built, nothing pushed│
        │ ok                       └──────────────────────────────┘
        ▼
  ┌───────────┐  red   ┌──────┐
  │   test    │───────▶│ FAIL │
  └───────────┘        └──────┘
        │ green
        ▼
  ┌──────────────────────────────┐
  │ build  (matrix: arm64, x64)  │  stamp Cargo.toml → build --bundles app
  │                              │  → sign → notarize → staple → ditto zip
  └──────────────────────────────┘
        │ 2 artifacts
        ▼
  ┌───────────┐   creates or UPDATES the release for v1.2.3
  │  release  │   + generated notes
  └───────────┘
        │
        ▼
  ┌───────────┐   download assets FROM the release → hash → render template
  │ homebrew  │   → placeholder guards (both directions) → commit iff changed
  └───────────┘
        │
        ▼
  ┌──────────────────────────┐
  │ verify-install-channels  │  (workflow_run) assets + tap version
  │                          │  + brew install + spctl -a -t exec
  └──────────────────────────┘
```

**Invariants**

1. Nothing is published before `preflight` passes (FR-018).
2. Nothing reaches the tap before the release exists — the `homebrew` job `needs: release`, and
   reads the release rather than the build (FR-011).
3. Every terminal state is a definite pass or fail. No step waits for a human or a GUI (SC-009).
4. Re-entering at the top with the same tag converges rather than duplicating (FR-014).
</content>
