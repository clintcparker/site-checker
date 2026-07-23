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

export function renderTable(tbody: HTMLElement, rows: Row[], now: number): void {
  tbody.replaceChildren(...rows.map((row) => renderRow(row, now)));
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
