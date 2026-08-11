/// <reference types="vite/client" />
import { describe, expect, it } from "vitest";
import { MAX_INTERVAL, MIN_INTERVAL } from "./form";

/**
 * The interval floor and ceiling are written down in more than one place, and
 * nothing used to notice when the copies drifted apart.
 *
 * `form.ts` owns the values, but `index.html`'s `min`/`max` attributes are what
 * the *browser* enforces before a keystroke ever reaches `form.ts` — so a stale
 * attribute silently changes the product's behaviour while every existing test
 * still passes. The fixture DOM inside each test file is a third and fourth
 * copy, hand-typed rather than loaded from the real document, so those drift
 * silently too: a test asserting the ceiling clamps at 86400 proves nothing once
 * its own fixture says something else.
 *
 * This is a source-text guard, the same blunt technique `lock.rs` uses for lock
 * discipline, and it earns its place the same way — it is the only thing here
 * that catches a category of mistake no behavioural test can see.
 *
 * It globs rather than naming files, deliberately. A guard with a
 * hand-maintained list of places to look is complete only until the next place
 * is added, and the roadmap's note on this gap observed that each new frontend
 * test file made the problem slightly worse. Now each new file is simply covered.
 *
 * Sources arrive through Vite's `?raw` rather than `node:fs`, so this needs no
 * `@types/node` and no assumption about the working directory — `pnpm build`
 * type-checks these files too.
 */

/** This file's own name. It is excluded below: it quotes the markup it checks. */
const SELF = "interval-bounds.test.ts";

function sourcesThatMayCarryTheBounds(): { name: string; source: string }[] {
  // `import.meta.glob` is rewritten at build time, so both calls need literal
  // arguments — the options object cannot be hoisted into a shared constant.
  const found = {
    ...(import.meta.glob("../index.html", {
      query: "?raw",
      import: "default",
      eager: true,
    }) as Record<string, string>),
    ...(import.meta.glob("./*.test.ts", {
      query: "?raw",
      import: "default",
      eager: true,
    }) as Record<string, string>),
  };

  return Object.entries(found)
    .map(([path, source]) => ({ name: path.split("/").pop()!, source }))
    .filter(({ name }) => name !== SELF);
}

/** Every `#site-interval` tag in a source file, whole-tag so its attributes are readable. */
function intervalInputs(source: string): string[] {
  return source.match(/<input[^>]*id="site-interval"[^>]*>/g) ?? [];
}

describe("the interval bounds are written down once, in effect", () => {
  it("finds the markup it is supposed to be checking", () => {
    // Without this, deleting index.html's input — or a glob that quietly stops
    // matching — would turn every assertion below into a vacuous pass.
    const carriers = sourcesThatMayCarryTheBounds()
      .filter(({ source }) => intervalInputs(source).length > 0)
      .map(({ name }) => name);

    expect(carriers).toContain("index.html");
    expect(carriers).toContain("form.test.ts");
    expect(carriers).toContain("main.test.ts");
  });

  it.each(sourcesThatMayCarryTheBounds())("$name agrees with form.ts", ({ name, source }) => {
    for (const tag of intervalInputs(source)) {
      expect(tag, `${name} declares a min the app does not enforce`).toContain(
        `min="${MIN_INTERVAL}"`,
      );
      expect(tag, `${name} declares a max the app does not enforce`).toContain(
        `max="${MAX_INTERVAL}"`,
      );
    }
  });
});
