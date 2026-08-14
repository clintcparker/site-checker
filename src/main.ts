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
import { mountUrlOpener } from "./open";
import { mountAbout } from "./about";

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
  const previous = sites.get(site.id);

  // A status describes a URL, not a row. Once the URL changes, the last
  // confirmed result is about somewhere else — dropping it returns the row to
  // Pending, which is the honest state until the next check lands. A label- or
  // interval-only edit leaves the status alone; that result is still about this
  // URL. An add has no previous entry and so nothing to drop.
  //
  // Both sides of the comparison are backend-normalized: `addSite`/`updateSite`
  // resolve to the saved `Site` after `normalize_url`, so a purely cosmetic
  // difference the backend already collapsed cannot false-positive here.
  if (previous && previous.url !== site.url) statuses.delete(site.id);

  sites.set(site.id, site);
  repaint();
}

export function removeSite(id: string): void {
  sites.delete(id);
  statuses.delete(id);
  repaint();
}

async function mountAutostart(): Promise<void> {
  // Bailing out here rather than asserting non-null keeps the `catch` below
  // operating on a checkbox known to exist. With a non-null assertion, a
  // missing element made `catch`'s own `checkbox.disabled = true` throw, and
  // that second throw aborted the rest of `main()` — the whole page, over one
  // absent control.
  const checkbox = document.querySelector<HTMLInputElement>("#autostart");
  if (!checkbox) {
    showBanner("The autostart control is missing from the page.");
    return;
  }

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
  // These come before every other `await` on purpose. Tauri events have no
  // replay, so a `site-status` emitted before `listen` resolves is simply gone.
  // The startup calls below take milliseconds and the first check is jittered
  // 0-10s, so the window is small — but it costs nothing to close it.
  //
  // The consequence of the new order is benign: a status can now arrive for an
  // id `sites` does not hold yet. It lands in `statuses` and renders as soon as
  // `listSites()` populates `sites`, because `currentRows()` iterates `sites`,
  // not `statuses`. Both handlers close over module-level bindings that exist
  // before `main()` is ever called.
  await onSiteStatus((event) => {
    statuses.set(event.id, event);
    repaint();
  });
  await onStoreWarning(showBanner);

  for (const site of await listSites()) sites.set(site.id, site);
  repaint();

  mountForm({
    onSaved: upsertSite,
    onDeleted: removeSite,
    lookup: (id) => sites.get(id),
  });

  // A failed open is a problem that does not stop anything, so it goes to the
  // banner rather than the form's error line — nothing about it is about what
  // the user typed.
  mountUrlOpener(tbody, { onError: showBanner });

  // Same reasoning as above: a refused open is not about anything the user
  // typed, so it goes to the banner. `about.ts` closes the dialog first — a
  // modal would otherwise cover the message.
  mountAbout({ onError: showBanner });

  await mountAutostart();

  const startupWarning = await getWarning();
  if (startupWarning) showBanner(startupWarning);

  // The "time since" column ticks locally. It counts from the last completed
  // check, so this needs no backend chatter.
  setInterval(repaint, 1000);
}

main();
