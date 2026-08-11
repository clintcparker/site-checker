use std::sync::{Arc, Mutex, MutexGuard};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::store::Store;

/// Lock a mutex, surviving a prior poisoning. The `bool` is `true` when *this*
/// call was the one that found and cleared a poison.
///
/// Never panics and never returns an error, whatever state the mutex is in. On
/// a healthy mutex it is indistinguishable from an unrecovered lock — same
/// blocking behaviour, same hold time, no report.
///
/// `into_inner()` hands back the data *as the panicking thread left it*: a
/// half-applied change is still there, which is the point (FR-006). Recovery
/// continues from what is there rather than discarding a list the app cannot
/// reconstruct.
///
/// `clear_poison` is what keeps the report one-shot. Without it the mutex stays
/// poisoned for the life of the process and every later lock re-reports the same
/// long-past fault — a banner on every add, edit, delete, and background write.
/// Clearing it *while the recovered guard is still held* is deliberate: it
/// compiles either way, but under the guard no other thread can observe the
/// window between the clear and our return, so two threads cannot both report
/// one fault.
pub fn recover<T>(mutex: &Mutex<T>) -> (MutexGuard<'_, T>, bool) {
    match mutex.lock() {
        Ok(guard) => (guard, false),
        Err(poisoned) => {
            mutex.clear_poison();
            (poisoned.into_inner(), true)
        }
    }
}

/// The payload behind the app's one-line non-fatal banner. Moved here from
/// `commands.rs` unchanged — same event name, same single field, same JSON, so
/// the frontend's `onStoreWarning` needs nothing.
#[derive(Clone, Serialize)]
struct StoreWarning {
    message: String,
}

/// The site list as the rest of the app is allowed to see it.
///
/// `inner` is private and has **no accessor**, and that absence is the whole
/// point. A store lock cannot be taken un-recovered and cannot forget to warn,
/// because an unrecovered lock on the site list is simply unwritable outside
/// this module. That is a structural guarantee rather than a rule the eleventh
/// call site added next year has to remember (research R2).
#[derive(Clone)]
pub struct SharedStore {
    inner: Arc<Mutex<Store>>,
    app: AppHandle,
}

impl SharedStore {
    /// Takes ownership of the loaded `Store`. Called once, from `lib.rs`'s `setup()`.
    pub fn new(app: AppHandle, store: Store) -> Self {
        Self {
            inner: Arc::new(Mutex::new(store)),
            app,
        }
    }

