import { describe, expect, it } from "vitest";
import type { Site, StatusEvent } from "./api";
import { isOpenable, renderTable, type Row } from "./render";

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

function status(overrides: Partial<StatusEvent>): StatusEvent {
  return {
    id: "a",
    state: "up",
    checked_at: NOW,
    reason: null,
    ...overrides,
  };
}

function makeTbody(): HTMLElement {
  return document.createElement("tbody");
}

function trs(tbody: HTMLElement): HTMLTableRowElement[] {
  return Array.from(tbody.querySelectorAll("tr"));
}

function dotOf(tr: HTMLTableRowElement): HTMLElement {
  return tr.children[1].children[0] as HTMLElement;
}

function labelOf(tr: HTMLTableRowElement): HTMLElement {
  return tr.children[1].children[1] as HTMLElement;
}

function sinceOf(tr: HTMLTableRowElement): HTMLElement {
  return tr.children[2] as HTMLElement;
}

function nameOf(tr: HTMLTableRowElement): HTMLElement {
  return tr.children[0] as HTMLElement;
}

/** The node carrying the address, whichever slot it is in and whether or not
 *  it turned out to be a control. */
function urlOf(tr: HTMLTableRowElement): HTMLElement {
  const name = nameOf(tr);
  return name.children[name.children.length - 1] as HTMLElement;
}

