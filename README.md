# Site Checker

A personal status dashboard for the websites and endpoints I care about. It
answers one question: is this thing up, and how long ago did we last confirm
that?

Not a monitoring service — no alerting, no history, no SLA math. One Mac, one
person.

![Site Checker's window: a table of sites with URL, status dot, and last-checked
time, an add/edit form beneath it, and a "Launch at login"
checkbox.](assets/screenshot.png)

That is the whole application. Sites you add are checked on their own interval,
the dot goes green or red, and the column on the right tells you how long ago
that was confirmed.

## Install

```sh
brew install clintcparker/tap/site-checker
```

No toolchain, no clone, no build. Then:

```sh
site-checker
```

Use the fully-qualified name — recent Homebrew refuses a bare `brew install
site-checker` from a third-party tap unless you name the tap in the command.

### Getting it into /Applications

Site Checker installs as a Homebrew **formula**, not a cask, so the bundle lives
in the Homebrew prefix rather than `/Applications`. That is deliberate: casks
attach a quarantine flag that macOS refuses to open for an app without a paid
Apple Developer signature, and there is no longer any way for a cask to opt out.
A formula is not quarantined, so the app just opens.

The `site-checker` command works immediately. For a Finder and Dock icon,
symlink it once — the link points at Homebrew's `opt` path, so it survives
upgrades:

```sh
ln -s "$(brew --prefix site-checker)/libexec/Site Checker.app" /Applications/
```

Spotlight and Launchpad do not index apps through a symlink, so launch it with
the `site-checker` command or from Finder.

What you give up versus a signed build: nothing verifies that the bytes you
downloaded are the bytes that were built. See
[docs/how-to/release.md](docs/how-to/release.md).

### Uninstall

```sh
brew uninstall site-checker
rm "/Applications/Site Checker.app"   # only if you made the symlink above
```

**Your site list is never touched by Homebrew** — a formula has no equivalent of
a cask's `--zap`, so uninstalling and reinstalling always brings your sites back.
To remove the list yourself:

```sh
rm -rf ~/Library/"Application Support"/com.clintparker.site-checker
```

If you previously built Site Checker yourself, the installed copy reads the same
`sites.json`, so your list comes across untouched. A hand-built copy in
`/Applications` and the Homebrew copy can coexist — they share the same site
list, so run one at a time.

## Build it yourself

Everything below is for working on Site Checker. You do not need any of it to
use it.

### Requirements

- macOS
- Rust stable (≥ 1.88) via [rustup](https://rustup.rs)
- Node + pnpm

## Develop

```bash
pnpm install
pnpm tauri dev
```

## Test

```bash
pnpm test                   # frontend: time formatter, table rendering, form, startup wiring
cd src-tauri && cargo test  # backend: model, store, lock discipline, and HTTP classifier
```

`pnpm build` is a gate too, not just a build — it runs `tsc` over the test files
as well, so a type error there fails the build even when `pnpm test` is green.

## Build

```bash
pnpm tauri build
```

The bundle lands in `src-tauri/target/release/bundle/macos/`. This produces the
`.app` only — nothing distributes a DMG any more, and building one calls out to
the Finder, which hangs anywhere without an interactive session. If you want a
DMG locally, ask for it explicitly:

```bash
pnpm tauri build --bundles dmg
```

The version reported by a local build is the `0.0.0` sentinel. Real versions are
written from the release tag; see [docs/how-to/release.md](docs/how-to/release.md).

## Release

Push one annotated `v<MAJOR>.<MINOR>.<PATCH>` tag. That is the entire procedure —
[docs/how-to/release.md](docs/how-to/release.md) covers the one-time setup and
what to do when a release fails partway through.

## Where data lives

`~/Library/Application Support/com.clintparker.site-checker/sites.json`

Check results are never written to disk — every site starts Pending on launch.

## How it was built

Each change was specified before it was written. The specs, plans, and task
breakdowns live in [specs/](specs/), and [CHANGELOG.md](CHANGELOG.md) links every
entry to the spec it came from.

## License

[0BSD](LICENSE) — do what you like with it, no attribution required.

`.specify/extensions/` vendors six third-party Spec Kit extensions that are **not**
covered by that license. Each keeps its own MIT license and copyright holder in its
own directory.
