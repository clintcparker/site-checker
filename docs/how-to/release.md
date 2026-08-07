# How to Release Site Checker

The entire release procedure is pushing one annotated tag. Everything else — the
four gates, both architecture builds, signing, notarization, stapling, the
GitHub Release, and the Homebrew cask in the tap — is done by
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

**Prereleases are rejected on purpose.** `v1.2.3-rc.1` fails pre-flight rather
than releasing. A cask has no channel concept, so a prerelease rendered into the
tap becomes *the* version served to everyone by `brew install`. If prereleases
are ever wanted, they need either a separate cask name or a `prerelease: true`
release that the `homebrew` job skips — a decision to make deliberately, not
something to allow by accident.

**There is no version number to bump in any file.** `src-tauri/Cargo.toml` and
`package.json` carry an inert `0.0.0`, `src-tauri/tauri.conf.json` has no
`version` key at all (Tauri falls back to `Cargo.toml`), and the release job
stamps the tag-derived version into `Cargo.toml` at build time. CI fails any pull
request that edits those sentinels — see the "Version sentinel" step in
[`ci.yml`](../../.github/workflows/ci.yml).

After the run goes green, the release-triggered **Verify Install Channels**
workflow confirms the assets are reachable, the tap advertises the same version,
and a real `brew install` on a clean macOS runner produces an app that Gatekeeper
accepts.

## One-time setup checklist

The `preflight` job fails within seconds — naming every missing piece in one run
rather than one per attempt — until all of the following exist. None of them
recur per release.

1. **The repository must be public.** Homebrew downloads a cask's `url` with an
   unauthenticated request, so a release asset on a private repository returns
   404 to every user, including you on a machine without a token. There is no
   cask-side workaround: casks cannot carry a credential.

   ```sh
   gh repo edit clintcparker/site-checker --visibility public --accept-visibility-change-consequences
   ```

2. **Public tap repository `clintcparker/homebrew-tap`.** It already exists and
   already holds `Formula/name-on.rb`; the release job creates `Casks/` on first
   publish. Never edit `Casks/site-checker.rb` there by hand — the canonical
   template is [`install/homebrew/site-checker.rb`](../../install/homebrew/site-checker.rb)
   in this repository, and the tap is written only by automation.

3. **`TAP_PUSH_TOKEN`** — a fine-grained personal access token with
   **Contents: Read and write** on **only** `clintcparker/homebrew-tap`. Secrets
   do not cross repositories, so it must be set here even though `name-on`
   already has one. Create at
   <https://github.com/settings/personal-access-tokens/new>, then:

   ```sh
   gh secret set TAP_PUSH_TOKEN --repo clintcparker/site-checker
   ```

4. **Apple Developer Program membership** ($99/yr) and a **Developer ID
   Application** certificate. Export it as a `.p12` and set three secrets:

   ```sh
   base64 -i DeveloperID.p12 | gh secret set APPLE_CERTIFICATE --repo clintcparker/site-checker
   gh secret set APPLE_CERTIFICATE_PASSWORD --repo clintcparker/site-checker
   gh secret set APPLE_SIGNING_IDENTITY --repo clintcparker/site-checker   # the certificate common name
   ```

5. **App Store Connect API key** for notarization — an Issuer ID, a Key ID, and
   the `.p8`:

   ```sh
   gh secret set APPLE_API_ISSUER --repo clintcparker/site-checker
   gh secret set APPLE_API_KEY_ID --repo clintcparker/site-checker
   base64 -i AuthKey_XXXXXXXXXX.p8 | gh secret set APPLE_API_KEY --repo clintcparker/site-checker
   ```

   An API key rather than an app-specific password on purpose: an app-specific
   password is tied to the Apple ID's 2FA and is revoked whenever that account's
   password changes, which would break a release silently and at the worst
   moment. API keys are revoked independently.

   Note the deliberate name shift inside the workflow: Tauri's `APPLE_API_KEY`
   environment variable wants the *Key ID*, while the repository secret of that
   name holds the base64 `.p8`. The workflow writes the `.p8` to a file and
   points `APPLE_API_KEY_PATH` at it.

### Why signing is not optional here

Homebrew *formulae* do not quarantine what they install; *casks* do. An
unnotarized `.app` installed from a cask opens as "damaged" on any machine but
the one that built it. The cheap alternative — `quarantine: false` in the cask —
would stop the user seeing a prompt but would not make the bundle verifiable, and
`spctl -a -t exec` would still reject it. That check is the one thing catching
"the maintainer sees a green release, the user sees a broken install", so it is
kept and the membership is paid for.

## What pre-flight can and cannot tell you

Pre-flight checks that every secret is **present**. It cannot check that any of
them is **valid**.

An expired Developer ID certificate, a revoked App Store Connect key, or a
`TAP_PUSH_TOKEN` scoped to the wrong repository all pass pre-flight and fail
later — at the signing step, at `notarytool submit`, or at the tap push. This is
a known limit, not an oversight: validating a certificate means using it, and
using it means building first, which is exactly what pre-flight exists to avoid.

Credential expiry between releases is caught by the daily **Verify Install
Channels** run, not by a fast pre-flight message.

## Re-running a failed release

A partially failed run can be re-run from the same tag and converges rather than
erroring:

- **GitHub Release** — `softprops/action-gh-release` updates the existing release
  for the tag rather than failing, re-uploading assets as needed.
- **Tap cask commit** — skipped when the rendered `Casks/site-checker.rb` is
  byte-identical to what the tap already holds, so re-runs do not create empty
  commits.

**"Unchanged content → no commit" is not "re-running is a no-op."** A re-run
rebuilds, and a Tauri build is not bit-reproducible — timestamps differ and the
signature embeds a signing time — so the `.zip` checksums change, which changes
the rendered cask, which means the tap *will* commit again. That is correct: the
cask must match the assets that actually exist. The skip path is only exercised
when the release job did not re-upload, which in practice means re-running the
`homebrew` job alone.

To abandon a botched tag entirely:

```sh
git push origin :refs/tags/<tag>
gh release delete <tag> --yes   # only if a release object was created
```

## Building a DMG locally

Nothing distributes a DMG any more — the cask serves a `.zip` containing the
`.app`, and `tauri.conf.json`'s bundle targets are reduced to `["app"]` so no
automated build can reach the DMG bundler. That is deliberate: laying out a DMG's
Finder window calls `osascript`, which blocks forever waiting for a GUI-automation
approval that a headless runner can never grant.

If you want one on your own Mac, ask for it explicitly:

```sh
pnpm tauri build --bundles dmg
```
