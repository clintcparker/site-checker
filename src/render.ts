import type { Site, StatusEvent } from "./api";
import { formatSince } from "./time";

export interface Row {
  site: Site;
  /** null until the first check of this session completes — the Pending state. */
  status: StatusEvent | null;
}

const DOT: Record<"up" | "down" | "pending", string> = {
  up: "🟢",
  down: "🔴",
  pending: "⚪",
};

const LABEL: Record<"up" | "down" | "pending", string> = {
  up: "Up",
  down: "Down",
  pending: "Pending",
};

/**
 * Whether a site's URL is offered as something that can be opened.
 *
 * This is the frontend half of a rule the backend's `openable_url` holds
 * authoritatively, and it is spelled in both places on purpose — the same
 * shape of known duplication as the interval floor. Whether a row's URL
 * renders as a control is a decision taken on every repaint, up to once a
 * second per row, so it cannot be an IPC round trip; and asking
 * asynchronously would still be answering a question this synchronous render
 * needs now. If the two ever drift, the UI offers something the backend then
 * refuses, which surfaces as a message in the banner rather than as silence.
 *
 * FR-007's second sentence makes that drift a defect and not just an
 * inefficiency: a URL the app will not open must not be *presented* as
 * activatable. A bare `^https?://` prefix test is not enough to hold it — the
 * QA run confirmed `https://`, `http://`, `https://[bad` and
 * `https://exa mple.com` all pass a prefix test, all render as buttons, and are
 * all refused by the backend. So this asks the platform's own URL parser the
 * same two questions `openable_url` asks `url::Url`: does it parse, and does it
 * have a host. `URL` is synchronous and pure, which is what the repaint needs.
 *
 * The host is then checked for `%` and whitespace, and that third question is
 * the one experience had to teach. Engines disagree about a *forbidden*
 * character in the host: Rust's `url` refuses `https://exa mple.com`, and
 * Chromium's forgiving host parser instead percent-encodes it into the host as
 * `exa%20mple.com` and reports success. Delegating the whole verdict to `URL`
 * therefore makes this function's answer depend on which engine is hosting it —
 * and this app ships in neither of the two it gets tested in, but in WKWebView.
 * A real host never contains a `%` or a space, so rejecting those closes the
 * gap the same way in every engine. It deliberately looks at the parsed `host`
 * and not at the whole string, so a legitimate space in a *path*
 * (`https://example.com/a b`, which the add form accepts and the backend opens)
 * is left alone.
 */
