import { addSite, deleteSite, updateSite, type Site } from "./api";

interface FormHooks {
  onSaved: (site: Site) => void;
  onDeleted: (id: string) => void;
  lookup: (id: string) => Site | undefined;
}

const MIN_INTERVAL = 10;
const DEFAULT_INTERVAL = 60;

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
    clearError();

    const url = urlField.value.trim();
    if (url === "") {
      showError("Enter a URL");
      return;
    }

    // The backend clamps too; doing it here keeps the field honest about what
    // was actually saved.
    const parsed = Number.parseInt(intervalField.value, 10);
    const interval = Number.isNaN(parsed)
      ? DEFAULT_INTERVAL
      : Math.max(MIN_INTERVAL, parsed);

    const label = labelField.value.trim() || null;
    const id = idField.value;

    try {
      const saved = id
        ? await updateSite(id, url, label, interval)
        : await addSite(url, label, interval);
      hooks.onSaved(saved);
      resetToAddMode();
    } catch (message) {
      // Rust `Err(String)` arrives here as the bare string.
      showError(String(message));
    }
  });

  cancel.addEventListener("click", resetToAddMode);

  tbody.addEventListener("click", async (e) => {
    const button = (e.target as HTMLElement).closest<HTMLElement>("[data-action]");
    if (!button) return;

    const id = button.dataset.id!;
    if (button.dataset.action === "edit") {
      const site = hooks.lookup(id);
      if (site) enterEditMode(site);
      return;
    }

    if (button.dataset.action === "delete") {
      try {
        await deleteSite(id);
      } catch (message) {
        // Rust `Err(String)` arrives here as the bare string.
        showError(String(message));
        return;
      }
      hooks.onDeleted(id);
      if (idField.value === id) resetToAddMode();
    }
  });

  resetToAddMode();
}
