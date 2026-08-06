import { beforeEach, describe, expect, it, vi } from "vitest";
import { addSite, deleteSite, updateSite, type Site } from "./api";
import { mountForm } from "./form";

// `form.ts` is the thin shell over these three commands. Stubbing the module is
// what lets a DOM test hold a call open and drive the in-flight window without a
// Tauri backend behind it. Hoisted above the imports by vitest, so the bindings
// imported above are already the mocks.
vi.mock("./api", () => ({
  addSite: vi.fn(),
  updateSite: vi.fn(),
  deleteSite: vi.fn(),
}));

interface Deferred<T> {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (reason: unknown) => void;
}

/** A promise whose settling this test controls, so the window between "the
 *  command was called" and "the command came back" can be inspected. */
function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

/** Lets every already-queued microtask and the awaits chained off it run. */
function flush(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

/** The form markup `mountForm` queries for, ids and types copied from
 *  `index.html`. Kept a literal copy rather than a shared fixture module,
 *  matching `render.test.ts`'s local-fixture style. */
function mountFixture(): void {
  document.body.innerHTML = `
    <table><tbody id="rows"></tbody></table>
    <form id="site-form" class="site-form" novalidate>
      <input type="hidden" id="site-id" />
      <input id="site-url" type="text" placeholder="example.com" required />
      <input id="site-label" type="text" placeholder="Label (optional)" />
      <input id="site-interval" type="number" min="10" max="86400" step="1" value="60" />
      <button type="submit" id="site-submit">Add</button>
      <button type="button" id="site-cancel" hidden>Cancel</button>
      <p id="site-error" class="form-error" hidden></p>
    </form>
  `;
}

function el<T extends HTMLElement>(selector: string): T {
  return document.querySelector<T>(selector)!;
}

const form = () => el<HTMLFormElement>("#site-form");
const urlField = () => el<HTMLInputElement>("#site-url");
const intervalField = () => el<HTMLInputElement>("#site-interval");
const idField = () => el<HTMLInputElement>("#site-id");
const submitButton = () => el<HTMLButtonElement>("#site-submit");
const cancelButton = () => el<HTMLButtonElement>("#site-cancel");
const errorLine = () => el<HTMLElement>("#site-error");
const tbody = () => el<HTMLElement>("#rows");

function site(id: string, overrides: Partial<Site> = {}): Site {
  return {
    id,
    url: `https://${id}.example.com`,
    method_override: null,
    interval_secs: 30,
    ...overrides,
  };
}

function mount(lookup: (id: string) => Site | undefined = () => undefined) {
  const onSaved = vi.fn();
  const onDeleted = vi.fn();
  mountForm({ onSaved, onDeleted, lookup });
  return { onSaved, onDeleted };
}

function submitForm(): void {
  form().dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
}

/** A row's Delete button, as `render.ts` builds it — the click listener is
 *  delegated on `#rows`, so the test only needs the button's shape. */
function addDeleteButton(id: string): HTMLButtonElement {
  const tr = document.createElement("tr");
  tr.dataset.id = id;
  const button = document.createElement("button");
  button.className = "row-action row-action-delete";
  button.dataset.action = "delete";
  button.dataset.id = id;
  button.textContent = "Delete";
  tr.append(button);
  tbody().append(tr);
  return button;
}

beforeEach(() => {
  // `reset`, not `clear` — each test installs its own stub behaviour and must
  // not inherit the previous one's.
  vi.resetAllMocks();
  mountFixture();
});

describe("mountForm", () => {
  it("starts in Add mode", () => {
    mount();

    expect(submitButton().textContent).toBe("Add");
    expect(cancelButton().hidden).toBe(true);
    expect(intervalField().value).toBe("60");
  });
});

describe("submit is guarded while a save is in flight", () => {
  it("fires exactly one addSite for two submits in the same tick", async () => {
    const pending = deferred<Site>();
    vi.mocked(addSite).mockReturnValue(pending.promise);
    mount();

    urlField().value = "example.com";
    submitForm();
    submitForm();

    expect(addSite).toHaveBeenCalledTimes(1);
    expect(submitButton().disabled).toBe(true);

    pending.resolve(site("a"));
    await flush();

    expect(addSite).toHaveBeenCalledTimes(1);
    expect(submitButton().disabled).toBe(false);
  });

  it("routes an edit through updateSite and guards it the same way", async () => {
    const existing = site("a");
    const pending = deferred<Site>();
    vi.mocked(updateSite).mockReturnValue(pending.promise);
    const { onSaved } = mount(() => existing);

    idField().value = existing.id;
    urlField().value = "https://moved.example.com";
    submitForm();
    submitForm();

    expect(updateSite).toHaveBeenCalledTimes(1);
    expect(addSite).not.toHaveBeenCalled();
    expect(submitButton().disabled).toBe(true);

    pending.resolve(site("a", { url: "https://moved.example.com" }));
    await flush();

    expect(onSaved).toHaveBeenCalledTimes(1);
    expect(submitButton().disabled).toBe(false);
  });

  it("re-enables and shows the error when the save fails, so a retry is possible", async () => {
    const pending = deferred<Site>();
    vi.mocked(addSite).mockReturnValue(pending.promise);
    mount();

    urlField().value = "nope";
    submitForm();
    expect(submitButton().disabled).toBe(true);

    pending.reject("Enter a valid http(s) URL");
    await flush();

    expect(submitButton().disabled).toBe(false);
    expect(errorLine().hidden).toBe(false);
    expect(errorLine().textContent).toBe("Enter a valid http(s) URL");
  });
});

describe("the interval is clamped at both ends before it reaches the backend", () => {
  // Hardcoded rather than imported: this pins the ceiling, so changing
  // `form.ts`'s constant without meaning to shows up here as a failure.
  const MAX_INTERVAL = 86_400;

  const CASES: Array<[string, string, number]> = [
    ["below the floor", "5", 10],
    ["in range", "60", 60],
    ["above the ceiling", "999999999", MAX_INTERVAL],
    ["empty", "", 60],
    ["non-numeric", "abc", 60],
  ];

  it.each(CASES)("sends %s as %s -> %d", async (_label, typed, expected) => {
    vi.mocked(addSite).mockResolvedValue(site("a"));
    mount();

    urlField().value = "example.com";
    intervalField().value = typed;
    submitForm();
    await flush();

    expect(addSite).toHaveBeenCalledWith("example.com", null, expected);
  });
});

describe("delete is guarded while a delete is in flight", () => {
  it("fires exactly one deleteSite for two clicks in the same tick", async () => {
    const pending = deferred<void>();
    vi.mocked(deleteSite).mockReturnValue(pending.promise);
    const { onDeleted } = mount();
    const button = addDeleteButton("a");

    button.click();
    button.click();

    expect(deleteSite).toHaveBeenCalledTimes(1);
    expect(button.disabled).toBe(true);

    pending.resolve();
    await flush();

    expect(deleteSite).toHaveBeenCalledTimes(1);
    expect(onDeleted).toHaveBeenCalledTimes(1);
    expect(onDeleted).toHaveBeenCalledWith("a");
  });

  it("re-enables the row's Delete button when the delete fails", async () => {
    const pending = deferred<void>();
    vi.mocked(deleteSite).mockReturnValue(pending.promise);
    const { onDeleted } = mount();
    const button = addDeleteButton("a");

    button.click();
    expect(button.disabled).toBe(true);

    pending.reject("sites.json is read-only");
    await flush();

    expect(button.disabled).toBe(false);
    expect(onDeleted).not.toHaveBeenCalled();
    expect(errorLine().textContent).toBe("sites.json is read-only");
  });
});