export function isOpenable(url: string): boolean {
  const trimmed = url.trim();
  // Cheap scheme test first: it is the only question for the common refusal
  // (a hand-edited `ftp://` entry) and it keeps a non-http scheme from ever
  // reaching the parser.
  if (!/^https?:\/\//i.test(trimmed)) return false;
  try {
    const { host } = new URL(trimmed);
    return host !== "" && !/[%\s]/.test(host);
  } catch {
    return false;
  }
}

/**
 * Reconciles `tbody`'s `<tr>` children against `rows`, keyed by `row.site.id`,
 * instead of tearing the table down and rebuilding it every call. This runs
 * once a second from `main.ts`'s repaint timer, and element identity has to
 * survive that: a native `title` tooltip only appears under sustained hover
 * on the *same* element, and keyboard focus on a row-action button is lost
 * the instant its element is replaced. So an existing row's `<tr>`, dot
 * `<span>`, and action `<button>`s are updated in place and never recreated;
 * only a genuinely new site gets a new `<tr>`, and only a site no longer in
 * `rows` gets removed.
 */
export function renderTable(tbody: HTMLElement, rows: Row[], now: number): void {
  const existingById = new Map<string, HTMLTableRowElement>();
  for (const child of Array.from(tbody.children)) {
    const id = (child as HTMLElement).dataset.id;
    if (id) existingById.set(id, child as HTMLTableRowElement);
  }

  const keepIds = new Set(rows.map((row) => row.site.id));
  for (const [id, tr] of existingById) {
    if (!keepIds.has(id)) tr.remove();
  }

  rows.forEach((row, index) => {
    const existing = existingById.get(row.site.id);
    const tr = existing ? updateRow(existing, row, now) : renderRow(row, now);

    // Only touch the DOM position if it's actually wrong — moving a node
    // (even to the spot it's already in) can break an in-progress hover.
    const current = tbody.children.item(index);
    if (current !== tr) {
      tbody.insertBefore(tr, current);
    }
  });
}

function renderRow(row: Row, now: number): HTMLTableRowElement {
  const state = row.status?.state ?? "pending";

  const tr = document.createElement("tr");
  tr.dataset.id = row.site.id;

  const name = document.createElement("td");
  name.className = "site";
  name.append(...nameChildren(row.site));

  const status = document.createElement("td");
  status.className = "status";
  const dot = text("span", `dot dot-${state}`, DOT[state]);
  // The failure reason lives in a tooltip, per the spec.
  if (row.status?.reason) dot.title = row.status.reason;
  status.append(dot, text("span", "status-label", LABEL[state]));

  const since = document.createElement("td");
  since.className = "since";
  since.textContent = formatSince(row.status?.checked_at ?? null, now);

  const actions = document.createElement("td");
  actions.className = "actions";
  actions.append(
    button("edit", "Edit", row.site.id),
    button("delete", "Delete", row.site.id),
  );

  tr.append(name, status, since, actions);
  return tr;
}

/** Updates an existing `<tr>` (previously produced by `renderRow`) to match
 *  `row`, writing to a node's `textContent` / `className` / `title` only
 *  when the value actually changed, so untouched nodes never lose an
 *  in-progress hover or a mid-interaction DOM mutation. */
function updateRow(tr: HTMLTableRowElement, row: Row, now: number): HTMLTableRowElement {
  const state = row.status?.state ?? "pending";

  const name = tr.children[0] as HTMLElement;
  updateName(name, row.site);

  const status = tr.children[1] as HTMLElement;
  const dot = status.children[0] as HTMLElement;
  const statusLabel = status.children[1] as HTMLElement;
  setClass(dot, `dot dot-${state}`);
  setText(dot, DOT[state]);
  setTitle(dot, row.status?.reason ?? "");
  setText(statusLabel, LABEL[state]);

  const since = tr.children[2] as HTMLElement;
  setText(since, formatSince(row.status?.checked_at ?? null, now));

  // The actions cell's buttons are keyed by the same site id as the row
  // itself, which cannot change without the row being re-keyed — nothing
  // there ever needs updating in place.

  return tr;
}

/** The name cell's children, in order. The URL takes the primary line when
 *  there is no label and the secondary one when there is; the label itself is
 *  never a control, so clicking it does nothing. An address that cannot be
 *  opened is shown as plain text in whichever slot it occupies — present and
 *  readable, but not offered as activatable and not in the tab order. */
function nameChildren(site: Site): HTMLElement[] {
  const slot = site.label ? "site-secondary" : "site-primary";
  const url = urlElement(slot, site.url);
  return site.label ? [text("span", "site-primary", site.label), url] : [url];
}

function urlElement(slot: string, url: string): HTMLElement {
  return isOpenable(url) ? urlButton(slot, url) : text("span", slot, url);
}

/** A `<button type="button">`, not an `<a href>`. An anchor is the more
 *  semantic element and would get keyboard activation free, but it carries a
 *  URL this webview can follow on a middle-click, a Cmd-click, or any path
 *  where the JS handler did not run — and navigating the dashboard away from
 *  itself is unrecoverable. A button has nothing to navigate to. The cost is
 *  that assistive technology announces "button" rather than "link".
 *
 *  `data-open-url`, deliberately not `data-action`: `form.ts`'s row listener
 *  matches `.closest("[data-action]")`, so keying this control on a different
 *  attribute is what makes activating a URL structurally unable to reach a
 *  row's Edit or Delete — rather than a convention someone has to remember.
 *  The attribute carries the whole stored address no matter how much of it the
 *  cell ends up rendering, which is what makes truncation and wrapping
 *  irrelevant to what actually opens. */
function urlButton(slot: string, url: string): HTMLButtonElement {
  const el = document.createElement("button");
  el.type = "button";
  el.className = `${slot} site-url`;
  el.dataset.openUrl = url;
  el.textContent = url;
  return el;
}

function updateName(name: HTMLElement, site: Site): void {
  const hadLabel = name.children.length > 1;
  const hasLabel = Boolean(site.label);

  // Adding or removing a label moves the URL between the primary and the
  // secondary slot, so the node holding it changes role — and, when the URL is
  // openable, element type, which the DOM cannot do in place. Rebuilding the
  // cell's two children is the honest way to say that. It does not weaken the
  // never-recreate rule the rest of this file keeps: a label only changes on a
  // user-initiated save, which has already left the row, so no hover or focus
  // is in progress on what is being replaced. A repaint never gets here.
  if (hadLabel !== hasLabel) {
    name.replaceChildren(...nameChildren(site));
    return;
  }

  const slot = hasLabel ? "site-secondary" : "site-primary";
  const urlEl = name.children[hasLabel ? 1 : 0] as HTMLElement;

  if (hasLabel) {
    const label = name.children[0] as HTMLElement;
    setClass(label, "site-primary");
    setText(label, site.label!);
  }

  // An edit can turn an address that could not be opened into one that can
  // (never the other way round — `update_site` normalizes, so a non-http
  // scheme cannot be saved). That is an element-type change, so the node is
  // replaced rather than mutated.
  const openable = isOpenable(site.url);
  if (openable !== urlEl.hasAttribute("data-open-url")) {
    urlEl.replaceWith(urlElement(slot, site.url));
    return;
  }

  setClass(urlEl, openable ? `${slot} site-url` : slot);
  setText(urlEl, site.url);
  if (openable && urlEl.dataset.openUrl !== site.url) {
    urlEl.dataset.openUrl = site.url;
  }
}

function setText(el: HTMLElement, value: string): void {
  if (el.textContent !== value) el.textContent = value;
}

function setClass(el: HTMLElement, value: string): void {
  if (el.className !== value) el.className = value;
}

/** Writes `title` only on change, and removes the attribute entirely (rather
 *  than leaving a stale reason showing) once there's nothing to say. */
function setTitle(el: HTMLElement, value: string): void {
  if (value) {
    if (el.getAttribute("title") !== value) el.title = value;
  } else if (el.hasAttribute("title")) {
    el.removeAttribute("title");
  }
}

function text(tag: string, className: string, content: string): HTMLElement {
  const el = document.createElement(tag);
  el.className = className;
  el.textContent = content;
  return el;
}

function button(action: string, label: string, id: string): HTMLButtonElement {
  const el = document.createElement("button");
  el.className = `row-action row-action-${action}`;
  el.dataset.action = action;
  el.dataset.id = id;
  el.textContent = label;
  return el;
}
