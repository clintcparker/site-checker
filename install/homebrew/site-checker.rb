# Homebrew formula TEMPLATE for site-checker — do not copy to the tap by hand.
# To use: brew install clintcparker/tap/site-checker
#
# The "homebrew" job in .github/workflows/release.yml renders this file on every
# release: it substitutes the version and per-architecture sha256 placeholders
# below (computed from the assets actually attached to the GitHub Release) and
# commits the result to clintcparker/homebrew-tap as Formula/site-checker.rb.
# The render step fails the release if any placeholder is missing here or
# survives into the rendered output, and refuses to publish a file that is not
# valid Ruby.
#
# A FORMULA, not a cask, even though Site Checker ships an application bundle.
#
# Casks apply com.apple.quarantine unconditionally: Cask::Download#fetch
# quarantines every download, there has never been a `quarantine` cask DSL
# stanza, and Homebrew removed --no-quarantine outright ("Prepare for
# deprecation of `--no-quarantine`", then "Remove leftover code for
# `--no-quarantine`"). A quarantined bundle carrying only an ad-hoc signature
# opens as "damaged", and no cask-side or user-side setting can prevent it.
# Formulae never set the attribute — every Quarantine call site in Homebrew
# lives under its cask/ directory. Homebrew resolves a tap-qualified name
# against both Formula/ and Casks/, so the advertised install line is unchanged.
#
# What the switch costs is real and is recorded in docs/ROADMAP.md: a formula
# cannot write to /Applications, because Homebrew's build and post-install
# sandboxes deny every write outside the Cellar. The bundle therefore lives in
# the keg, reached by the `site-checker` wrapper below or by the optional
# symlink in `caveats`. Spotlight and Launchpad do not index through a symlink.

class SiteChecker < Formula
  desc "Small macOS dashboard that checks whether your sites are up"
  homepage "https://github.com/clintcparker/site-checker"
  license "0BSD"
  version "VERSION"

  depends_on :macos

  on_arm do
    url "https://github.com/clintcparker/site-checker/releases/download/v#{version}/site-checker-aarch64-apple-darwin.zip"
    sha256 "SHA256_ARM64"
  end

  on_intel do
    url "https://github.com/clintcparker/site-checker/releases/download/v#{version}/site-checker-x86_64-apple-darwin.zip"
    sha256 "SHA256_X86_64"
  end

  # libexec rather than the keg root, and the choice is load-bearing. Cleaner
  # walks the prefix removing empty directories and unresolved symlinks and
  # chmodding what it finds, but calls Find.prune at libexec — so nothing inside
  # the bundle is touched and the ad-hoc signature applied during the release
  # stays sealed. `brew test` below is what would catch that changing.
  #
  # The two branches are not defensive padding — the first is the live path.
  # Homebrew stages an archive by extracting it and then, when exactly one
  # top-level entry remains and it is a directory, chdir-ing *into* it
  # (AbstractDownloadStrategy#chdir). Ours always leaves exactly one:
  # `ditto --keepParent` puts "Site Checker.app" at the archive root, and the zip
  # strategy deletes the sibling __MACOSX before the count is taken. So `install`
  # runs *inside* the bundle, and naming it would look for a nested copy of
  # itself — which is exactly how the first throwaway release failed, with
  # "Errno::ENOENT: No such file or directory - Site Checker.app".
  #
  # The second branch stays because the chdir is Homebrew's behaviour, not ours:
  # an archive that ever gains a second top-level entry stages *beside* the
  # bundle instead, and should install rather than fail.
  def install
    staged = Pathname.pwd
    if staged.basename.to_s == "Site Checker.app"
      # `Contents` by name, and *not* the staged directory's children. Homebrew
      # sets buildpath to the staged directory — which, after the chdir above, is
      # the bundle itself — and creates `.brew_home` inside it to use as HOME for
      # the install (Formula#stage). Sweeping in every child therefore installs
      # Homebrew's own scratch directory into the app, and `codesign --verify
      # --strict` rejects it with "unsealed contents present in the bundle root".
      # That is not a hypothetical: it is how the second throwaway release failed.
      #
      # Naming `Contents` is also simply the correct rule rather than a
      # workaround. A macOS bundle's root *is* `Contents` by definition, and it is
      # exactly what the signature seals — so anything else that appears beside it
      # is by construction not ours to install.
      (libexec/"Site Checker.app").install "Contents"
    else
      libexec.install "Site Checker.app"
    end
    (bin/"site-checker").write <<~SH
      #!/bin/bash
      exec /usr/bin/open -a "#{opt_libexec}/Site Checker.app"
    SH
    # Explicit rather than left to Cleaner. Cleaner would grant 0555 anyway,
    # because it treats a file with a shebang as executable — but that is a
    # detail of Homebrew's internals, and a wrapper that silently lands at 0444
    # is a broken install with no error to read.
    chmod 0555, bin/"site-checker"
  end

  def caveats
    <<~EOS
      Site Checker is an application bundle, installed at
        #{opt_libexec}/Site Checker.app

      Launch it from a terminal with:
        site-checker

      A Homebrew formula cannot install into /Applications. To reach it from
      Finder and the Dock, symlink it once — the link points at opt/, so it
      keeps working across upgrades:
        ln -s "#{opt_libexec}/Site Checker.app" /Applications/

      Spotlight and Launchpad do not index apps through a symlink, so use the
      `site-checker` command or open it from Finder.

      This build is signed ad-hoc rather than notarized, which is why it is a
      formula: nothing quarantines it, so it opens without a security prompt.
      What that does not give you is proof the bytes you got are the bytes that
      were built.

      Your site list lives at
        ~/Library/Application Support/com.clintparker.site-checker
      Homebrew never removes it — `brew uninstall site-checker` keeps it. To
      remove it yourself:
        rm -rf ~/Library/"Application Support"/com.clintparker.site-checker
      and delete /Applications/Site Checker.app if you made the symlink.
    EOS
  end

  # Cheap, and the tripwire for the one thing that would silently break the
  # bundle: anything in Homebrew's install path re-signing or rewriting the
  # inner binary would leave the seal stale and fail --verify here.
  test do
    app = libexec/"Site Checker.app"
    assert_equal version.to_s,
                 shell_output(
                   "/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' " \
                   "'#{app}/Contents/Info.plist'",
                 ).strip
    system "/usr/bin/codesign", "--verify", "--strict", app
  end
end
