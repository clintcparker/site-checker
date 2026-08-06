import { beforeAll, describe, expect, it, vi } from "vitest";
import {
  getWarning,
  listSites,
  onSiteStatus,
  onStoreWarning,
  type Site,
  type StatusEvent,
} from "./api";

// Every Tauri-facing call is stubbed, which is what lets `main.ts` be imported
// at all: it calls `main()` at module load, so importing it *is* running
// startup. `listSites` and friends must resolve to real values rather than the
// bare `undefined` a default `vi.fn()` returns — `main()` iterates the result.
vi.mock("./api", () => ({
  listSites: vi.fn(() => Promise.resolve([])),
  getWarning: vi.fn(() => Promise.resolve(null)),
  getAutostart: vi.fn(() => Promise.resolve(false)),
  setAutostart: vi.fn(() => Promise.resolve(false)),
  onSiteStatus: vi.fn(() => Promise.resolve(() => {})),
  onStoreWarning: vi.fn(() => Promise.resolve(() => {})),
  addSite: vi.fn(),
  updateSite: vi.fn(),
  deleteSite: vi.fn(),
}));

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

function status(id: string, overrides: Partial<StatusEvent> = {}): StatusEvent {
  return { id, state: "up", checked_at: NOW, reason: null, ...overrides };
}

function flush(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

/**
 * The markup `main.ts` queries at module load, plus the form `mountForm` needs.
 *
 * `#autostart` is deliberately absent: that is US5's scenario, and leaving it
 * out here means every assertion below doubles as proof that a missing control
 * does not abort the rest of `main()`.
 */
function mountFixture(): void {
  document.body.innerHTML = `
    <div id="banner" class="banner" hidden></div>
    <table><tbody id="rows"></tbody></table>
    <p id="empty" class="empty" hidden></p>
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
}

type MainModule = typeof import("./main");

let main: MainModule;
/** The handler `main()` handed to `onSiteStatus` — the only way into the
 *  module-private `statuses` map, and the same path a real event takes. */
let emitStatus: (event: StatusEvent) => void;

beforeAll(async () => {
  mountFixture();
  // Imported once, not per test: `vi.resetModules()` would hand `main.ts` a
  // fresh copy of the mocked `./api`, and the handler captured below would then
  // belong to the wrong instance. Tests use distinct site ids instead.
  main = await import("./main");
  await flush();
  emitStatus = vi.mocked(onSiteStatus).mock.calls[0][0];
});

describe("startup", () => {
  it("registers both listeners before awaiting anything else", () => {
    expect(onSiteStatus).toHaveBeenCalledTimes(1);
    expect(emitStatus).toBeTypeOf("function");

    // This is US1's actual property, and it needs call *order*, not just call
    // count: a Tauri event emitted before `listen` resolves has nowhere to
    // land and is never replayed. Asserting both registrations precede every
    // startup IPC call is what would catch someone moving them back down.
    const registered = Math.max(
      vi.mocked(onSiteStatus).mock.invocationCallOrder[0],
      vi.mocked(onStoreWarning).mock.invocationCallOrder[0],
    );
    // `getAutostart` is absent from this list on purpose: the fixture has no
    // `#autostart`, so US5's guard returns before it is ever called.
    const firstStartupCall = Math.min(
      vi.mocked(listSites).mock.invocationCallOrder[0],
      vi.mocked(getWarning).mock.invocationCallOrder[0],
    );

    expect(registered).toBeLessThan(firstStartupCall);
  });

  it("survives a missing #autostart control, reporting it in the banner", () => {
    const banner = document.querySelector<HTMLElement>("#banner")!;

    expect(banner.hidden).toBe(false);
    expect(banner.textContent).toBe("The autostart control is missing from the page.");
    // The real assertion is that `main()` got past `mountAutostart` at all:
    // the form is mounted and the status listener registered, both of which
    // happen either side of it.
    expect(document.querySelector("#site-submit")).not.toBeNull();
    expect(onSiteStatus).toHaveBeenCalled();
  });

  it("keeps a status that arrives before its site is known", () => {
    // The consequence of US1's reorder: an event can beat `listSites()`. It
    // must survive in `statuses` and surface once the site appears, because
    // `currentRows()` iterates `sites`.
    emitStatus(status("early"));
    expect(main.currentRows().find((r) => r.site.id === "early")).toBeUndefined();

    main.upsertSite(site("early"));
    expect(main.currentRows().find((r) => r.site.id === "early")?.status?.state).toBe("up");
  });
});

describe("upsertSite drops a stale status only when the URL changes", () => {
  function statusOf(id: string) {
    return main.currentRows().find((row) => row.site.id === id)?.status ?? null;
  }

  it("resets the row to Pending when the URL is edited", () => {
    main.upsertSite(site("a", { url: "https://a.example.com" }));
    emitStatus(status("a"));
    expect(statusOf("a")?.state).toBe("up");

    main.upsertSite(site("a", { url: "https://moved.example.com" }));

    // Pending is `status === null`; the old dot described the old URL.
    expect(statusOf("a")).toBeNull();
  });

  it("keeps the status when only the label changes", () => {
    main.upsertSite(site("b", { url: "https://b.example.com" }));
    emitStatus(status("b"));

    main.upsertSite(site("b", { url: "https://b.example.com", label: "Renamed" }));

    expect(statusOf("b")?.state).toBe("up");
  });

  it("keeps the status when only the interval changes", () => {
    main.upsertSite(site("c", { url: "https://c.example.com", interval_secs: 30 }));
    emitStatus(status("c"));

    main.upsertSite(site("c", { url: "https://c.example.com", interval_secs: 600 }));

    expect(statusOf("c")?.state).toBe("up");
  });

  it("has nothing to drop when the site is new", () => {
    main.upsertSite(site("d"));

    expect(statusOf("d")).toBeNull();
    expect(main.currentRows().some((row) => row.site.id === "d")).toBe(true);
  });
});
