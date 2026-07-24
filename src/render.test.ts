import { describe, expect, it } from "vitest";
import type { Site, StatusEvent } from "./api";
import { renderTable, type Row } from "./render";

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
