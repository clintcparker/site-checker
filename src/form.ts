import { addSite, deleteSite, updateSite, type Site } from "./api";

interface FormHooks {
  onSaved: (site: Site) => void;
  onDeleted: (id: string) => void;
  lookup: (id: string) => Site | undefined;
}

export const MIN_INTERVAL = 10;
const DEFAULT_INTERVAL = 60;
/**
 * A product guardrail, not a protocol limit: the backend enforces only the
 * `MIN_INTERVAL_SECS` floor, and a day between checks is already past the point
 * where this app is telling you anything useful. It exists to keep a pasted
 * 21-digit number from reaching a `u64` at the IPC boundary.
 *
 * The value is spelled out in more than one place — here, `index.html`'s `max`
 * attribute, and the `#site-interval` markup inside each test file's fixture DOM.
 * That is not ideal, but it is now *pinned*: `interval-bounds.test.ts` scans
 * `index.html` and every `*.test.ts` in this directory and fails if any copy
 * disagrees with these two constants. Add another copy anywhere and the guard
 * finds it without being told.
 */
export const MAX_INTERVAL = 86400;

export function mountForm(hooks: FormHooks): void {
  const form = document.querySelector<HTMLFormElement>("#site-form")!;
  const idField = document.querySelector<HTMLInputElement>("#site-id")!;
  const urlField = document.querySelector<HTMLInputElement>("#site-url")!;
  const labelField = document.querySelector<HTMLInputElement>("#site-label")!;
  const intervalField = document.querySelector<HTMLInputElement>("#site-interval")!;
  const submit = document.querySelector<HTMLButtonElement>("#site-submit")!;
  const cancel = document.querySelector<HTMLButtonElement>("#site-cancel")!;
  const error = document.querySelector<HTMLElement>("#site-error")!;
  const tbody = document.querySelector<HTMLElement>("#rows")!;

  function showError(message: string): void {
    error.textContent = message;
    error.hidden = false;
  }

  function clearError(): void {
    error.hidden = true;
  }

  function resetToAddMode(): void {
    idField.value = "";
    urlField.value = "";
    labelField.value = "";
    intervalField.value = String(DEFAULT_INTERVAL);
    submit.textContent = "Add";
    cancel.hidden = true;
    clearError();
  }

  function enterEditMode(site: Site): void {
    idField.value = site.id;
    urlField.value = site.url;
    labelField.value = site.label ?? "";
    intervalField.value = String(site.interval_secs);
    submit.textContent = "Save";
    cancel.hidden = false;
    clearError();
    urlField.focus();
  }

  form.addEventListener("submit", async (e) => {
    e.preventDefault();

    // Disabling the button below stops a second *click*, but this handler also
    // runs for a programmatic submit, which never consults `disabled`. The
    // explicit check is what actually makes one save mean one command.
    if (submit.disabled) return;

    clearError();

    const url = urlField.value.trim();
    if (url === "") {
      showError("Enter a URL");
      return;
    }

    // The backend clamps the floor too; doing it here keeps the field honest
    // about what was actually saved. The ceiling is enforced only here and by
    // `index.html`'s `max` — the form is `novalidate`, so that attribute shapes
    // the spinner rather than blocking a paste, and this clamp is what a pasted
    // value actually meets.
    const parsed = Number.parseInt(intervalField.value, 10);
    const interval = Number.isNaN(parsed)
      ? DEFAULT_INTERVAL
      : Math.min(MAX_INTERVAL, Math.max(MIN_INTERVAL, parsed));

    const label = labelField.value.trim() || null;
    const id = idField.value;

    try {
      submit.disabled = true;
      const saved = id
        ? await updateSite(id, url, label, interval)
        : await addSite(url, label, interval);
      hooks.onSaved(saved);
      resetToAddMode();
    } catch (message) {
      // Rust `Err(String)` arrives here as the bare string.
      showError(String(message));
    } finally {
      // `finally`, not just the success path — a rejected save must stay
      // retryable, otherwise one bad URL locks the form for good.
      submit.disabled = false;
    }
  });

  cancel.addEventListener("click", resetToAddMode);

  tbody.addEventListener("click", async (e) => {
    // `render.ts` builds both row actions as real <button>s, which is what
    // makes the `disabled` guard below available.
    const button = (e.target as HTMLElement).closest<HTMLButtonElement>("[data-action]");
    if (!button) return;

    const id = button.dataset.id!;
    if (button.dataset.action === "edit") {
      const site = hooks.lookup(id);
      if (site) enterEditMode(site);
      return;
    }

    if (button.dataset.action === "delete") {
      if (button.disabled) return;
      button.disabled = true;

      try {
        await deleteSite(id);
      } catch (message) {
        // Rust `Err(String)` arrives here as the bare string.
        button.disabled = false;
        showError(String(message));
        return;
      }

      // Deliberately no re-enable on success: `onDeleted` removes the row, and
      // this button with it. Only the failure path above has a button left to
      // hand back to the user.
      hooks.onDeleted(id);
      if (idField.value === id) resetToAddMode();
    }
  });

  resetToAddMode();
}
