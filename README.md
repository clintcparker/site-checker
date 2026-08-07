# Site Checker

A personal status dashboard for the websites and endpoints I care about. It
answers one question: is this thing up, and how long ago did we last confirm
that?

Not a monitoring service — no alerting, no history, no SLA math. One Mac, one
person.

## Install

> **Not released yet.** The install pipeline is built and tested, but no version
> has been published, so the command below does not work today. Until a release
> exists, [build it yourself](#build-it-yourself). Progress is in
> [CHANGELOG.md](CHANGELOG.md).

```sh
brew install clintcparker/tap/site-checker
```

That is the whole thing — no toolchain, no clone, no build.

### Uninstall

```sh
brew uninstall site-checker          # app gone, your site list kept
brew uninstall --zap site-checker    # app gone, your site list trashed too
```

The site list is only ever removed by `--zap`, and it goes to the Trash rather
than being deleted outright.

If you previously built Site Checker yourself, the installed copy reads the same
`sites.json`, so your list comes across untouched. Homebrew installs to
`/Applications`; if a hand-built `Site Checker.app` is already sitting there,
Homebrew refuses rather than overwriting it — move it aside first.

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
pnpm test                  # frontend: the relative-time formatter
cd src-tauri && cargo test  # backend: model, store, and HTTP classifier
```

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
