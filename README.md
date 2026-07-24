# Site Checker

A personal status dashboard for the websites and endpoints I care about. It
answers one question: is this thing up, and how long ago did we last confirm
that?

Not a monitoring service — no alerting, no history, no SLA math. One Mac, one
person.

## Requirements

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

The bundle lands in `src-tauri/target/release/bundle/`.

## Where data lives

`~/Library/Application Support/com.clintparker.site-checker/sites.json`

Check results are never written to disk — every site starts Pending on launch.

## Design

See [docs/superpowers/specs/2026-07-23-site-checker-design.md](docs/superpowers/specs/2026-07-23-site-checker-design.md).