describe("renderTable reconciliation", () => {
  it("preserves element identity across a repaint with only `now` advanced", () => {
    const tbody = makeTbody();
    const rows: Row[] = [{ site: site("a"), status: null }];

    renderTable(tbody, rows, NOW);
    const trBefore = trs(tbody)[0];
    const dotBefore = dotOf(trBefore);

    renderTable(tbody, rows, NOW + 5_000);
    const trAfter = trs(tbody)[0];
    const dotAfter = dotOf(trAfter);

    expect(trAfter).toBe(trBefore);
    expect(dotAfter).toBe(dotBefore);
  });

  it("updates the since cell's text in place on the preserved element", () => {
    const tbody = makeTbody();
    const rows: Row[] = [
      { site: site("a"), status: status({ id: "a", checked_at: NOW }) },
    ];

    renderTable(tbody, rows, NOW);
    const tr = trs(tbody)[0];
    expect(sinceOf(tr).textContent).toBe("0s ago");

    renderTable(tbody, rows, NOW + 5_000);
    expect(trs(tbody)[0]).toBe(tr);
    expect(sinceOf(tr).textContent).toBe("5s ago");
  });

  it("updates the dot, label, and title through a Pending -> Up -> Down transition", () => {
    const tbody = makeTbody();
    const id = "a";

    renderTable(tbody, [{ site: site(id), status: null }], NOW);
    const tr = trs(tbody)[0];
    const dot = dotOf(tr);
    const label = labelOf(tr);

    expect(label.textContent).toBe("Pending");
    expect(dot.className).toBe("dot dot-pending");
    expect(dot.hasAttribute("title")).toBe(false);

    renderTable(
      tbody,
      [{ site: site(id), status: status({ id, state: "up", reason: null }) }],
      NOW,
    );
    expect(trs(tbody)[0]).toBe(tr);
    expect(dotOf(tr)).toBe(dot);
    expect(label.textContent).toBe("Up");
    expect(dot.className).toBe("dot dot-up");
    expect(dot.hasAttribute("title")).toBe(false);

    renderTable(
      tbody,
      [
        {
          site: site(id),
          status: status({ id, state: "down", reason: "HTTP 404" }),
        },
      ],
      NOW,
    );
    expect(trs(tbody)[0]).toBe(tr);
    expect(label.textContent).toBe("Down");
    expect(dot.className).toBe("dot dot-down");
    expect(dot.title).toBe("HTTP 404");
  });

  it("clears a stale tooltip once the reason is gone, rather than leaving it showing", () => {
    const tbody = makeTbody();
    const id = "a";

    renderTable(
      tbody,
      [
        {
          site: site(id),
          status: status({ id, state: "down", reason: "Could not connect" }),
        },
      ],
      NOW,
    );
    const dot = dotOf(trs(tbody)[0]);
    expect(dot.title).toBe("Could not connect");

    renderTable(
      tbody,
      [{ site: site(id), status: status({ id, state: "up", reason: null }) }],
      NOW,
    );
    expect(dot.hasAttribute("title")).toBe(false);
    expect(dot.title).toBe("");
  });

  it("appends a row for a newly added site without disturbing existing rows", () => {
    const tbody = makeTbody();
    renderTable(tbody, [{ site: site("a"), status: null }], NOW);
    const trA = trs(tbody)[0];

    renderTable(
      tbody,
      [
        { site: site("a"), status: null },
        { site: site("b"), status: null },
      ],
      NOW,
    );

    const rows = trs(tbody);
    expect(rows).toHaveLength(2);
    expect(rows[0]).toBe(trA);
    expect(rows[1].dataset.id).toBe("b");
  });

  it("removes exactly the row for a deleted site and leaves the rest intact", () => {
    const tbody = makeTbody();
    renderTable(
      tbody,
      [
        { site: site("a"), status: null },
        { site: site("b"), status: null },
        { site: site("c"), status: null },
      ],
      NOW,
    );
    const [trA, , trC] = trs(tbody);

    renderTable(
      tbody,
      [
        { site: site("a"), status: null },
        { site: site("c"), status: null },
      ],
      NOW,
    );

    const rows = trs(tbody);
    expect(rows).toHaveLength(2);
    expect(rows.map((tr) => tr.dataset.id)).toEqual(["a", "c"]);
    expect(rows[0]).toBe(trA);
    expect(rows[1]).toBe(trC);
  });

  // The two tests below are the ends of the range. The seven above cover the
  // middle — identity, in-place cell updates, transitions, append, single
  // removal, reorder — and each of these is a case where a diffing loop can be
  // subtly wrong in a way none of the middle cases reach.

  it("clears a populated tbody when the last site is deleted", () => {
    const tbody = makeTbody();
    renderTable(
      tbody,
      [
        { site: site("a"), status: null },
        { site: site("b"), status: null },
      ],
      NOW,
    );
    expect(trs(tbody)).toHaveLength(2);

    renderTable(tbody, [], NOW);

    // Deleting the last site is the one case where the desired end state is
    // "nothing" — a loop that only ever walks the incoming rows has nothing to
    // walk and leaves the old rows on screen.
    expect(trs(tbody)).toHaveLength(0);
    expect(tbody.children).toHaveLength(0);
  });

  it("inserts a new row at the front rather than appending it", () => {
    const tbody = makeTbody();
    renderTable(tbody, [{ site: site("b"), status: null }], NOW);
    const trB = trs(tbody)[0];

    renderTable(
      tbody,
      [
        { site: site("a"), status: null },
        { site: site("b"), status: null },
      ],
      NOW,
    );

    // The append test proves a new row can arrive last. This proves position is
    // actually honoured: a diff that appends unconditionally passes that one and
    // fails this one.
    const rows = trs(tbody);
    expect(rows.map((tr) => tr.dataset.id)).toEqual(["a", "b"]);
    expect(rows[1]).toBe(trB);
  });

  it("reorders existing rows to match the new row order, moving rather than recreating", () => {
    const tbody = makeTbody();
    renderTable(
      tbody,
      [
        { site: site("a"), status: null },
        { site: site("b"), status: null },
        { site: site("c"), status: null },
      ],
      NOW,
    );
    const [trA, trB, trC] = trs(tbody);

    renderTable(
      tbody,
      [
        { site: site("c"), status: null },
        { site: site("a"), status: null },
        { site: site("b"), status: null },
      ],
      NOW,
    );

    const rows = trs(tbody);
    expect(rows.map((tr) => tr.dataset.id)).toEqual(["c", "a", "b"]);
    expect(rows[0]).toBe(trC);
    expect(rows[1]).toBe(trA);
    expect(rows[2]).toBe(trB);
  });
});

