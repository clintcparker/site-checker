import { beforeEach, describe, expect, it, vi } from "vitest";
import INDEX_HTML from "../index.html?raw";
import {
  addSite,
  deleteSite,
  getVersion,
  listSites,
  openUrl,
  updateSite,
} from "./api";
import { mountAbout } from "./about";

// The same convention `open.test.ts` and `form.test.ts` use. `about.ts` is a
// shell over two calls — one to open an address, one to read a version string.
//
// Every store command is mocked too, unused on purpose: T9 asserts that none of
// them is reached, and a name that is not mocked could not be asserted about.
vi.mock("./api", () => ({
  openUrl: vi.fn(() => Promise.resolve()),
  getVersion: vi.fn(() => Promise.resolve("1.2.3")),
  listSites: vi.fn(() => Promise.resolve([])),
  addSite: vi.fn(),
  updateSite: vi.fn(),
  deleteSite: vi.fn(),
}));

const SITE_URL = "https://clintparker.com";
const NOW = 1_700_000_000_000;

/**
 * The About markup is lifted out of the shipped `index.html` rather than
 * hand-typed here.
 *
 * `open.test.ts` builds its rows with the real `render.ts` for this reason: a
 * fixture that restates the markup asserts only that the fixture agrees with
 * itself. T5 in particular — that the link carries
 * `data-open-url="https://clintparker.com"` exactly — proves nothing unless the
 * string under test is the one that ships.
 *
 * `?raw` is Vite's own suffix, typed by `src/vite-env.d.ts`; it needs no Node
 * types and so no new dependency.
 */
function extract(pattern: RegExp, what: string): string {
  const match = INDEX_HTML.match(pattern);
  if (!match) throw new Error(`index.html no longer contains ${what}`);
  return match[0];
}

const DIALOG_MARKUP = extract(/<dialog id="about"[\s\S]*?<\/dialog>/, '<dialog id="about">');
const OPENER_MARKUP = extract(
  /<button[^>]*id="about-open"[^>]*>[\s\S]*?<\/button>/,
  "#about-open",
);

interface Mounted {
  dialog: HTMLDialogElement;
  opener: HTMLElement;
  closer: HTMLElement;
  link: HTMLElement;
}

/** Builds the About surface as `index.html` ships it and mounts the module on
 *  it. `now` is injected so the activation window can be driven, exactly as
 *  `open.test.ts` drives `mountUrlOpener`'s. */
function mount(now: () => number = () => NOW): Mounted {
  document.body.innerHTML = `
    <div id="banner" class="banner" hidden></div>
    ${OPENER_MARKUP}
    ${DIALOG_MARKUP}
  `;

  const dialog = document.querySelector<HTMLDialogElement>("#about")!;

  // The hook records whether the dialog was still open *at the moment it was
  // called*, not afterwards. T8's requirement is an ordering one — a message
  // written while a modal still covers it is not visible — and only a reading
  // taken inside the callback can tell the two orderings apart.
  mountAbout({
    onError: (message) => errors.push({ message, dialogWasOpen: dialog.open }),
    now,
  });

  return {
    dialog,
    opener: document.querySelector<HTMLElement>("#about-open")!,
    closer: dialog.querySelector<HTMLElement>("[data-about-close]")!,
    link: dialog.querySelector<HTMLElement>("[data-open-url]")!,
  };
}