    /// Take the site list, surviving a prior fault.
    ///
    /// On recovery — and only then — the user gets one banner saying their saved
    /// list may not match what they last asked for (FR-004). This is the one
    /// mutex in the app whose recovery is worth reporting: the list is the only
    /// thing the app owns on disk, so a fault that may have left it half-written
    /// is exactly the non-fatal problem the existing banner was built for
    /// (Constitution II). With no fault it emits nothing and behaves exactly as
    /// the unrecovered lock it replaced (FR-007).
    ///
    /// **This function is the deliberate exception to the rule stated in
    /// `commands.rs`.** `add_site` and `update_site` both carry comments about
    /// dropping the store guard before emitting, and both spend a pair of braces
    /// to do it — so it is worth being explicit that the call below emits with
    /// the guard still alive, on purpose, rather than leaving those comments
    /// reading as contradicted by the very function they call.
    ///
    /// It is safe because the ordering is one-way: `AppHandle::emit` never
    /// re-enters the store, and the store lock is never taken while `engine.rs`'s
    /// `tasks` lock is held, so there is no cycle to invert. And it is not
    /// cleanly avoidable — the recovery flag only exists once the guard is in
    /// hand, and handing back a `(guard, bool)` pair would surrender the
    /// "cannot forget to warn" guarantee that is the entire reason this type
    /// exists. The exception buys that guarantee; the two `commands.rs` sites
    /// have no such excuse, which is why they still pay the braces.
    pub fn lock(&self) -> MutexGuard<'_, Store> {
        let (guard, recovered) = recover(&self.inner);
        if recovered {
            self.warn(
                "Something went wrong inside the app while it was working with your list. \
                 Your saved list may not reflect your most recent change.",
            );
        }
        guard
    }

    /// A write failure must not lose the user's edit. The in-memory change stands
    /// and the UI shows a banner.
    ///
    /// That contract is true again as of this feature: the one branch that
    /// violated it — a refused add, where nothing was applied anywhere — no
    /// longer arrives here. See `AddError` in `store.rs`.
    pub fn warn_on_write_failure(&self, result: Result<(), String>) {
        if let Err(message) = result {
            self.warn(&message);
        }
    }

    /// Every banner in the app is raised from here, which is what FR-004's "using
    /// the existing warning banner rather than a new mechanism" asks for.
    fn warn(&self, message: &str) {
        let _ = self.app.emit(
            "store-warning",
            StoreWarning {
                message: message.to_string(),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Put a mutex into the poisoned state, deterministically: a thread panics
    /// while holding the guard, and we join it and discard the `Err`. No sleeps,
    /// no timing, no subprocess — unwinding out of a critical section is the only
    /// supported way to poison a `Mutex`, and it is exact.
    ///
    /// **No panic-hook dance here, deliberately.** Research R6 planned a scoped
    /// `set_hook`/`take_hook` pair to keep the panic message out of every green
    /// run, and left an explicit out: drop it if `cargo test`'s own capture
    /// already suppresses the message. It does. `thread::spawn` propagates
    /// libtest's output capture into the child thread, so the message is
    /// captured and printed only when a test actually fails — which is when you
    /// want it. Measured both ways in this crate rather than reasoned about: with
    /// the hook removed, a green `cargo test lock::` prints nothing.
    ///
    /// Dropping it is also strictly better than keeping it. The hook is global,
    /// so holding it across a spawn+join would swallow the panic message of any
    /// *other* test unlucky enough to fail inside that window.
    fn poison_with<T: Send + 'static>(
        mutex: &Arc<Mutex<T>>,
        half_apply: impl FnOnce(&mut T) + Send + 'static,
    ) {
        let held_by_the_doomed_thread = Arc::clone(mutex);

        let outcome = std::thread::spawn(move || {
            // Acquired through `recover` rather than a hand-rolled unrecovered
            // lock so that the source-text guard below can scan *this* module
            // too, with no carve-out. The mutex is healthy at this point, so
            // this is `recover`'s no-op path — the one T008 pins.
            let (mut guard, _) = recover(&held_by_the_doomed_thread);
            half_apply(&mut guard);
            panic!("deliberate fault inside the critical section");
        })
        .join();

        assert!(
            outcome.is_err(),
            "the helper must actually panic, or nothing is poisoned and the test proves nothing"
        );
        assert!(
            mutex.is_poisoned(),
            "the mutex must be poisoned before the assertions below can mean anything"
        );
    }

    #[test]
    fn recover_returns_a_usable_guard_after_a_fault_and_reports_it() {
        let mutex = Arc::new(Mutex::new(vec!["one".to_string()]));
        poison_with(&mutex, |v| v.push("two".to_string()));

        let (mut guard, recovered) = recover(&mutex);

        assert!(recovered, "the first lock after a fault must report the recovery");
        guard.push("three".to_string());
        assert_eq!(guard.len(), 3, "the guard is usable, not merely returned");
    }

    #[test]
    fn recovery_preserves_what_the_faulting_operation_had_already_applied() {
        // A two-step change interrupted between the steps: "two" landed, "three"
        // never did. FR-006 is that "two" is still there afterwards — recovery
        // continues from the half-applied state rather than discarding it, which
        // is what makes "continue, not reset" real rather than aspirational.
        let mutex = Arc::new(Mutex::new(vec!["one".to_string()]));
        poison_with(&mutex, |v| v.push("two".to_string()));

        let (guard, _) = recover(&mutex);

        assert_eq!(
            *guard,
            vec!["one".to_string(), "two".to_string()],
            "the half-applied change must survive; nothing is reset, replaced, or discarded"
        );
    }

    #[test]
    fn a_second_recovery_reports_clean_so_no_degraded_mode_accumulates() {
        let mutex = Arc::new(Mutex::new(0u32));
        poison_with(&mutex, |n| *n = 7);

        let (first_guard, first) = recover(&mutex);
        drop(first_guard);
        let (guard, second) = recover(&mutex);

        assert!(first, "the fault is reported once");
        assert!(
            !second,
            "and not again — otherwise every later action re-warns about one long-past fault"
        );
        assert_eq!(*guard, 7, "and the data is still exactly what the faulting thread left");
    }

    #[test]
    fn an_unpoisoned_lock_is_indistinguishable_from_todays_behaviour() {
        let mutex = Mutex::new(vec!["one".to_string()]);

        let (guard, recovered) = recover(&mutex);

        assert!(!recovered, "no fault, no report (FR-007)");
        assert_eq!(*guard, vec!["one".to_string()]);
    }

    /// A blunt instrument, and worth saying so plainly: it greps the crate's own
    /// source text.
    ///
    /// It earns its place because `SharedStore` can only make *six* of the nine
    /// remaining lock sites safe by construction. The other three — the two
    /// `tasks` locks in `engine.rs` and `get_warning`'s `warning` lock — are
    /// call-site discipline, and discipline is exactly what stops holding when
    /// someone adds an eleventh site next year. Nothing else in the suite catches
    /// that. This does.
    ///
    /// Its cost is real and was paid immediately: the doc comments above
    /// originally *described* the pattern using its own literal spelling, and
    /// this test failed on them. They were reworded to say "unrecovered lock"
    /// rather than weakening the check, because a guard with a carve-out for
    /// prose is one bad comment away from a carve-out for code. Test code is not
    /// exempt either — that is why the helper above and `store.rs`'s contention
    /// tests acquire through `recover`.
    ///
    /// Two things about *how* it greps are deliberate, because the version before
    /// this one got both wrong and so under-covered exactly where its own doc
    /// comment claimed strength.
    ///
    /// It reads the source directory at run time rather than naming files with
    /// `include_str!`. A hand-maintained list is complete only until the next
    /// module is added, and nothing fails when someone forgets to extend it — the
    /// new file simply falls outside the guard, silently, which is the failure
    /// this test exists to prevent happening one level up.
    ///
    /// And it looks for a *set* of spellings, not one. Unwrapping the lock result
    /// is merely the most common way to take an unrecovered lock; reaching for
    /// `expect` or an `unwrap_or_*` on the same result does the same damage and
    /// reads as ordinary Rust, so all three are needles now. Destructuring the
    /// result with `if let Ok(..)` or `match` still slips through — those have no
    /// fixed spelling to grep for — so this is a floor, not a ceiling, and the
    /// test's name promises slightly more than it can deliver. Better to say so
    /// than to let a future reader assume the coverage is total.
    ///
    /// Note the needles are assembled with `concat!` for the same reason the
    /// original one was: naming them contiguously anywhere in this crate, prose
    /// included, is itself a match. That is not a flaw to work around. A guard
    /// with a carve-out for comments is one bad comment away from a carve-out for
    /// code, so the prose bends and the check does not.
    #[test]
    fn no_module_in_this_crate_takes_a_lock_without_recovering() {
        // Concatenated so this assertion's own source text is not a match.
        let unrecovered = [
            concat!(".lock()", ".unwrap()"),
            concat!(".lock()", ".expect("),
            concat!(".lock()", ".unwrap_or"),
        ];

        let src = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
        let mut checked = 0;

        for entry in std::fs::read_dir(src).expect("the crate's own src/ must be readable") {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let source = std::fs::read_to_string(&path).unwrap();
            checked += 1;

            for needle in unrecovered {
                assert!(
                    !source.contains(needle),
                    "{name} takes a lock without recovering from a prior fault \
                     (found `{needle}`). Use SharedStore::lock for the site list, or \
                     lock::recover with a comment saying why the flag is discarded \
                     (SC-002, FR-001)."
                );
            }
        }

        assert!(
            checked >= 8,
            "only {checked} module(s) were scanned — the directory walk found less than the \
             crate holds, so a pass here would mean nothing"
        );
    }
}