describe("isOpenable", () => {
  it("accepts http and https regardless of case", () => {
    expect(isOpenable("https://example.com")).toBe(true);
    expect(isOpenable("http://example.com/health?q=A")).toBe(true);
    expect(isOpenable("HTTPS://example.com")).toBe(true);
    expect(isOpenable("  https://example.com  ")).toBe(true);
  });

  it("refuses every other scheme, and refuses a scheme-less address rather than repairing it", () => {
    expect(isOpenable("ftp://example.com")).toBe(false);
    expect(isOpenable("file:///etc/hosts")).toBe(false);
    expect(isOpenable("javascript:alert(1)")).toBe(false);
    // The frontend half of the rule must refuse this for the same reason the
    // backend does: prepending a scheme would offer to open an address nobody
    // stored.
    expect(isOpenable("example.com")).toBe(false);
    expect(isOpenable("")).toBe(false);
    expect(isOpenable("   ")).toBe(false);
  });

  it("is not fooled by a scheme that only appears later in the string", () => {
    expect(isOpenable("example.com?next=https://x.dev")).toBe(false);
  });
});

describe("the row's URL control", () => {
  it("renders an unlabelled site's URL as the primary line, as a button", () => {
    const tbody = makeTbody();
    renderTable(tbody, [{ site: site("a", { url: "https://a.dev" }), status: null }], NOW);

    const url = urlOf(trs(tbody)[0]);
    expect(url.tagName).toBe("BUTTON");
    expect((url as HTMLButtonElement).type).toBe("button");
    expect(url.className).toBe("site-primary site-url");
    expect(url.dataset.openUrl).toBe("https://a.dev");
    expect(url.textContent).toBe("https://a.dev");
    expect(nameOf(trs(tbody)[0]).children).toHaveLength(1);
  });

  it("renders a labelled site's label as inert text and its URL as the secondary line", () => {
    const tbody = makeTbody();
    renderTable(
      tbody,
      [{ site: site("a", { url: "https://a.dev", label: "Production" }), status: null }],
      NOW,
    );

    const name = nameOf(trs(tbody)[0]);
    expect(name.children).toHaveLength(2);

    const label = name.children[0] as HTMLElement;
    expect(label.tagName).toBe("SPAN");
    expect(label.className).toBe("site-primary");
    expect(label.textContent).toBe("Production");
    // The label is not a control: it carries nothing either listener matches.
    expect(label.hasAttribute("data-open-url")).toBe(false);
    expect(label.hasAttribute("data-action")).toBe(false);

    const url = name.children[1] as HTMLElement;
    expect(url.tagName).toBe("BUTTON");
    expect(url.className).toBe("site-secondary site-url");
    expect(url.dataset.openUrl).toBe("https://a.dev");
  });

  it("carries the full address in the attribute however much of it is rendered", () => {
    const long = `https://example.com/${"segment/".repeat(40)}end?q=1`;
    const tbody = makeTbody();
    renderTable(tbody, [{ site: site("a", { url: long }), status: null }], NOW);

    // The attribute is the address, not the text. This is what makes any
    // future truncation or wrapping of the visible line irrelevant to what
    // actually opens.
    expect(urlOf(trs(tbody)[0]).dataset.openUrl).toBe(long);
  });

  it("keys the control on data-open-url, which the row-action listener cannot match", () => {
    const tbody = makeTbody();
    renderTable(tbody, [{ site: site("a"), status: null }], NOW);
    const url = urlOf(trs(tbody)[0]);

    // `form.ts` matches `.closest("[data-action]")`. If the URL ever carried
    // that attribute, activating it would also reach Edit or Delete — so this
    // is the assertion that keeps those two paths structurally apart.
    expect(url.hasAttribute("data-action")).toBe(false);
    expect(url.closest("[data-action]")).toBeNull();
  });

  it("puts the URL in the tab order natively, with no tabindex of its own", () => {
    const tbody = makeTbody();
    renderTable(tbody, [{ site: site("a"), status: null }], NOW);

    // A <button> is focusable already. An explicit value here would be a
    // regression waiting to happen — it can only take the control out of
    // document order or duplicate what it already has.
    expect(urlOf(trs(tbody)[0]).hasAttribute("tabindex")).toBe(false);
  });

  it("shows an address it will not open as plain text, offering nothing", () => {
    const tbody = makeTbody();
    renderTable(tbody, [{ site: site("a", { url: "ftp://example.com" }), status: null }], NOW);

    const url = urlOf(trs(tbody)[0]);
    // Shown, not hidden and not flagged as invalid — just not presented as
    // something that can be opened.
    expect(url.tagName).toBe("SPAN");
    expect(url.textContent).toBe("ftp://example.com");
    expect(url.className).toBe("site-primary");
    expect(url.hasAttribute("data-open-url")).toBe(false);
    expect(url.hasAttribute("tabindex")).toBe(false);
  });
});

