import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Mirrors the Rust `Site`. Field names are snake_case — Tauri does not
 *  rename serialized struct fields, only command arguments. */
export interface Site {
  id: string;
  url: string;
  label?: string;
  interval_secs: number;
  method_override: "GET" | null;
}

/** Mirrors the Rust `StatusEvent`. */
export interface StatusEvent {
  id: string;
  state: "up" | "down";
  checked_at: number;
  reason: string | null;
}

export function listSites(): Promise<Site[]> {
  return invoke("list_sites");
}

export function getWarning(): Promise<string | null> {
  return invoke("get_warning");
}

// Command arguments ARE camelCase-converted by Tauri: intervalSecs → interval_secs.
export function addSite(
  url: string,
  label: string | null,
  intervalSecs: number,
): Promise<Site> {
  return invoke("add_site", { url, label, intervalSecs });
}

export function updateSite(
  id: string,
  url: string,
  label: string | null,
  intervalSecs: number,
): Promise<Site> {
  return invoke("update_site", { id, url, label, intervalSecs });
}

export function deleteSite(id: string): Promise<void> {
  return invoke("delete_site", { id });
}

export function onSiteStatus(
  handler: (event: StatusEvent) => void,
): Promise<UnlistenFn> {
  return listen<StatusEvent>("site-status", (e) => handler(e.payload));
}

export function onStoreWarning(
  handler: (message: string) => void,
): Promise<UnlistenFn> {
  return listen<{ message: string }>("store-warning", (e) =>
    handler(e.payload.message),
  );
}

export function getAutostart(): Promise<boolean> {
  return invoke("get_autostart");
}

export function setAutostart(enabled: boolean): Promise<boolean> {
  return invoke("set_autostart", { enabled });
}

// `url` is a single lowercase word, so the camelCase conversion noted above is
// a no-op on it. The value is passed verbatim — the backend refuses what it
// will not open rather than repairing it, so trimming or re-normalizing here
// would change which address actually opens.
export function openUrl(url: string): Promise<void> {
  return invoke("open_url", { url });
}
