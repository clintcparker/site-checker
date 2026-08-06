# Contract: `normalize_url`

**Module**: `src-tauri/src/model.rs` | **Feature**: `specs/003-durability`

`pub fn normalize_url(input: &str) -> Result<String, String>`

The single gate every stored URL passes through — `add_site` and `update_site` both call it
before constructing a `Site`. This feature changes exactly one thing about it: the scheme comes
back lowercase.

---

## The rule

> Trim, add `https://` if there is no leading scheme, **lowercase the scheme**, validate, and
> return the user's own text — not the re-serialized URL. — FR-007, FR-008

Returning the user's text rather than `url::Url`'s serialization is load-bearing and predates
this feature: it is why `example.com` yields `https://example.com` and not
`https://example.com/`. The lowercasing therefore has to happen *inside* the returned string,
not by switching to the parsed URL's own rendering. Doing the latter would re-add the trailing
slash and break SC-004.

---

## Accepted input → output

| Input | Output | Note |
|---|---|---|
| `example.com` | `https://example.com` | scheme added; **no trailing slash** |
| `  example.com  ` | `https://example.com` | trimmed |
| `http://example.com` | `http://example.com` | already lowercase, untouched |
| `https://api.foo.dev/health` | `https://api.foo.dev/health` | path preserved |
| **`HTTPS://example.com`** | **`https://example.com`** | **new** — spec scenario 1 |
| **`HtTp://example.com/health`** | **`http://example.com/health`** | **new** — scheme only; path case kept. Spec scenario 2 |
| `example.com?next=http://x.dev` | `https://example.com?next=http://x.dev` | the `://` in the query is not a leading scheme |
| **`example.com?next=HTTP://x.dev`** | **`https://example.com?next=HTTP://x.dev`** | **query verbatim** — the embedded `HTTP` is *not* lowercased. Spec scenario 4 |
| `https://EXAMPLE.com` | `https://EXAMPLE.com` | host case preserved — out of scope by decision |

## Rejected input → error

| Input | Error | Changed? |
|---|---|---|
| `""`, `"   "` | `Enter a URL` | no |
| `http://` | `Not a valid URL` | no |
| `not a url at all` | `Not a valid URL` | no |
| `ftp://example.com` | `Only http and https URLs are supported` | no |
| **`FTP://example.com`** | `Only http and https URLs are supported` | **still rejected** — spec scenario 5 |
| `file:///etc/hosts` | `Only http and https URLs are supported` | no |

Every error string is unchanged. — FR-008, FR-011

---

## Mechanism

`has_leading_scheme(&str) -> bool` becomes a function returning the scheme's byte length,
`Option<usize>`. The predicate is otherwise identical: a leading `scheme://` where the offset
is non-zero and every character before `://` is ASCII alphanumeric or one of `+`, `-`, `.`.

- `Some(i)` → `candidate` is `input[..i].to_ascii_lowercase()` concatenated with `input[i..]`
  verbatim.
- `None` → `candidate` is `format!("https://{trimmed}")`, exactly as today.

`to_ascii_lowercase`, not `to_lowercase`: URL schemes are ASCII by definition and the
character guard already proves it. The slice at `i` needs no boundary check — `find` returns a
byte index at a character boundary and the guard proves every preceding byte is ASCII.

Validation after that point is untouched: parse, reject non-`http`/`https`, reject a missing
or empty host.

---

## Scope boundaries

- **Only the leading scheme is case-normalized.** Not the host (case-insensitive but the
  spec's Assumptions rule it out as rewriting more of the user's text than was asked), not
  the path, not the query.
- **No migration.** A `sites.json` already holding `HTTPS://example.com` keeps it. `load()`
  does not call this function. The value is normalized only when the user next edits that
  site. — spec Assumptions, FR-005
- **Known consequence of that edit.** Changing a stored URL's scheme case *is* a URL change by
  the existing `update_site` rule, so `method_override` is cleared and HEAD support is
  re-learned on the next check — one extra request for that one site, once. Accepted under
  Constitution III.
