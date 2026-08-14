import { getVersion } from "./api";
import { mountUrlOpener } from "./open";

/** What the version line reads when the version could not be read. Also the
 *  placeholder `index.html` ships, so the line is never blank. */
const VERSION_UNAVAILABLE = "Version unavailable";

interface AboutHooks {
  /** Where a refusal goes — the banner, same as the table's opener. */
  onError: (message: string) => void;
  /** Injectable only so the repeat-suppression window can be exercised without
   *  waiting out a real second. Production passes nothing. Threaded straight
   *  through to `mountUrlOpener`, which is where the rule actually lives. */
  now?: () => number;
}

/**
 * Wires the About dialog — a third thin shell over the page, in the shape
 * `form.ts` and `open.ts` already established.
 *
 * The dialog is a real `<dialog>`, so `showModal()` supplies Esc-dismissal,
 * focus containment and top-layer stacking for free. The only state in this
 * module is the element's own `open` property; nothing is held in a
 * module-level variable, and nothing here reads or mutates `sites`,
 * `statuses`, or the scheduler.
 */
export function mountAbout(hooks: AboutHooks): void {
  // Bailing out rather than asserting non-null, for the reason recorded at
  // `main.ts:70-74`: a missing control should cost its own feature, not the
  // rest of the page.
  const dialog = document.querySelector<HTMLDialogElement>("#about");
  const opener = document.querySelector<HTMLElement>("#about-open");
  const closer = dialog?.querySelector<HTMLElement>("[data-about-close]");

  if (!dialog || !opener || !closer) {
    hooks.onError("The About dialog is missing from the page.");
    return;
  }

  const version = dialog.querySelector<HTMLElement>("#about-version");
  // Fetched once, on first open rather than at startup: it is a constant for
  // the life of the process, and a dialog nobody opens should cost nothing.
  let versionRequested = false;

  opener.addEventListener("click", () => {
    // Opened first, and synchronously. Awaiting the version before showing the
    // dialog would let a slow or hung read hold the whole surface closed, and
    // the version is the one thing on it nobody came for.
    dialog.showModal();

    if (versionRequested || !version) return;
    versionRequested = true;

    getVersion()
      .then((value) => {
        // Verbatim. `0.0.0` is the local-build sentinel that release.yml
        // replaces from the pushed tag; parsing or hiding it would be the app
        // second-guessing its own build.
        version.textContent = `Version ${value}`;
      })
      .catch(() => {
        // Deliberately not a banner. A version that could not be read is not
        // something the user can act on, and the attribution and the link —
        // what the dialog exists for — are unaffected.
        version.textContent = VERSION_UNAVAILABLE;
      });
  });

  closer.addEventListener("click", () => {
    dialog.close();
  });

  // `mountUrlOpener` unchanged, mounted on the dialog instead of on the table
  // body: it is written against a generic HTMLElement and matches
  // `[data-open-url]` by delegated listener, so the dialog needs no edit to
  // open.ts and no second copy of the rule.
  //
  // Repeat suppression comes entirely from its existing 1000 ms per-URL ledger.
  // That ledger is per-mount, so the dialog's is separate from the table's —
  // deliberate: a site whose URL happens to be https://clintparker.com must not
  // be able to suppress this link, or the reverse.
  mountUrlOpener(dialog, {
    onError: (message) => {
      // Closed *before* the message goes out. The banner lives behind the
      // modal, and a message the user cannot see does not satisfy "visible"
      // within SC-005's one second.
      dialog.close();
      hooks.onError(message);
    },
    now: hooks.now,
  });
}
