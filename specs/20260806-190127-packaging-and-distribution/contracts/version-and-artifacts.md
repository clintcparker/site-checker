# Contract: Version & Release Artifacts

The interface between a pushed tag and the bytes a user downloads. Anything that consumes a Site
Checker release — the cask, the verification workflow, a human with `curl` — depends on this.

## Tag → version

| Input | Output | Behaviour |
|---|---|---|
| `v1.0.0` | `1.0.0` | Release proceeds |
| `v1.2.3` | `1.2.3` | Release proceeds |
| `v10.0.1` | `10.0.1` | Release proceeds |
| `v1.2` | — | Rejected: not three components |
| `v1.2.3-rc.1` | — | Rejected (see note) |
| `vnext`, `v-old`, `1.0.0` | — | Rejected / not triggered |

Derivation is `VERSION="${GITHUB_REF_NAME#v}"`, followed by a validation gate:

```sh
case "$VERSION" in
  *[!0-9.]*|*..*|.*|*.) bad=1 ;;
esac
# plus: exactly three dot-separated, non-empty, numeric components
```

A tag that fails this gate exits non-zero from `preflight` — before any build, release, or tap
write (FR-009, FR-018).

**Note on prereleases.** `name-on`'s how-to permits `v1.3.0-rc.1`. This contract does **not**:
a prerelease would be rendered into the tap as the current version and served to every user by
`brew install`, because a cask has no channel concept. If prereleases are wanted later, they need
either a separate cask name or an explicit `prerelease: true` release that the `homebrew` job
skips — a deliberate design decision, not something to allow by accident.

## Version → build

| File | Committed value | Value at build time | Enforced by |
|---|---|---|---|
| `src-tauri/Cargo.toml` `[package].version` | `0.0.0` | tag-derived | `release.yml` stamps it; `ci.yml` asserts the sentinel |
| `package.json` `version` | `0.0.0` | `0.0.0` (unused by the bundle) | `ci.yml` asserts the sentinel |
| `src-tauri/tauri.conf.json` `version` | **absent** | n/a — Tauri falls back to `Cargo.toml` | The key does not exist |

Guarantee: the version reported by the installed application equals the tag that produced it
(FR-008, US2 acceptance 3). Verifiable after install with:

```sh
/usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" \
  "/Applications/Site Checker.app/Contents/Info.plist"
```

The release build must not pass `--locked` (stamping desynchronises `Cargo.lock`); `ci.yml`,
which does not stamp, must.

## Artifact names

```text
site-checker-aarch64-apple-darwin.zip     Apple Silicon
site-checker-x86_64-apple-darwin.zip      Intel
```

Stable across versions — the version lives in the download **path**, not the filename, matching
`name-on`'s convention:

```text
https://github.com/clintcparker/site-checker/releases/download/v<VERSION>/site-checker-<arch>-apple-darwin.zip
```

`<arch>` is the Rust target-triple prefix, so the same token names the `--target`, the asset, and
the cask's `arch` stanza value. There is no mapping table anywhere.

## Artifact format

| Property | Requirement |
|---|---|
| Container | ZIP produced by `ditto -c -k --sequesterRsrc --keepParent` |
| Top-level entries | Exactly one: `Site Checker.app` |
| Signature | Developer ID Application, hardened runtime enabled |
| Notarization | Ticket stapled to the `.app` **before** zipping |
| Bundle identifier | `com.clintparker.site-checker` (unchanged) |

**Acceptance check** — what FR-021 runs after a real install:

```sh
spctl -a -t exec -vv "/Applications/Site Checker.app"
# expected: accepted
#           source=Notarized Developer ID
```

`codesign -dv --verbose=4` additionally shows the `Developer ID Application` authority and the
hardened-runtime flag. An ad-hoc-signed bundle fails `spctl` regardless of quarantine state — this
is the check that makes research R2's decision non-optional.

## Checksums

SHA-256, computed by the `homebrew` job from the assets it downloads **from the published
release** — never from the build job's working directory and never predicted. This is what makes
a missing or renamed asset fail the release instead of publishing a cask that 404s (FR-011, and
the spec's "artifact missing when the channel is updated" edge case).

## Cardinality

Exactly two artifacts per release. One missing is a failed release, not a partial one: the
`homebrew` job's `gh release download` loop fails on the missing pattern and the tap is never
touched (FR-005, SC-004).
</content>
