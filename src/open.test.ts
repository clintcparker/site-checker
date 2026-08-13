import { beforeEach, describe, expect, it, vi } from "vitest";
import { openUrl, type Site } from "./api";
import { renderTable, type Row } from "./render";
import { ACTIVATION_WINDOW_MS, mountUrlOpener, shouldOpen } from "./open";

// The same convention `form.test.ts` and `main.test.ts` use. `open.ts` needs no
// Tauri backend behind it — the whole module is a shell over this one call.
vi.mock("./api", () => ({
  openUrl: vi.fn(() => Promise.resolve()),
}));

const URL_A = "https://a.example.com";
const URL_B = "https://b.example.com";
const NOW = 1_700_000_000_000;

function site(id: string, overrides: Partial<Site> = {}): Site {
  return {
    id,
    url: `https://${id}.example.com`,
    method_override: null,
    interval_secs: 30,
    ...overrides,
  };
}

function flush(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

/** A real table body, built by `render.ts` rather than hand-typed, so these
 *  tests break if the two sides of the `data-open-url` contract ever drift. */
function mountRows(rows: Row[], now: () => number = () => NOW): HTMLElement {
  const tbody = document.createElement("tbody");
  renderTable(tbody, rows, NOW);
  mountUrlOpener(tbody, { onError: (message) => errors.push(message), now });
  return tbody;
}

let errors: string[];

beforeEach(() => {
  vi.mocked(openUrl).mockReset();
  vi.mocked(openUrl).mockResolvedValue(undefined);
  errors = [];
});

describe("shouldOpen", () => {
  it("accepts a first activation", () => {
    expect(shouldOpen(new Map(), URL_A, NOW)).toBe(true);
  });

  it("suppresses a repeat of the same address inside the window", () => {
    const ledger = new Map<string, number>();
    expect(shouldOpen(ledger, URL_A, NOW)).toBe(true);
    expect(shouldOpen(ledger, URL_A, NOW + 1)).toBe(false);
    expect(shouldOpen(ledger, URL_A, NOW + ACTIVATION_WINDOW_MS - 1)).toBe(false);
  });

  it("accepts a repeat once the window has passed", () => {
    const ledger = new Map<string, number>();
    shouldOpen(ledger, URL_A, NOW);
    expect(shouldOpen(ledger, URL_A, NOW + ACTIVATION_WINDOW_MS)).toBe(true);
  });

  it("gives each address its own window", () => {
    const ledger = new Map<string, number>();
    expect(shouldOpen(ledger, URL_A, NOW)).toBe(true);
    // Two different sites clicked in quick succession are two intentional
    // opens, not impatience.
    expect(shouldOpen(ledger, URL_B, NOW + 1)).toBe(true);
  });

  it("does not let a suppressed activation extend the window", () => {
    const ledger = new Map<string, number>();
    shouldOpen(ledger, URL_A, NOW);

    // Someone drumming on the control through the whole window.
    for (let t = 1; t < ACTIVATION_WINDOW_MS; t += 100) {
      expect(shouldOpen(ledger, URL_A, NOW + t)).toBe(false);
    }

    // The assertion with teeth: if a rejection refreshed the entry, the window
    // would keep sliding ahead of the user and the address would never open.
    expect(shouldOpen(ledger, URL_A, NOW + ACTIVATION_WINDOW_MS)).toBe(true);
  });
});

describe("mountUrlOpener", () => {
  it("opens the address in the attribute when the control is clicked", async () => {
    const long = `https://example.com/${"segment/".repeat(40)}end?q=1`;
    const tbody = mountRows([{ site: site("a", { url: long }), status: null }]);

    tbody.querySelector<HTMLButtonElement>("[data-open-url]")!.click();
    await flush();

    // The whole stored address, not whatever the cell happened to render.
    expect(openUrl).toHaveBeenCalledTimes(1);
    expect(openUrl).toHaveBeenCalledWith(long);
  });

  it("does nothing when a labelled row's label is clicked", async () => {
    const tbody = mountRows([
      { site: site("a", { url: URL_A, label: "Production" }), status: null },
    ]);

    (tbody.querySelector<HTMLElement>(".site-primary")!).click();
    await flush();

    expect(openUrl).not.toHaveBeenCalled();
  });

  it("does nothing when a row's Edit or Delete button is clicked", async () => {
    const tbody = mountRows([{ site: site("a"), status: null }]);

    for (const action of ["edit", "delete"]) {
      tbody.querySelector<HTMLButtonElement>(`[data-action="${action}"]`)!.click();
    }
    await flush();

    // The two listeners are keyed on different attributes, so neither can
    // reach the other's controls.
    expect(openUrl).not.toHaveBeenCalled();
  });

  it("does nothing when an address that cannot be opened is clicked", async () => {
    const tbody = mountRows([{ site: site("a", { url: "ftp://example.com" }), status: null }]);

    (tbody.querySelector<HTMLElement>(".site-primary")!).click();
    await flush();

    expect(openUrl).not.toHaveBeenCalled();
  });

  it("collapses a double-click into a single open", async () => {
    const tbody = mountRows([{ site: site("a", { url: URL_A }), status: null }]);
    const control = tbody.querySelector<HTMLButtonElement>("[data-open-url]")!;

    control.click();
    control.click();
    await flush();

    expect(openUrl).toHaveBeenCalledTimes(1);
  });

  it("opens again once the window has passed", async () => {
    let now = NOW;
    const tbody = mountRows([{ site: site("a", { url: URL_A }), status: null }], () => now);
    const control = tbody.querySelector<HTMLButtonElement>("[data-open-url]")!;

    control.click();
    now += ACTIVATION_WINDOW_MS;
    control.click();
    await flush();

    expect(openUrl).toHaveBeenCalledTimes(2);
  });

  it("routes a refusal to the error hook, bare, without throwing", async () => {
    vi.mocked(openUrl).mockRejectedValue(
      'Only http and https addresses can be opened, and this one uses "ftp". Nothing was opened.',
    );
    const tbody = mountRows([{ site: site("a", { url: URL_A }), status: null }]);

    tbody.querySelector<HTMLButtonElement>("[data-open-url]")!.click();
    await flush();

    // Rust `Err(String)` arrives as the bare string — no `.message` to unwrap.
    expect(errors).toEqual([
      'Only http and https addresses can be opened, and this one uses "ftp". Nothing was opened.',
    ]);
  });

  it("stays usable after a refusal", async () => {
    let now = NOW;
    const tbody = mountRows([{ site: site("a", { url: URL_A }), status: null }], () => now);
    const control = tbody.querySelector<HTMLButtonElement>("[data-open-url]")!;

    vi.mocked(openUrl).mockRejectedValueOnce("macOS would not open it.");
    control.click();
    await flush();
    expect(errors).toHaveLength(1);

    // Nothing was disabled and nothing was mutated on the way to the failure,
    // so the next attempt is an ordinary one.
    now += ACTIVATION_WINDOW_MS;
    control.click();
    await flush();
    expect(openUrl).toHaveBeenCalledTimes(2);
    expect(errors).toHaveLength(1);
  });
});
