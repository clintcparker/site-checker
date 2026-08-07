# Contract: Install Channel

The user-facing interface of this entire feature. Two files: a template this repository owns and
edits, and a rendered entry the tap holds and only automation writes.

## The advertised command

```sh
brew install clintcparker/tap/site-checker
```

Unchanged from the roadmap's promise. Homebrew resolves a tap-qualified name against **both**
`Formula/` and `Casks/` in the tap, so shipping a cask instead of a formula (FR-002) is invisible
here — the one deliberate divergence from `name-on` that does not leak into the convention.

Uninstall:

```sh
brew uninstall site-checker           # app gone, sites.json kept        (FR-004)
brew uninstall --zap site-checker     # app gone, sites.json trashed too (FR-003)
```

## Template

**Path**: `install/homebrew/site-checker.rb` — canonical, human-edited, never copied to the tap by
hand.

**Required header** (mirrors `install/homebrew/name-on.rb` in spirit): states that it is a
template, names `.github/workflows/release.yml` as the only thing that renders it, names its
destination `clintcparker/homebrew-tap` → `Casks/site-checker.rb`, and states that the render step
fails the release when a placeholder is missing here or survives into the output.

**Shape**:

```ruby
cask "site-checker" do
  arch arm: "aarch64", intel: "x86_64"

  version "VERSION"
  sha256 arm:   "SHA256_ARM64",
         intel: "SHA256_X86_64"

  url "https://github.com/clintcparker/site-checker/releases/download/v#{version}/site-checker-#{arch}-apple-darwin.zip"
  name "Site Checker"
  desc "Small macOS dashboard that checks whether your sites are up"
  homepage "https://github.com/clintcparker/site-checker"

  app "Site Checker.app"

  uninstall quit: "com.clintparker.site-checker"

  zap trash: [
    "~/Library/Application Support/com.clintparker.site-checker",
  ]
end
```

## Placeholders

| Placeholder | Replaced with | Source |
|---|---|---|
| `VERSION` | `1.2.3` | The tag |
| `SHA256_ARM64` | 64 hex chars | `sha256sum` of the downloaded `aarch64` asset |
| `SHA256_X86_64` | 64 hex chars | `sha256sum` of the downloaded `x86_64` asset |

**Substitution safety.** `#{version}` and `#{arch}` are Ruby interpolations evaluated by Homebrew
at install time, not placeholders. Neither contains the literal token `VERSION`, so a
`sed 's/VERSION/1.2.3/g'` pass cannot corrupt them. This is load-bearing — if a future edit
introduces a placeholder that is a substring of another, ordering becomes significant and this
note stops being true.

## Render guards (FR-013 — both directions)

```sh
placeholders="VERSION SHA256_ARM64 SHA256_X86_64"

# 1. template drifted ahead of the workflow
for p in $placeholders; do
  grep -q "$p" "$template" || { echo "::error::placeholder $p missing from $template"; exit 1; }
done

# ... sed substitution ...

# 2. workflow drifted ahead of the template
for p in $placeholders; do
  grep -q "$p" rendered/site-checker.rb && { echo "::error::placeholder $p survived rendering"; exit 1; }
done
```

Both are hard failures of the release. Neither is a warning.

## Rendered entry

**Path**: `Casks/site-checker.rb` in `clintcparker/homebrew-tap` (public). The tap currently holds
`Formula/name-on.rb` and a README; `Casks/` is created by the release job on first publish.

| Property | Requirement |
|---|---|
| Written by | `release.yml`'s `homebrew` job, using `TAP_PUSH_TOKEN` |
| Written by hand | Never |
| Commit message | `site-checker <VERSION>` |
| Committed when | Content differs from what the tap holds; skipped otherwise (FR-014) |
| Contains | Zero placeholders, a literal `version "<L>"`, two literal checksums |

The literal `version "<L>"` line is the exact string `verify-install-channels.yml` greps for, so a
tap left behind by a failed release fails verification by construction (FR-020).

## Uninstall semantics — why `zap` and `uninstall` are separate

| Stanza | Runs on | Touches `~/Library/Application Support/com.clintparker.site-checker` |
|---|---|---|
| `uninstall quit:` | `brew uninstall` **and** `brew uninstall --zap` | No |
| `zap trash:` | `brew uninstall --zap` only | Yes — moves to Trash |

The data directory appears in exactly one stanza, and it is the opt-in one. This is what makes
FR-003 and FR-004 simultaneously satisfiable, and it is the mechanism backing Constitution II
("config is sacred") for the first uninstall path this project has ever had.

`trash:` rather than `delete:` — recoverable. `quit:` prevents removing the bundle out from under
a running instance.

## Compatibility with a hand-built copy

A user who built locally shares the same `sites.json` path. Installing via Homebrew picks up their
existing list, which is the desired behaviour. If they had already placed a hand-built
`Site Checker.app` in `/Applications`, Homebrew refuses to overwrite rather than clobbering it.
One README sentence; no mechanism.
</content>
