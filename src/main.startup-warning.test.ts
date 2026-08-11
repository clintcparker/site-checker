import { beforeAll, describe, expect, it, vi } from "vitest";

/**
 * `getWarning`'s banner is the one startup path `main.test.ts` structurally
 * cannot reach.
 *
 * That file imports `main.ts` exactly once, in `beforeAll`, because importing it
 * *is* running startup — and it documents why it must not `vi.resetModules()`:
 * a fresh copy of the mocked `./api` would orphan the status handler it captured.
 * So the mocks there are fixed for the whole file, and `getWarning` resolves
 * null.
 *
 * A separate file is the cheap way out. Vitest gives each test file its own
 * module registry, so this one can mock `getWarning` to resolve a message and
 * watch the banner without disturbing anything next door.
 *
 * `#autostart` is present here, unlike in `main.test.ts` — that file omits it on
 * purpose to exercise US5. Including it means the banner asserted below can only
 * have come from the startup warning, not from the missing-control path.
 */

const WARNING = "sites.json is not valid JSON (expected value at line 1 column 1).";

vi.mock("./api", () => ({
  listSites: vi.fn(() => Promise.resolve([])),
  getWarning: vi.fn(() => Promise.resolve(WARNING)),
  getAutostart: vi.fn(() => Promise.resolve(false)),
  setAutostart: vi.fn(() => Promise.resolve(false)),
  onSiteStatus: vi.fn(() => Promise.resolve(() => {})),
  onStoreWarning: vi.fn(() => Promise.resolve(() => {})),
  addSite: vi.fn(),
  updateSite: vi.fn(),
  deleteSite: vi.fn(),
}));

function flush(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

beforeAll(async () => {
  document.body.innerHTML = `
    <div id="banner" class="banner" hidden></div>
    <table><tbody id="rows"></tbody></table>
    <p id="empty" class="empty" hidden></p>
    <input type="checkbox" id="autostart" />
    <form id="site-form" novalidate>
      <input type="hidden" id="site-id" />
      <input id="site-url" type="text" />
      <input id="site-label" type="text" />
      <input id="site-interval" type="number" min="10" max="86400" step="1" value="60" />
      <button type="submit" id="site-submit">Add</button>
      <button type="button" id="site-cancel" hidden>Cancel</button>
      <p id="site-error" hidden></p>
    </form>
  `;
  await import("./main");
  await flush();
});

describe("a warning from the last load", () => {
  it("reaches the banner at startup", () => {
    const banner = document.querySelector<HTMLElement>("#banner")!;

    expect(banner.hidden).toBe(false);
    expect(banner.textContent).toBe(WARNING);
  });

  it("does not stop the rest of startup", () => {
    // The warning is raised after `mountForm` and `mountAutostart`, so this is
    // really asserting the ordering holds: a banner is not an abort.
    expect(document.querySelector("#site-submit")).not.toBeNull();
    expect(document.querySelector<HTMLInputElement>("#autostart")!.disabled).toBe(false);
  });
});
