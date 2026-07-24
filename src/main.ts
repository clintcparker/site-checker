import {
  getAutostart,
  getWarning,
  listSites,
  onSiteStatus,
  onStoreWarning,
  setAutostart,
  type Site,
  type StatusEvent,
} from "./api";
import { renderTable, type Row } from "./render";
import { mountForm } from "./form";

const sites = new Map<string, Site>();
const statuses = new Map<string, StatusEvent>();

const tbody = document.querySelector<HTMLElement>("#rows")!;
const empty = document.querySelector<HTMLElement>("#empty")!;
const banner = document.querySelector<HTMLElement>("#banner")!;

export function currentRows(): Row[] {
  return [...sites.values()].map((site) => ({
    site,
    status: statuses.get(site.id) ?? null,
  }));
}

export function repaint(): void {
  const rows = currentRows();
  renderTable(tbody, rows, Date.now());
  empty.hidden = rows.length > 0;
}

export function showBanner(message: string): void {
  banner.textContent = message;
  banner.hidden = false;
}

export function upsertSite(site: Site): void {
  sites.set(site.id, site);
  repaint();
}

export function removeSite(id: string): void {
  sites.delete(id);
  statuses.delete(id);
  repaint();
}

async function mountAutostart(): Promise<void> {
  const checkbox = document.querySelector<HTMLInputElement>("#autostart")!;

  try {
    checkbox.checked = await getAutostart();
  } catch (message) {
    showBanner(`Could not read the login item: ${String(message)}`);
    checkbox.disabled = true;
    return;
  }

  checkbox.addEventListener("change", async () => {
    try {
      // Trust what the OS reports rather than what was clicked.
      checkbox.checked = await setAutostart(checkbox.checked);
    } catch (message) {
      checkbox.checked = !checkbox.checked;
      showBanner(`Could not change the login item: ${String(message)}`);
    }
  });
}

async function main(): Promise<void> {
  for (const site of await listSites()) sites.set(site.id, site);
  repaint();

  mountForm({
    onSaved: upsertSite,
    onDeleted: removeSite,
    lookup: (id) => sites.get(id),
  });

  await mountAutostart();

  const startupWarning = await getWarning();
  if (startupWarning) showBanner(startupWarning);

  await onSiteStatus((event) => {
    statuses.set(event.id, event);
    repaint();
  });
  await onStoreWarning(showBanner);

  // The "time since" column ticks locally. It counts from the last completed
  // check, so this needs no backend chatter.
  setInterval(repaint, 1000);
}

main();
