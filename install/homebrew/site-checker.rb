# Homebrew cask TEMPLATE for site-checker — do not copy to the tap by hand.
# To use: brew install clintcparker/tap/site-checker
#
# The "homebrew" job in .github/workflows/release.yml renders this file on every
# release: it substitutes the version and per-architecture sha256 placeholders
# below (computed from the assets actually attached to the GitHub Release) and
# commits the result to clintcparker/homebrew-tap as Casks/site-checker.rb.
# The render step fails the release if any placeholder is missing here or
# survives into the rendered output.
#
# A cask rather than a formula because Site Checker ships an application bundle
# rather than a command-line binary. Homebrew resolves a tap-qualified name
# against both Formula/ and Casks/, so the advertised install line is unchanged.

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

  # The site list appears in exactly one stanza, and it is this opt-in one, so
  # `brew uninstall` keeps it and only `brew uninstall --zap` removes it.
  # `trash:` rather than `delete:` — the removal is recoverable.
  zap trash: [
    "~/Library/Application Support/com.clintparker.site-checker",
  ]
end