function flush(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

interface RecordedError {
  message: string;
  dialogWasOpen: boolean;
}

let errors: RecordedError[];

beforeEach(() => {
  vi.mocked(openUrl).mockReset();
  vi.mocked(openUrl).mockResolvedValue(undefined);
  vi.mocked(getVersion).mockReset();
  vi.mocked(getVersion).mockResolvedValue("1.2.3");
  vi.mocked(listSites).mockClear();
  vi.mocked(addSite).mockClear();
  vi.mocked(updateSite).mockClear();
  vi.mocked(deleteSite).mockClear();
  errors = [];
});

describe("the About surface", () => {
  // T1 — FR-001
  it("opens when the footer control is activated", async () => {
    const { dialog, opener } = mount();
    expect(dialog.open).toBe(false);

    opener.click();
    await flush();

    expect(dialog.open).toBe(true);
  });

  // T1 — the other half of FR-001: it has to close again.
  it("closes when the dismissal control is activated", async () => {
    const { dialog, opener, closer } = mount();

    opener.click();
    await flush();
    closer.click();

    expect(dialog.open).toBe(false);
  });

  // T2 — FR-004, FR-002
  it("names the app and attributes it to Clint Parker", async () => {
    const { dialog, opener } = mount();
    opener.click();
    await flush();

    const text = dialog.textContent ?? "";
    // The exact spelling and capitalisation is the requirement, not a
    // paraphrase of it.
    expect(text).toContain("Created by Clint Parker");
    expect(text).toContain("Site Checker");
  });

  // T2 — FR-004's negative half, from the contract's "what must NOT appear".
  it("surfaces no email address, handle, or username", async () => {
    const { dialog, opener } = mount();
    opener.click();
    await flush();

    expect(dialog.textContent ?? "").not.toMatch(/@/);
  });

  // T9 — FR-007, SC-006
  it("reaches no store command when opened and closed", async () => {
    const { opener, closer } = mount();

    opener.click();
    await flush();
    closer.click();
    await flush();

    // The dialog is rendered from constants and one version string. Anything
    // that could add, remove, re-schedule or re-check a site lives behind one
    // of these four calls, and none of them is reached.
    expect(listSites).not.toHaveBeenCalled();
    expect(addSite).not.toHaveBeenCalled();
    expect(updateSite).not.toHaveBeenCalled();
    expect(deleteSite).not.toHaveBeenCalled();
    expect(openUrl).not.toHaveBeenCalled();
  });
});

describe("the version line", () => {
  // T3 — FR-003
  it("renders what getVersion resolved with, verbatim", async () => {
    vi.mocked(getVersion).mockResolvedValue("0.0.0");
    const { dialog, opener } = mount();

    opener.click();
    await flush();

    // `0.0.0` is the local-build sentinel and is shown as-is. Nothing here
    // parses, prettifies, or hides a version.
    expect(dialog.textContent ?? "").toContain("0.0.0");
  });

  it("renders an unusual version string without interpreting it", async () => {
    vi.mocked(getVersion).mockResolvedValue("1.4.0-beta.2+build.77");
    const { dialog, opener } = mount();

    opener.click();
    await flush();

    expect(dialog.textContent ?? "").toContain("1.4.0-beta.2+build.77");
  });

  // T4 — FR-003, research R-005
  it("degrades to 'Version unavailable' without blocking the open or raising a banner", async () => {
    vi.mocked(getVersion).mockRejectedValue("the version could not be read");
    const { dialog, opener, link } = mount();

    opener.click();
    await flush();

    // A missing version stamp is not something the user can act on, so the
    // dialog still opens and still carries everything it was opened for.
    expect(dialog.open).toBe(true);
    const text = dialog.textContent ?? "";
    expect(text).toContain("Version unavailable");
    expect(text).toContain("Created by Clint Parker");
    expect(link).not.toBeNull();

    // Explicitly not a banner: this is the whole of R-005's decision.
    expect(errors).toEqual([]);
    expect(document.querySelector<HTMLElement>("#banner")!.hidden).toBe(true);
  });
});

describe("the clintparker.com link", () => {
  // T5 — FR-005
  it("carries the exact address in its data-open-url attribute", () => {
    const { link } = mount();

    // Secure scheme, apex domain, no path, no trailing slash. This is the
    // string `openable_url` returns byte-identical, so it is the string that
    // opens.
    expect(link.dataset.openUrl).toBe(SITE_URL);
  });

  // T5 — the contract's "what must NOT appear".
  it("is a button, not an anchor", () => {
    const { dialog, link } = mount();

    // An anchor can navigate the dashboard away from itself if the handler
    // does not run, and there is no way back — the reasoning already recorded
    // at render.ts:184-187.
    expect(link.tagName).toBe("BUTTON");
    expect(dialog.querySelector("a[href]")).toBeNull();
    expect(dialog.querySelector("[target]")).toBeNull();
  });

  // T5 — the visible text names the destination without the scheme.
  it("reads as clintparker.com", () => {
    const { link } = mount();
    expect(link.textContent?.trim()).toBe("clintparker.com");
  });

  // T6 — FR-006
  it("hands the address to the backend once when activated", async () => {
    const { opener, link } = mount();
    opener.click();
    await flush();

    link.click();
    await flush();

    expect(openUrl).toHaveBeenCalledTimes(1);
    // From the attribute, never from textContent — the rendered text omits the
    // scheme and would not open.
    expect(openUrl).toHaveBeenCalledWith(SITE_URL);
  });

  // T7 — FR-008, SC-004
  it("collapses ten activations inside one second into a single open", async () => {
    let now = NOW;
    const { opener, link } = mount(() => now);
    opener.click();
    await flush();

    // Driven through the injected clock rather than by waiting out a real
    // second, the seam open.ts already provides for exactly this.
    for (let i = 0; i < 10; i += 1) {
      now = NOW + i * 90; // ten activations spanning 810ms — inside the window
      link.click();
    }
    await flush();

    expect(openUrl).toHaveBeenCalledTimes(1);
  });

  it("opens again once the window has passed", async () => {
    let now = NOW;
    const { opener, link } = mount(() => now);
    opener.click();
    await flush();

    link.click();
    now += 1000;
    link.click();
    await flush();

    // Suppression is impatience-handling, not a permanent lockout.
    expect(openUrl).toHaveBeenCalledTimes(2);
  });

  // T8 — FR-009, SC-005
  it("closes the dialog before putting a refusal in the banner", async () => {
    vi.mocked(openUrl).mockRejectedValue(
      "macOS would not open https://clintparker.com.",
    );
    const { dialog, opener, link } = mount();
    opener.click();
    await flush();
    expect(dialog.open).toBe(true);

    link.click();
    await flush();

    expect(errors).toHaveLength(1);
    // The assertion with teeth. Writing the message behind a modal satisfies
    // the letter of FR-009 and fails its intent: the user cannot see it.
    expect(errors[0].dialogWasOpen).toBe(false);
    // Rust `Err(String)` arrives as the bare string — no `.message` to unwrap,
    // and nothing prepended to it here.
    expect(errors[0].message).toBe("macOS would not open https://clintparker.com.");
    expect(dialog.open).toBe(false);
  });

  it("stays usable after a refusal", async () => {
    let now = NOW;
    const { dialog, opener, link } = mount(() => now);
    opener.click();
    await flush();

    vi.mocked(openUrl).mockRejectedValueOnce("macOS would not open it.");
    link.click();
    await flush();
    expect(errors).toHaveLength(1);

    // Nothing was disabled and nothing was mutated on the way to the failure,
    // so reopening and trying again is an ordinary path.
    opener.click();
    now += 1000;
    link.click();
    await flush();

    expect(dialog.open).toBe(true);
    expect(openUrl).toHaveBeenCalledTimes(2);
    expect(errors).toHaveLength(1);
  });
});
