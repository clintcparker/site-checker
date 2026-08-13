import { openUrl } from "./api";

/**
 * How long after an accepted activation the same address is treated as
 * impatience rather than as a second request.
 *
 * A second is long enough to absorb a double-click (macOS's own double-click
 * interval is 500 ms by default) and the "nothing happened yet, click again"
 * reflex, and short enough that deliberately re-opening a page is not told no.
 *
 * The alternative — disabling the control while the command is in flight —
 * guards almost nothing: `open` returns as soon as LaunchServices accepts the
 * address, which is well before any browser is actually forward.
 */
export const ACTIVATION_WINDOW_MS = 1000;

/**
 * URL → epoch milliseconds of the last accepted activation.
 *
 * Keyed by the address rather than by the site id, because the rule is about
 * what is being opened: two sites pointing at the same URL share one window,
 * and a site whose URL was just edited starts a fresh one. It lives for as long
 * as the window does and is never persisted; its size is bounded by the number
 * of distinct addresses activated in one session, which at this app's scale
 * needs no eviction.
 */
export type ActivationLedger = Map<string, number>;

interface OpenerHooks {
  /** Where a refusal goes. The banner, not the form's error line: the failure
   *  is not about anything the user typed. */
  onError: (message: string) => void;
  /** Injectable only so the window rule can be exercised without waiting out a
   *  real second. Production passes nothing. */
  now?: () => number;
}

/**
 * Whether an activation of `url` at `now` should be acted on, recording it in
 * `ledger` when it should.
 *
 * A *suppressed* activation deliberately does not refresh the entry. If it did,
 * a user drumming on the control would keep pushing the window out ahead of
 * themselves and the address would never open at all.
 */
export function shouldOpen(ledger: ActivationLedger, url: string, now: number): boolean {
  const last = ledger.get(url);
  if (last !== undefined && now - last < ACTIVATION_WINDOW_MS) return false;
  ledger.set(url, now);
  return true;
}

/**
 * Wires the table body's URL controls to the backend — a second thin shell over
 * the `api.ts` boundary, in the shape `form.ts` already established.
 *
 * One delegated listener rather than one per row, so rows that come and go with
 * a repaint need no listener bookkeeping. It matches `[data-open-url]`, which
 * `form.ts`'s `[data-action]` listener cannot see and which cannot see
 * `form.ts`'s buttons.
 */
export function mountUrlOpener(tbody: HTMLElement, hooks: OpenerHooks): void {
  const ledger: ActivationLedger = new Map();
  const now = hooks.now ?? Date.now;

  tbody.addEventListener("click", async (e) => {
    const control = (e.target as HTMLElement).closest<HTMLElement>("[data-open-url]");
    if (!control) return;

    // Read from the attribute, never from `textContent`: the cell may render a
    // long address wrapped or truncated, and what opens has to be the whole
    // stored one.
    const url = control.dataset.openUrl!;
    if (!shouldOpen(ledger, url, now())) return;

    try {
      await openUrl(url);
    } catch (message) {
      // Rust `Err(String)` arrives here as the bare string. Nothing was
      // mutated on the way to this point, so there is no partial state to
      // unwind — the table keeps updating and the app stays usable.
      hooks.onError(String(message));
    }
  });
}
