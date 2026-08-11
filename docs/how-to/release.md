# How to Release Site Checker

The entire release procedure is pushing one annotated tag. Everything else — the
four gates, both architecture builds, ad-hoc signing, the GitHub Release, and the
Homebrew formula in the tap — is done by
[`.github/workflows/release.yml`](../../.github/workflows/release.yml).

## Release procedure

```sh
git tag -a v1.0.0 -m "site-checker 1.0.0"
git push origin v1.0.0
gh run watch   # "Release"
```

The tag **must** be `v<MAJOR>.<MINOR>.<PATCH>` — a `v` prefix and exactly three
numeric components. `v1.0.0`, `v1.2.3`, and `v10.0.1` release; `v1.2`, `vnext`,
and `1.0.0` do not.

**The tag must be annotated.** `git tag -a`, not a bare `git tag`. Pre-flight
checks this against the API — an annotated tag's ref points at a `tag` object, a
lightweight one points straight at a `commit`.

**Prereleases are rejected on purpose.** `v1.2.3-rc.1` fails pre-flight rather
than releasing. A tap has no channel concept, so a prerelease rendered into it
becomes *the* version served to everyone by `brew install`. If prereleases are
ever wanted, they need either a separate formula name or a `prerelease: true`
release that the `homebrew` job skips — a decision to make deliberately, not
something to allow by accident.

**There is no version number to bump in any file.** `src-tauri/Cargo.toml` and
`package.json` carry an inert `0.0.0`, `src-tauri/tauri.conf.json` has no
`version` key at all (Tauri falls back to `Cargo.toml`), and the release job
stamps the tag-derived version into `Cargo.toml` at build time. CI fails any pull
request that edits those sentinels — see the "Version sentinel" step in
[`ci.yml`](../../.github/workflows/ci.yml).

After the run goes green, the release-triggered **Verify Install Channels**
workflow confirms the assets are reachable, the tap advertises the same version
*and the same checksums*, and a real `brew install` on a clean macOS runner
produces a bundle that is unquarantined and whose signature verifies.

## One-time setup checklist

The `preflight` job fails within seconds — naming every missing piece in one run
rather than one per attempt — until all of the following exist. None of them
recur per release.

1. **The repository must be public.** ✅ Done — 2026-08-06. Homebrew downloads
   the `url` with an unauthenticated request, so a release asset on a private
   repository returns 404 to every user, including you on a machine without a
   token. There is no workaround: a tap entry cannot carry a credential.

2. **Public tap repository `clintcparker/homebrew-tap`.** ✅ Exists, and already
   holds `Formula/name-on.rb`. The release job writes
   `Formula/site-checker.rb` alongside it. Never edit that file by hand — the
   canonical template is
   [`install/homebrew/site-checker.rb`](../../install/homebrew/site-checker.rb)
   in this repository, and the tap is written only by automation.

3. **`TAP_PUSH_TOKEN`** — a fine-grained personal access token with
   **Contents: Read and write** on **only** `clintcparker/homebrew-tap`. Secrets
   do not cross repositories, so it must be set here even though `name-on`
   already has one. Create at
   <https://github.com/settings/personal-access-tokens/new>, then:

   ```sh
   gh secret set TAP_PUSH_TOKEN --repo clintcparker/site-checker
   ```

That is the whole checklist. There is no Apple credential to set — see below.

### Why this is a formula, and what that costs

**Decided 2026-08-11: no Apple Developer Program membership.** The consequence
is not cosmetic, so it is written down here rather than left to be rediscovered.

Homebrew *formulae* do not quarantine what they install; *casks* do —
unconditionally. `Cask::Download#fetch` attaches `com.apple.quarantine` to every
download, there has never been a `quarantine` cask DSL stanza, and Homebrew
removed `--no-quarantine` outright. A quarantined bundle carrying only an ad-hoc
signature opens as *"damaged"*, and **nothing the cask author or the user can set
will prevent it**. So a cask is only viable with a Developer ID signature and
notarization, which needs the $99/yr membership.

Shipping a formula sidesteps quarantine entirely: nothing flags the download, so
the app opens with no prompt and no right-click-open. `brew install
clintcparker/tap/site-checker` is unchanged, because Homebrew resolves a
tap-qualified name against both `Formula/` and `Casks/`.

Three things are genuinely lost, and none of them is recoverable without paying:

- **Provenance.** An ad-hoc signature proves the bundle has not been modified
  since it was signed. It proves nothing about *who* signed it. There is no
  longer any check that the bytes a user installs are the bytes CI built.
  `spctl -a -t exec` cannot pass, so FR-021 is amended to `codesign --verify`,
  which is a real check of a strictly weaker property.
- **`/Applications`.** A formula cannot write there — Homebrew's build and
  post-install sandboxes deny every write outside the Cellar, and `brew
  linkapps` is gone. The bundle lives in `libexec`, reached by the
  `site-checker` wrapper on `PATH` or by a symlink the user makes once.
- **Spotlight and Launchpad.** Neither indexes an app through a symlink.

If the membership is ever bought, the way back is: restore the certificate and
notarization steps in `release.yml`, swap this template back to a cask with an
`app`/`uninstall`/`zap` stanza, and put `spctl -a -t exec` back in
`verify-install-channels.yml`. Nothing else changes.

## What pre-flight can and cannot tell you

Pre-flight checks that `TAP_PUSH_TOKEN` is **present**, and that the tag is
annotated and correctly shaped. It cannot check that the token is **valid**.

A revoked token, or one scoped to the wrong repository, passes pre-flight and
fails later at the tap push. This is a known limit, not an oversight: validating
a credential means using it, and using it means building first, which is exactly
what pre-flight exists to avoid.

Credential expiry between releases is caught by the daily **Verify Install
Channels** run, not by a fast pre-flight message.

## Re-running a failed release

A partially failed run can be re-run from the same tag and converges rather than
erroring:

- **GitHub Release** — `softprops/action-gh-release` updates the existing release
  for the tag rather than failing, re-uploading assets as needed.
- **Tap formula commit** — skipped when the rendered `Formula/site-checker.rb`
  is byte-identical to what the tap already holds, so re-runs do not create empty
  commits.

**"Unchanged content → no commit" is not "re-running is a no-op."** A re-run
rebuilds, and a Tauri build is not bit-reproducible — timestamps differ and the
signature embeds a signing time — so the `.zip` checksums change, which changes
the rendered formula, which means the tap *will* commit again. That is correct:
the formula must match the assets that actually exist. The skip path is only
exercised when the release job did not re-upload, which in practice means
re-running the `homebrew` job alone.

This is also why the daily verification compares checksums and not just the
version string. A run where `release` succeeded and `homebrew` did not leaves a
tap whose version and URLs look right and whose `sha256` values point at bytes
that no longer exist — and `brew install` aborts on the mismatch.

To abandon a botched tag entirely:

```sh
git push origin :refs/tags/<tag>
gh release delete <tag> --yes   # only if a release object was created
```

## Building a DMG locally

Nothing distributes a DMG any more — the tap serves a `.zip` containing the
`.app`, and `tauri.conf.json`'s bundle targets are reduced to `["app"]` so no
automated build can reach the DMG bundler. That is deliberate: laying out a DMG's
Finder window calls `osascript`, which blocks forever waiting for a GUI-automation
approval that a headless runner can never grant.

If you want one on your own Mac, ask for it explicitly:

```sh
pnpm tauri build --bundles dmg
```
