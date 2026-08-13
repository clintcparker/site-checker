# Contract: the row's URL element

The UI contract this feature adds to the site table. The page markup
(`index.html`) is unchanged — this is a contract on what `render.ts` produces
inside the existing `<td class="site">`, which the frontend living spec already
treats as part of the capability rather than as presentation.

## Structure of the name cell

`<td class="site">` keeps its two positional children.

**Unlabelled site** — the URL is the primary line:

```html
<td class="site">
  <button class="site-primary site-url" data-open-url="https://example.com">https://example.com</button>
</td>
```

**Labelled site** — the label is primary, the URL is secondary (FR-008):

```html
<td class="site">
  <span class="site-primary">Production</span>
  <button class="site-secondary site-url" data-open-url="https://example.com">https://example.com</button>
</td>
```

**Non-http/https entry** — shown, inert (FR-007):

```html
<td class="site">
  <span class="site-primary">ftp://example.com</span>
</td>
```

The `<span>` carries no `data-open-url`, no `site-url` class, no `tabindex`, and
no pointer cursor. It is not in the tab order and not announced as actionable.

## Element choice

A `<button type="button">`, not an `<a href>`. An element with no `href` cannot
be followed, which is what makes FR-003 hold under middle-click, Cmd-click, and
any path where the JS handler did not run. Reasoning and the accessibility cost
accepted: [../research.md](../research.md) §2.

`type="button"` is explicit so the element can never act as a submit control if
the markup is ever nested differently.

## Dispatch attribute

`data-open-url`, deliberately **not** `data-action`.

`form.ts`'s existing delegated listener matches `.closest("[data-action]")` for
the row's Edit and Delete buttons. Keying this control on a different attribute
means that listener never matches it at all, so activating a URL cannot reach
the row's other actions — FR-010 holds structurally, not by a convention someone
must remember. The new listener in `main.ts` matches
`.closest("[data-open-url]")` and is the only handler for it.

Its value is the **full stored URL**, independent of how much of it the cell
renders (FR-006). Reading the address from the attribute rather than from
`textContent` is what makes truncation or wrapping irrelevant.

## Interaction

| Input | Result |
|---|---|
| Click | The URL is opened. The dashboard does not navigate (FR-003). |
| Tab | The control takes focus, in row order, before Edit and Delete (FR-005). Native, no `tabindex` needed. |
| Enter, Space | Same as click — native `<button>` activation. |
| Hover | Pointer cursor and an underline; identifiable as openable before any click (FR-004). |
| Focus via keyboard | Visible `:focus-visible` ring. |
| Repeat within 1000 ms of the same URL | Suppressed — one browser navigation, not many (FR-012). See [../data-model.md](../data-model.md). |
| Click on the label of a labelled row | Nothing. The label is a `<span>`, not a control (FR-008). |

## Styling contract (`src/styles.css`)

`.site-url` must strip the native button chrome and read as a link, without
disturbing the row's existing rhythm:

- `background: none; border: none; padding: 0; font: inherit; color: inherit`
  — so the secondary line keeps its 12 px / 0.5 opacity treatment and the
  primary line keeps its own.
- `display: block; text-align: left` — matches `.site-primary`/`.site-secondary`,
  which are already `display: block`.
- `text-decoration: underline; cursor: pointer` — the FR-004 affordance.
- A `:focus-visible` outline — the FR-005 indication. `:focus-visible` rather
  than `:focus`, so a mouse click does not leave a ring behind.
- `text-align: left` matters: a `<button>` centres its text by default, which
  would visibly shift the URL against the other rows.

## Reconciliation contract

The existing rule from the frontend living spec holds unchanged: elements are
updated in place and never recreated on a repaint.

- **A repaint** (a status event, or the 1 s age tick) does not read `site.url`
  and writes nothing to this element. Focus and hover survive — FR-011, and US2
  scenario 3.
- **A URL edit** rewrites `textContent` and `data-open-url` through the existing
  change-guarded helpers, replacing the node only if activatability flipped.
- **A label edit** rebuilds the name cell's two children, because the URL moves
  between the primary and secondary slot. Scoped exception, justified in
  [../research.md](../research.md) §6.

## Failure surface

An open that fails puts its reason in the **banner** (`#banner`, via
`showBanner`) — not in the form's `#site-error`, which belongs to what the user
typed. The banner is the existing non-blocking notice the frontend living spec
assigns to problems that do not stop the app. Nothing else in the UI changes:
the table keeps updating, and adding, editing, deleting, and checking all
continue to work (FR-009, US3 scenario 2).