describe("the URL control survives reconciliation", () => {
  it("preserves the control's element identity across a repaint with only `now` advanced", () => {
    const tbody = makeTbody();
    const rows: Row[] = [{ site: site("a"), status: null }];

    renderTable(tbody, rows, NOW);
    const before = urlOf(trs(tbody)[0]);

    renderTable(tbody, rows, NOW + 5_000);

    // Replacing this node would drop keyboard focus mid-interaction, once a
    // second, forever.
    expect(urlOf(trs(tbody)[0])).toBe(before);
  });

  it("preserves the control's element identity when a status event lands", () => {
    const tbody = makeTbody();
    const id = "a";

    renderTable(tbody, [{ site: site(id), status: null }], NOW);
    const before = urlOf(trs(tbody)[0]);

    renderTable(
      tbody,
      [{ site: site(id), status: status({ id, state: "down", reason: "HTTP 500" }) }],
      NOW,
    );

    expect(urlOf(trs(tbody)[0])).toBe(before);
    expect(urlOf(trs(tbody)[0]).dataset.openUrl).toBe(`https://${id}.example.com`);
  });

  it("updates the address in place when only the URL is edited", () => {
    const tbody = makeTbody();
    renderTable(tbody, [{ site: site("a", { url: "https://one.dev" }), status: null }], NOW);
    const before = urlOf(trs(tbody)[0]);

    renderTable(tbody, [{ site: site("a", { url: "https://two.dev" }), status: null }], NOW);

    const after = urlOf(trs(tbody)[0]);
    expect(after).toBe(before);
    expect(after.textContent).toBe("https://two.dev");
    expect(after.dataset.openUrl).toBe("https://two.dev");
  });

  it("rebuilds the cell when a label is added, moving the URL to the secondary slot", () => {
    const tbody = makeTbody();
    renderTable(tbody, [{ site: site("a", { url: "https://a.dev" }), status: null }], NOW);
    const name = nameOf(trs(tbody)[0]);

    renderTable(
      tbody,
      [{ site: site("a", { url: "https://a.dev", label: "Prod" }), status: null }],
      NOW,
    );

    // The cell itself is preserved; only its two children are rebuilt, because
    // the URL changed which slot it occupies.
    expect(nameOf(trs(tbody)[0])).toBe(name);
    expect(name.children).toHaveLength(2);
    expect((name.children[0] as HTMLElement).textContent).toBe("Prod");
    const url = name.children[1] as HTMLElement;
    expect(url.className).toBe("site-secondary site-url");
    expect(url.dataset.openUrl).toBe("https://a.dev");
  });

  it("rebuilds the cell when a label is removed, moving the URL back to the primary slot", () => {
    const tbody = makeTbody();
    renderTable(
      tbody,
      [{ site: site("a", { url: "https://a.dev", label: "Prod" }), status: null }],
      NOW,
    );

    renderTable(tbody, [{ site: site("a", { url: "https://a.dev" }), status: null }], NOW);

    const name = nameOf(trs(tbody)[0]);
    expect(name.children).toHaveLength(1);
    const url = name.children[0] as HTMLElement;
    expect(url.tagName).toBe("BUTTON");
    expect(url.className).toBe("site-primary site-url");
  });

  it("replaces the node when an address that could not be opened becomes one that can", () => {
    const tbody = makeTbody();
    renderTable(tbody, [{ site: site("a", { url: "ftp://a.dev" }), status: null }], NOW);
    const before = urlOf(trs(tbody)[0]);
    expect(before.tagName).toBe("SPAN");

    renderTable(tbody, [{ site: site("a", { url: "https://a.dev" }), status: null }], NOW);

    // Element type cannot be mutated, so this is the one in-slot case where the
    // node is genuinely replaced rather than updated.
    const after = urlOf(trs(tbody)[0]);
    expect(after).not.toBe(before);
    expect(after.tagName).toBe("BUTTON");
    expect(after.className).toBe("site-primary site-url");
    expect(after.dataset.openUrl).toBe("https://a.dev");
  });
});
