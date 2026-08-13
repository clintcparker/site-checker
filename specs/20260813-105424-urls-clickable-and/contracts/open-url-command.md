# Contract: `open_url` Tauri command

The one new interface this feature exposes across the Rust/TS boundary.

## Signature

**Rust** (`src-tauri/src/commands.rs`), registered in `lib.rs`'s
`generate_handler!` list:

```rust
#[tauri::command(async)]
pub fn open_url(url: String) -> Result<(), String>
```

`(async)` on a synchronous function is load-bearing: without it Tauri runs the
body on the main thread and the wait on the child process stalls the window.
See [../research.md](../research.md) §3.

**TypeScript** (`src/api.ts`) — the only place the frontend may reach this
command from:

```ts
export function openUrl(url: string): Promise<void> {
  return invoke("open_url", { url });
}
```

## Argument

| Name | Type | Notes |
|---|---|---|
| `url` | `string` | The site's stored URL, passed **verbatim** — not trimmed, re-serialized, or re-normalized by the caller (FR-006). A single lowercase word, so Tauri's argument case conversion is a no-op on it (Constitution V). |

## Behaviour

1. Validate with `openable_url(&url)`. On `Err`, return it unchanged — nothing
   is spawned.
2. Spawn `/usr/bin/open` with the URL as its single argument and wait for it.
   Absolute path, so the launched binary does not depend on the inherited `PATH`.
3. Exit status `0` → `Ok(())`. Non-zero → `Err` carrying the child's stderr,
   trimmed, wrapped in a sentence that names the address.
4. Spawn failure (`io::Error`) → `Err` naming the failure.

The command makes no HTTP request, reads and writes no file, touches neither the
store nor the engine, and holds no lock. It therefore cannot alter a site's
stored data, its schedule, or its status (FR-010).

## Guard: `openable_url`

Pure function in `src-tauri/src/model.rs`, beside `normalize_url`. No
`AppHandle`, no filesystem — unit-tested by plain `cargo test`
(Constitution IV).

```rust
pub fn openable_url(input: &str) -> Result<String, String>
```

**Contract**: returns `input` **byte-identical** on success, or a message
explaining the refusal.

| Input | Result |
|---|---|
| `https://example.com` | `Ok("https://example.com")` |
| `http://example.com/health?q=A` | `Ok("http://example.com/health?q=A")` — path and query case preserved |
| `HTTPS://example.com` | `Ok("HTTPS://example.com")` — accepted (`url::Url` lowercases the scheme for the *check*; the returned value is untouched, and `open` handles it) |
| `ftp://example.com` | `Err` — scheme not http/https |
| `file:///etc/hosts` | `Err` |
| `javascript:alert(1)` | `Err` |
| `example.com` | `Err` — **not repaired**. Unlike `normalize_url`, this never prepends a scheme. |
| `""` / `"   "` | `Err` |
| `https://` (no host) | `Err` |

**It must not call `normalize_url`.** That function repairs a scheme-less
string, which would turn a hand-edited `sites.json` entry into something
openable rather than refusing it. The two contracts are opposites and live
adjacent so the contrast is visible. See [../research.md](../research.md) §4.

## Errors returned to the frontend

Rust `Err(String)` arrives in JS as the bare string — the idiom `form.ts`
already relies on. Every message must name a consequence the user can act on,
per the constitution's "never fail silently" posture and the frontend living
spec's "A rejected change is explained without discarding what was typed".

| Cause | Shape of the message |
|---|---|
| Non-http/https scheme | Names the scheme and that only http and https can be opened |
| Empty or unparseable | Names that the stored address is not a URL |
| No handler (no default browser) | Carries macOS's own stderr, so the reason is the system's, not a guess |
| Spawn failed | Names that the address could not be handed to macOS |

The frontend routes all of these to the **banner** (`showBanner`), not to the
form's `#site-error`: the failure is not about anything the user typed into the
form, and the frontend living spec assigns non-fatal, non-form problems to the
persistent notice (FR-009). The app stays fully usable throughout — the command
mutates nothing, so there is no partial state to unwind.

## What this contract does *not* include

- No `open_path`, no `reveal_in_dir`, no "open in a chosen browser". FR-002 says
  the system default; the spec's Assumptions decline a browser preference.
- No capability-file change. This is a first-party command, not a plugin, so it
  needs no permission entry in `src-tauri/capabilities/default.json`.
- No new dependency in `Cargo.toml`, `package.json`, or either lockfile.
