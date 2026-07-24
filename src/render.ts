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
  name.append(text("span", "site-primary", row.site.label ?? row.site.url));
  if (row.site.label) {
    name.append(text("span", "site-secondary", row.site.url));
  }

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

function updateName(name: HTMLElement, site: Site): void {
  const primary = name.children[0] as HTMLElement;
  setClass(primary, "site-primary");
  setText(primary, site.label ?? site.url);

  const secondary = name.children[1] as HTMLElement | undefined;
  if (site.label) {
    if (secondary) {
      setText(secondary, site.url);
    } else {
      name.append(text("span", "site-secondary", site.url));
    }
  } else if (secondary) {
    secondary.remove();
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
