use std::io::Write;
use std::path::PathBuf;

use crate::model::{clamp_interval, Site, MIN_INTERVAL_SECS};

pub struct Store {
    path: PathBuf,
    sites: Vec<Site>,
}

/// Why an `add` did not happen.
///
/// Two variants rather than one string because they carry **opposite** promises
/// to the caller, and the shell has to answer them differently: one keeps the
/// user's row and warns, the other must not show a row at all.
#[derive(Debug)]
pub enum AddError {
    /// The list already holds this id. **Nothing was applied** — not in memory,
    /// not on disk. The two still agree.
    ///
    /// The payload names the clashing id. `add_site` — the only caller today —
    /// deliberately does not surface it: the user gets the plain FR-010 wording,
    /// not an internal UUID. It is kept, and the `allow` is scoped to the field
    /// rather than dropped, because this branch exists for the non-UI caller that
    /// does not exist yet (an importer, a restore path), and "something was
    /// refused" is not a useful thing to hand one. The test
    /// `a_refused_add_changes_neither_the_list_nor_the_file` asserts on it, so it
    /// is pinned rather than merely retained.
    DuplicateId(#[allow(dead_code)] String),
    /// The site **is** in the in-memory list; the save failed. The payload is
    /// the message the banner shows.
    Write(String),
}

/// What `Store::replace` hands back when there was an entry to edit.
pub struct Replaced {
    /// The entry as it now stands, after the `method_override` rule was applied.
    pub site: Site,
    /// Whether the save that followed succeeded. `Err` keeps today's behaviour:
    /// the in-memory change stands and the banner fires.
    pub write: Result<(), String>,
}

pub struct LoadOutcome {
    pub store: Store,
    /// Set when the file existed but could not be read or parsed. The caller
    /// surfaces this as a non-fatal banner.
    pub warning: Option<String>,
}

/// Read `sites.json`. Never fails: a missing file is an empty list, and an
/// unreadable or corrupt file is an empty list plus a warning. In the corrupt
/// case the file on disk is deliberately left untouched — the next write will
/// overwrite it, but nothing before that does, so it can be recovered by hand.
pub fn load(path: PathBuf) -> LoadOutcome {
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return LoadOutcome {
                store: Store { path, sites: Vec::new() },
                warning: None,
            };
        }
        Err(e) => {
            return LoadOutcome {
                store: Store { path, sites: Vec::new() },
                warning: Some(format!("Could not read sites.json ({e}). Starting empty.")),
            };
        }
    };

    match serde_json::from_str::<Vec<Site>>(&raw) {
        Ok(sites) => {
            let (sites, dropped) = drop_duplicate_ids(sites);
            let (sites, clamped) = clamp_intervals(sites);

            let mut warnings: Vec<String> = Vec::new();
            if dropped > 0 {
                warnings.push(format!(
                    "sites.json held {dropped} entr{} sharing an id with an earlier one. \
                     The first of each was kept and the rest ignored; the existing file \
                     has been left alone.",
                    if dropped == 1 { "y" } else { "ies" }
                ));
            }
            if clamped > 0 {
                warnings.push(format!(
                    "{clamped} entr{} in sites.json had an interval_secs below the minimum \
                     ({MIN_INTERVAL_SECS}s) and {} been raised to the floor.",
                    if clamped == 1 { "y" } else { "ies" },
                    if clamped == 1 { "has" } else { "have" },
                ));
            }
            LoadOutcome {
                store: Store { path, sites },
                warning: if warnings.is_empty() { None } else { Some(warnings.join(" ")) },
            }
        }
        Err(e) => LoadOutcome {
            store: Store { path, sites: Vec::new() },
            warning: Some(format!(
                "sites.json is not valid JSON ({e}). Starting with an empty list; \
                 the existing file has been left alone."
            )),
        },
    }
}

/// Raise every site whose `interval_secs` is below the floor up to it, and report
/// how many were adjusted.
///
/// `add_site` and `update_site` apply `clamp_interval` on the command surface, but a
/// hand-edited, restored, or synced `sites.json` bypasses that path. Enforcing the
/// floor here means it holds for every `Store` regardless of origin, which is what
/// the requirement asks for: the floor is a property of *what gets scheduled*, not
/// of what the UI submits.
fn clamp_intervals(sites: Vec<Site>) -> (Vec<Site>, usize) {
    let mut clamped = 0usize;
    let sites = sites
        .into_iter()
        .map(|mut s| {
            let floored = clamp_interval(s.interval_secs);
            if floored != s.interval_secs {
                s.interval_secs = floored;
                clamped += 1;
            }
            s
        })
        .collect();
    (sites, clamped)
}

/// Keep the first entry for each id, and report how many later ones were dropped.
///
/// `Store::add` refuses an id the list already holds, but that guard only covers
/// the **append** path. A hand-edited, restored, or imported `sites.json` can put
/// two entries with the same id in front of `load`, and every lookup below this
/// line assumes ids are unique: `get`, `replace`, and `update` act on the first
/// match while `delete` removes both. Enforcing the invariant here means it holds
/// for every `Store` however the list arrived, rather than only for lists this
/// process appended to.
///
/// The *later* entry loses, so the result is what a reader going down the file
/// top-down would have seen. The file itself is deliberately left alone — the
/// next save rewrites it, and until then the discarded entry is recoverable by
/// hand, which is the same bargain the corrupt-file branch above strikes.
fn drop_duplicate_ids(sites: Vec<Site>) -> (Vec<Site>, usize) {
    let before = sites.len();
    let mut seen = std::collections::HashSet::new();
    let kept: Vec<Site> = sites
        .into_iter()
        .filter(|s| seen.insert(s.id.clone()))
        .collect();
    let dropped = before - kept.len();
    (kept, dropped)
}

impl Store {
    pub fn list(&self) -> Vec<Site> {
        self.sites.clone()
    }

    pub fn get(&self, id: &str) -> Option<Site> {
        self.sites.iter().find(|s| s.id == id).cloned()
    }

    /// Append a site. An id already in the list is refused *before* anything is
    /// mutated, so a refusal leaves the in-memory list and the file agreeing —
    /// pushing first and unwinding on failure would not.
    ///
    /// The two failure modes are the `AddError` variants, and their doc comments
    /// are the contract. This used to be a paragraph here, because there was no
    /// type to say it with; the part about what the *caller* then owes the user
    /// lives with the caller, in `commands.rs`.
    pub fn add(&mut self, site: Site) -> Result<(), AddError> {
        if self.sites.iter().any(|s| s.id == site.id) {
            return Err(AddError::DuplicateId(format!(
                "A site with id {} already exists",
                site.id
            )));
        }
        self.sites.push(site);
        self.save().map_err(AddError::Write)
    }

    /// Apply an edit: read the current entry, decide the learned request method
    /// from it, write the result, and save — all under one `&mut self` borrow.
    ///
    /// That single borrow is the guarantee. There is no moment between the read
    /// and the write for a second edit to interleave into, so FR-013 holds by
    /// construction rather than by a rule each caller has to remember. The
    /// negative control in this file's tests reproduces the two-lock shape this
    /// replaced and asserts that it *does* lose an overlapping edit.
    ///
    /// Inputs arrive pre-shaped. `normalize_url`, `clamp_interval`, and
    /// `empty_to_none` stay in `commands.rs`: they are input shaping, not list
    /// invariants. The one rule this owns is what happens to `method_override`,
    /// moved here verbatim from `commands.rs` — the behaviour is unchanged
    /// (FR-014), only its home is.
    ///
    /// `Option<Replaced>` rather than a `Result` because "there was nothing to
    /// edit" and "the edit happened but did not persist" demand opposite
    /// responses from the caller: the first must report the site is gone and
    /// change nothing, the second must keep the row and warn.
    pub fn replace(
        &mut self,
        id: &str,
        url: String,
        label: Option<String>,
        interval_secs: u64,
    ) -> Option<Replaced> {
        let site = {
            let slot = self.sites.iter_mut().find(|s| s.id == id)?;

            // A changed URL invalidates what we learned about HEAD support.
            let method_override = if slot.url == url {
                slot.method_override
            } else {
                None
            };

            *slot = Site {
                id: id.to_string(),
                url,
                label,
                interval_secs,
                method_override,
            };
            slot.clone()
        };

        Some(Replaced {
            site,
            write: self.save(),
        })
    }

    /// Replace the site with a matching id, preserving list order. A site that
    /// is not present is a no-op, still followed by a save.
    ///
    /// Deliberately **not** absorbed into `replace`, and kept for
    /// `engine::persist_get_fallback`: recording that a site needs GET is a
    /// legitimate blind write, and giving it the edit rules would mean it had
    /// opinions about URLs and labels that it has no business having.
    pub fn update(&mut self, site: Site) -> Result<(), String> {
        if let Some(slot) = self.sites.iter_mut().find(|s| s.id == site.id) {
            *slot = site;
        }
        self.save()
    }

    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        self.sites.retain(|s| s.id != id);
        self.save()
    }

    /// Where the staged copy goes: the real file name with `.tmp` appended,
    /// in the real file's own directory.
    fn staging_path(&self) -> PathBuf {
        let mut name = self.path.file_name().unwrap_or_default().to_os_string();
        name.push(".tmp");
        self.path.with_file_name(name)
    }

    /// Write the list to disk without publishing it, and hand back the staging
    /// path for `save` to rename into place.
    ///
    /// The staging file is a *sibling* of `sites.json` because `rename` is only
    /// atomic within one filesystem — staging in `std::env::temp_dir()` could
    /// cross a volume boundary, where the rename stops being atomic and usually
    /// fails outright. Its name is *fixed* rather than randomized, so a run of
    /// interrupted saves keeps reusing the one artifact instead of leaving an
    /// orphan per crash. An orphan is inert either way: `load` opens only the
    /// path it was handed, so it never sees a sibling.
    ///
    /// Split out from `save` because the split is the design — everything here
    /// is invisible to a reader and the rename is the instant of publication.
    /// It is also the only way to prove the guarantee in a unit test: a test
    /// can call this and stop, which is exactly "the process died mid-save".
    fn stage(&self) -> Result<PathBuf, String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(&self.sites)
            .map_err(|e| format!("Could not serialize sites: {e}"))?;

        let staged = self.staging_path();
        let mut file = std::fs::File::create(&staged)
            .map_err(|e| format!("Could not write sites.json: {e}"))?;
        file.write_all(json.as_bytes())
            .map_err(|e| format!("Could not write sites.json: {e}"))?;
        // Before the rename, not after. The rename publishes the *name*; if the
        // bytes are still only in the page cache at that point, a crash can
        // leave the published name pointing at an empty file. `File::create` +
        // `write_all` rather than `fs::write` purely because `fs::write` hands
        // back no handle to sync.
        file.sync_all()
            .map_err(|e| format!("Could not flush sites.json to disk: {e}"))?;
        Ok(staged)
    }

    /// Publish the list atomically. `rename` within a filesystem is atomic at
    /// the VFS layer, so a reader sees either the complete previous file or the
    /// complete new one — never the truncated middle that the plain `fs::write`
    /// this replaced left exposed for the whole time it took to refill the file.
    ///
    /// The honest limit: this defends against the *process* dying — a panic, a
    /// kill, a dev-server restart — because the kernel completes the rename
    /// whether or not we survive it. It is not a power-loss guarantee. macOS
    /// `fsync` does not force the drive's own write cache the way `F_FULLFSYNC`
    /// does, and the parent directory is deliberately not synced.
    ///
    /// Neither failure path cleans up the staging file, on purpose. If staging
    /// fails the rename never runs and `sites.json` is untouched; if the rename
    /// fails the staged copy stays as the single permitted orphan, bounded at
    /// one by the fixed name and reclaimed by the next save.
    fn save(&self) -> Result<(), String> {
        let staged = self.stage()?;
        std::fs::rename(&staged, &self.path)
            .map_err(|e| format!("Could not replace sites.json: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lock::recover;
    use crate::model::Method;
    use std::sync::{Arc, Barrier, Mutex};

    /// The address the contention tests start from, and the one the *later* edit
    /// keeps.
    const U0: &str = "https://u0.example.com/";
    /// The address the *earlier* edit repoints the site at.
    const U1: &str = "https://u1.example.com/";

    /// A site whose request method the app has already learned, at a cost of one
    /// failed request. That learned value is the thing an overlapping edit can
    /// throw away, so it is what the contention tests watch.
    fn a_learned_site() -> Site {
        Site {
            id: "one".to_string(),
            url: U0.to_string(),
            label: None,
            interval_secs: 60,
            method_override: Some(Method::Get),
        }
    }

    /// One edit, applied by some implementation. `pause` is that
    /// implementation's read→write window: whatever another thread does inside
    /// it is what the implementation is blind to.
    type Edit = fn(&Arc<Mutex<Store>>, &str, &str, u64, &dyn Fn());

    /// Today's `commands.rs` shape, reproduced: lock and read, **drop the
    /// guard**, decide `method_override` from what was read, then lock again and
    /// write. The window is between the two locks, which is the whole problem.
    ///
    /// Acquired through `lock::recover` rather than a hand-rolled unrecovered
    /// lock so `lock.rs`'s source-text guard needs no carve-out for this file.
    fn edit_the_old_two_lock_way(
        shared: &Arc<Mutex<Store>>,
        id: &str,
        url: &str,
        interval_secs: u64,
        pause: &dyn Fn(),
    ) {
        let existing = recover(shared).0.get(id).expect("the site must exist");

        pause();

        // A changed URL invalidates what we learned about HEAD support.
        let method_override = if existing.url == url {
            existing.method_override
        } else {
            None
        };

        let site = Site {
            id: id.to_string(),
            url: url.to_string(),
            label: existing.label,
            interval_secs,
            method_override,
        };
        let _ = recover(shared).0.update(site);
    }

    /// The same edit through `Store::replace`. The window can only go *before*
    /// the lock, because there is no inside — that absence is the fix, and
    /// putting `pause` here is the most generous possible placement for it.
    fn edit_atomically(
        shared: &Arc<Mutex<Store>>,
        id: &str,
        url: &str,
        interval_secs: u64,
        pause: &dyn Fn(),
    ) {
        pause();

        recover(shared)
            .0
            .replace(id, url.to_string(), None, interval_secs)
            .expect("the site must exist");
    }

    /// Drive two overlapping edits through `edit` and return the entry left
    /// behind.
    ///
    /// Both tests below call this with the *same* choreography and the *same*
    /// inputs, differing only in the implementation handed in. That is what
    /// makes the negative control mean something: the two cannot drift apart,
    /// because there is only one sequence.
    ///
    /// Sequenced with a `Barrier`, never sleeps. The earlier edit runs to
    /// completion strictly inside the later edit's window, so a stale read is
    /// deterministic rather than likely:
    ///
    /// - the later edit opens its window and waits
    /// - the earlier edit repoints the site at `U1`, correctly clearing the
    ///   learned method, and finishes
    /// - the later edit closes its window and writes `U0`
    ///
    /// An implementation that read before the window decides `U0 == U0` and
    /// carries `Some(Get)` forward. One that reads after it sees `U1`, and must
    /// clear. Same sequence, opposite answers.
    fn run_overlapping_edits(edit: Edit) -> Site {
        let dir = tempfile::tempdir().unwrap();
        let mut store = load(dir.path().join("sites.json")).store;
        store.add(a_learned_site()).unwrap();
        let shared = Arc::new(Mutex::new(store));
        let gate = Arc::new(Barrier::new(2));

        let earlier = {
            let shared = Arc::clone(&shared);
            let gate = Arc::clone(&gate);
            std::thread::spawn(move || {
                gate.wait();
                edit(&shared, "one", U1, 60, &|| {});
                gate.wait();
            })
        };

        let gate_for_later = Arc::clone(&gate);
        edit(&shared, "one", U0, 300, &move || {
            gate_for_later.wait();
            gate_for_later.wait();
        });
        earlier.join().unwrap();

        // Bound rather than returned directly: as a tail expression the guard
        // temporary would outlive `shared` itself.
        let site = recover(&shared).0.get("one").unwrap();
        site
    }

    /// **This test asserts the bug, deliberately.**
    ///
    /// It is the negative control for `two_overlapping_edits_decide_from_the_current_entry`.
    /// Without it that test could pass against an implementation which never had
    /// the race to begin with, and nobody could tell — quickstart §4 says so
    /// outright: "if it passes both ways it is testing the wrong thing."
    ///
    /// If this one ever starts failing, the shape it reproduces is no longer the
    /// shape `replace` replaced, and its partner has stopped proving anything.
    #[test]
    fn the_old_two_lock_shape_would_lose_the_earlier_edit() {
        let site = run_overlapping_edits(edit_the_old_two_lock_way);

        assert_eq!(site.url, U0, "the later edit's address wins, as it should");
        assert_eq!(
            site.method_override,
            Some(Method::Get),
            "and it resurrects a learned method decided from a picture the earlier edit had \
             already replaced — by then the entry said U1, so the method was not the later \
             edit's to carry forward. This is the bug `replace` removes."
        );
    }

    /// The story. Identical choreography to the negative control above —
    /// literally the same function, the same barrier sequence, the same two
    /// edits — with `replace` in place of the two-lock shape.
    ///
    /// The earlier edit still completes strictly inside the later edit's window.
    /// The difference is that `replace` has no window between its read and its
    /// write to be blind to, so the later edit decides from the earlier edit's
    /// *result* and clears a method that is no longer its to carry.
    #[test]
    fn two_overlapping_edits_decide_from_the_current_entry() {
        let site = run_overlapping_edits(edit_atomically);

        assert_eq!(site.url, U0, "the later edit's address wins");
        assert_eq!(
            site.method_override, None,
            "and the learned method is cleared, because by the time this edit was decided the \
             entry said U1 — the address did change. Run the identical sequence through the \
             two-lock shape and it answers Some(Get); that difference is the whole story."
        );
    }

    #[test]
    fn an_unchanged_url_carries_the_learned_method_forward() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = load(dir.path().join("sites.json")).store;
        store.add(a_learned_site()).unwrap();

        let replaced = store
            .replace("one", U0.to_string(), Some("renamed".to_string()), 300)
            .unwrap();

        assert_eq!(
            replaced.site.method_override,
            Some(Method::Get),
            "the address did not change, so the learned method must not be thrown away"
        );
        assert_eq!(replaced.site.label.as_deref(), Some("renamed"));
        assert_eq!(replaced.site.interval_secs, 300);
    }

    #[test]
    fn a_changed_url_drops_the_learned_method_so_it_is_relearned() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = load(dir.path().join("sites.json")).store;
        store.add(a_learned_site()).unwrap();

        let replaced = store.replace("one", U1.to_string(), None, 60).unwrap();

        assert_eq!(
            replaced.site.method_override, None,
            "HEAD support is a property of the address, so a new address must be re-probed"
        );
    }

    #[test]
    fn replacing_an_unknown_id_writes_nothing_at_all() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");
        let mut store = load(path.clone()).store;
        store.add(a_learned_site()).unwrap();
        let before = std::fs::read_to_string(&path).unwrap();

        let outcome = store.replace("nobody", U1.to_string(), None, 60);

        assert!(outcome.is_none(), "an absent id is reported, not invented");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            before,
            "and no save runs — `update`'s no-op-then-save behaviour is not inherited here"
        );
        assert_eq!(
            file_names(dir.path()),
            vec!["sites.json"],
            "so no staging artifact appears either"
        );
    }

    #[test]
    fn replace_preserves_list_order() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");
        let mut store = load(path.clone()).store;
        store.add(a_learned_site()).unwrap();
        store.add(a_site("two")).unwrap();
        store.add(a_site("three")).unwrap();

        store.replace("two", U1.to_string(), None, 120).unwrap();

        let ids: Vec<String> = load(path).store.list().into_iter().map(|s| s.id).collect();
        assert_eq!(ids, vec!["one", "two", "three"], "an edit is in-place, not a move");
    }

    #[test]
    fn a_failed_save_still_leaves_the_edit_standing_in_memory() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");
        let mut store = load(path).store;
        store.add(a_learned_site()).unwrap();

        // Same failure injection as `a_failed_save_leaves_the_previous_file_intact`:
        // a directory where the staging file wants to be.
        std::fs::create_dir(dir.path().join("sites.json.tmp")).unwrap();

        let replaced = store.replace("one", U1.to_string(), None, 300).unwrap();

        assert!(replaced.write.is_err(), "the caller must learn the save failed");
        assert_eq!(
            store.get("one").unwrap().url,
            U1,
            "but the edit stands in memory, so the caller keeps the row and just warns"
        );
    }

    fn a_site(id: &str) -> Site {
        Site {
            id: id.to_string(),
            url: format!("https://{id}.example.com"),
            label: None,
            interval_secs: 60,
            method_override: None,
        }
    }

    #[test]
    fn missing_file_yields_an_empty_list_and_no_warning() {
        let dir = tempfile::tempdir().unwrap();
        let outcome = load(dir.path().join("sites.json"));
        assert!(outcome.store.list().is_empty());
        assert!(outcome.warning.is_none());
    }

    #[test]
    fn add_then_reload_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");

        let mut store = load(path.clone()).store;
        store.add(a_site("one")).unwrap();
        store.add(a_site("two")).unwrap();

        let reloaded = load(path).store;
        assert_eq!(reloaded.list().len(), 2);
        assert_eq!(reloaded.list()[0].url, "https://one.example.com");
    }

    #[test]
    fn update_replaces_the_matching_site_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");

        let mut store = load(path.clone()).store;
        store.add(a_site("one")).unwrap();
        store.add(a_site("two")).unwrap();

        let mut edited = a_site("one");
        edited.interval_secs = 300;
        edited.method_override = Some(Method::Get);
        store.update(edited).unwrap();

        let reloaded = load(path).store;
        let got = reloaded.get("one").unwrap();
        assert_eq!(got.interval_secs, 300);
        assert_eq!(got.method_override, Some(Method::Get));
        assert_eq!(reloaded.list().len(), 2);
        assert_eq!(reloaded.list()[0].id, "one", "order is preserved");
    }

    /// `update` is the blind write `engine::persist_get_fallback` uses, and it
    /// silently does nothing when the id is gone — which is the *right* behaviour
    /// (a check that outlived its site's deletion must not resurrect it) but had
    /// no test, so nothing distinguished it from a bug.
    ///
    /// The `Ok` matters as much as the no-op: this returns the result of the save
    /// that follows, so a caller cannot read `Ok` as "the site was updated". It
    /// means "nothing failed".
    #[test]
    fn updating_a_missing_id_changes_nothing_and_still_reports_ok() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");

        let mut store = load(path.clone()).store;
        store.add(a_site("one")).unwrap();

        let mut ghost = a_site("gone");
        ghost.interval_secs = 999;
        assert!(
            store.update(ghost).is_ok(),
            "the save still ran and still succeeded"
        );

        let reloaded = load(path).store;
        assert_eq!(
            reloaded.list().len(),
            1,
            "a blind write for a deleted site must not resurrect it"
        );
        assert_eq!(reloaded.list()[0].id, "one");
        assert!(reloaded.get("gone").is_none());
    }

    #[test]
    fn delete_removes_only_the_named_site() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");

        let mut store = load(path.clone()).store;
        store.add(a_site("one")).unwrap();
        store.add(a_site("two")).unwrap();
        store.delete("one").unwrap();

        let reloaded = load(path).store;
        assert_eq!(reloaded.list().len(), 1);
        assert_eq!(reloaded.list()[0].id, "two");
    }

    #[test]
    fn corrupt_file_yields_an_empty_list_a_warning_and_is_left_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");
        std::fs::write(&path, "{ this is not json").unwrap();

        let outcome = load(path.clone());
        assert!(outcome.store.list().is_empty());
        assert!(outcome.warning.is_some());

        let still_there = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            still_there, "{ this is not json",
            "the bad file must not be overwritten so it can be recovered by hand"
        );
    }

    /// Two entries, one id. `add` could never have produced this file; a hand
    /// edit, a restore, or an importer can. The invariant `AddError::DuplicateId`
    /// protects has to hold for a list that arrived through `load` too, because
    /// everything downstream of here already assumes it does.
    #[test]
    fn load_keeps_the_first_of_two_entries_sharing_an_id_and_warns() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");
        let clash = Site {
            url: "https://second.example.com".to_string(),
            ..a_site("one")
        };
        std::fs::write(&path, serde_json::to_string(&[a_site("one"), clash]).unwrap()).unwrap();

        let outcome = load(path.clone());

        assert_eq!(outcome.store.list().len(), 1, "the duplicate must not load");
        assert_eq!(
            outcome.store.list()[0].url,
            "https://one.example.com",
            "the first entry wins, so the result is what a top-down reader would have seen"
        );
        assert!(
            outcome.warning.is_some(),
            "silently dropping a site the user can see in the file would be worse than the \
             duplicate — this rides the same banner channel the corrupt-file case uses"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            serde_json::to_string(&[
                a_site("one"),
                Site {
                    url: "https://second.example.com".to_string(),
                    ..a_site("one")
                }
            ])
            .unwrap(),
            "and the file is left alone, so the discarded entry is recoverable by hand"
        );
    }

    /// A site with `interval_secs: 0` in `sites.json` must be raised to the
    /// floor and a warning emitted, so it cannot poll in a tight loop.
    ///
    /// This is the load-path parallel to the command-surface guard in `add_site`
    /// and `update_site`. A hand-edited or restored file can set any value; the
    /// floor must hold regardless of origin, not only for values that came through
    /// the UI.
    #[test]
    fn load_clamps_an_interval_below_the_floor_and_warns() {
        use crate::model::MIN_INTERVAL_SECS;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");
        let mut zero_interval = a_site("one");
        zero_interval.interval_secs = 0;
        std::fs::write(&path, serde_json::to_string(&[zero_interval]).unwrap()).unwrap();

        let outcome = load(path);

        assert_eq!(
            outcome.store.list()[0].interval_secs,
            MIN_INTERVAL_SECS,
            "a zero interval must be raised to the floor, not scheduled as-is"
        );
        assert!(
            outcome.warning.is_some(),
            "the clamping must be surfaced as a warning so the user knows the file was adjusted"
        );
    }

    /// The anti-test: a file where every interval is valid must produce no warning
    /// on load. Without this, `load_clamps_an_interval_below_the_floor_and_warns`
    /// could pass against an implementation that always warns.
    #[test]
    fn load_warns_about_nothing_when_all_intervals_are_at_or_above_the_floor() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");
        std::fs::write(
            &path,
            serde_json::to_string(&[a_site("one"), a_site("two")]).unwrap(),
        )
        .unwrap();

        let outcome = load(path);

        assert_eq!(outcome.store.list().len(), 2);
        assert!(outcome.warning.is_none());
    }

    /// Both defects at once: duplicate ids AND a below-floor interval. Both must
    /// be corrected and both must be reported in a single warning string.
    #[test]
    fn load_combines_duplicate_and_interval_warnings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");
        let mut zero_interval = a_site("one");
        zero_interval.interval_secs = 0;
        let clash = Site { url: "https://second.example.com".to_string(), ..a_site("one") };
        std::fs::write(
            &path,
            serde_json::to_string(&[zero_interval, clash]).unwrap(),
        )
        .unwrap();

        let outcome = load(path);

        assert_eq!(outcome.store.list().len(), 1, "duplicate must be dropped");
        assert_eq!(
            outcome.store.list()[0].interval_secs,
            crate::model::MIN_INTERVAL_SECS,
            "interval must be raised to the floor"
        );
        let warning = outcome.warning.expect("both defects must produce a warning");
        assert!(
            warning.contains("interval") || warning.contains("minimum"),
            "warning must mention the interval clamp"
        );
    }

    /// The other half: the guard must not fire on a file that is merely fine.
    /// Without this, `load` returning a warning for every list would pass the
    /// test above.
    #[test]
    fn load_warns_about_nothing_when_every_id_is_distinct() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");
        std::fs::write(
            &path,
            serde_json::to_string(&[a_site("one"), a_site("two"), a_site("three")]).unwrap(),
        )
        .unwrap();

        let outcome = load(path);

        assert_eq!(outcome.store.list().len(), 3);
        assert!(outcome.warning.is_none());
    }

    /// A duplicate that survived `load` would make `delete` remove two rows for
    /// one click while `get` only ever saw one of them — the concrete divergence
    /// the dedupe exists to prevent, pinned so the reason outlives the fix.
    #[test]
    fn a_deduped_load_leaves_delete_and_get_agreeing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");
        std::fs::write(
            &path,
            serde_json::to_string(&[a_site("one"), a_site("one"), a_site("two")]).unwrap(),
        )
        .unwrap();

        let mut store = load(path).store;

        // This is the assertion that distinguishes the two implementations, and
        // it has to come *before* the delete: without the dedupe the list is
        // three long and `get` can still only ever reach the first "one", so the
        // window shows one row per id while the store holds two.
        assert_eq!(
            store.list().len(),
            2,
            "the list the window renders must have one entry per id"
        );

        assert!(store.get("one").is_some());
        store.delete("one").unwrap();

        assert!(store.get("one").is_none());
        assert_eq!(store.list().len(), 1);
        assert_eq!(store.list()[0].id, "two");
    }

    #[test]
    fn writes_create_the_parent_directory() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("deeper").join("sites.json");

        let mut store = load(path.clone()).store;
        store.add(a_site("one")).unwrap();

        assert!(path.exists());
    }

    /// Names in `dir`, sorted, so a directory's contents can be asserted exactly.
    fn file_names(dir: &std::path::Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn an_interrupted_save_leaves_the_previous_list_loadable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");

        let mut store = load(path.clone()).store;
        store.add(a_site("one")).unwrap();
        store.add(a_site("two")).unwrap();

        // A save that got as far as staging and then died. `stage` is everything
        // `save` does except the rename, so calling it directly reproduces the
        // exact instant the atomicity guarantee is about.
        store.sites.push(a_site("three"));
        store.stage().unwrap();

        let outcome = load(path);
        assert_eq!(
            outcome.store.list().len(),
            2,
            "the staged third site must not be visible until the rename publishes it"
        );
        assert!(
            outcome.warning.is_none(),
            "an interrupted save must not make the live file look corrupt"
        );
    }

    #[test]
    fn a_staged_save_holds_the_new_contents_beside_the_live_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");

        let mut store = load(path.clone()).store;
        store.add(a_site("one")).unwrap();
        store.add(a_site("two")).unwrap();

        store.sites.push(a_site("three"));
        let staged = store.stage().unwrap();

        assert_eq!(
            staged.parent(),
            path.parent(),
            "the staging file must be a sibling or the rename stops being atomic"
        );
        let staged_sites: Vec<Site> =
            serde_json::from_str(&std::fs::read_to_string(&staged).unwrap()).unwrap();
        assert_eq!(staged_sites.len(), 3, "the staged copy is complete, just unpublished");
        assert_eq!(load(path).store.list().len(), 2, "the live file still holds the old list");
    }

    #[test]
    fn a_successful_save_leaves_no_staging_file_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");

        let mut store = load(path).store;
        store.add(a_site("one")).unwrap();
        store.add(a_site("two")).unwrap();

        assert_eq!(
            file_names(dir.path()),
            vec!["sites.json"],
            "the rename consumes the staging file; a leftover means it became a copy"
        );
    }

    #[test]
    fn repeated_staging_never_accumulates_more_than_one_artifact() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");

        let mut store = load(path).store;
        store.add(a_site("one")).unwrap();

        // Three interrupted saves in a row. A randomized staging name would
        // leave three orphans here and pass every other test in this story.
        store.stage().unwrap();
        store.stage().unwrap();
        store.stage().unwrap();

        assert_eq!(
            file_names(dir.path()),
            vec!["sites.json", "sites.json.tmp"],
            "the staging name is fixed, so orphans cannot accumulate"
        );
    }

    #[test]
    fn a_failed_save_leaves_the_previous_file_intact() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");

        let mut store = load(path.clone()).store;
        store.add(a_site("one")).unwrap();
        let before = std::fs::read_to_string(&path).unwrap();

        // A directory where the staging file wants to be: `File::create` fails,
        // so the rename never runs. Failing the *staging* step rather than the
        // rename is what leaves a previous file to assert on at all.
        std::fs::create_dir(dir.path().join("sites.json.tmp")).unwrap();

        assert!(
            matches!(store.add(a_site("two")), Err(AddError::Write(_))),
            "a save failure must report as a write failure, so the caller keeps the row"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            before,
            "a failed save must not touch the previous file"
        );
    }

    #[test]
    fn a_failed_publish_leaves_the_staged_copy_as_the_single_orphan() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");

        // A directory at the live path: staging succeeds, the rename does not.
        // This is the other half of the failure contract from
        // `a_failed_save_leaves_the_previous_file_intact`, which fails earlier.
        std::fs::create_dir(&path).unwrap();

        let mut store = load(path.clone()).store;
        assert!(store.add(a_site("one")).is_err(), "the rename must report the failure");
        assert!(
            dir.path().join("sites.json.tmp").exists(),
            "a failed publish leaves the staged copy rather than cleaning it up"
        );
        assert_eq!(
            file_names(dir.path()),
            vec!["sites.json", "sites.json.tmp"],
            "and leaves exactly one, not one per attempt"
        );
    }

    #[test]
    fn add_rejects_a_duplicate_id() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");

        let mut store = load(path.clone()).store;
        store.add(a_site("one")).unwrap();

        // A different site under the same id, so a silent replace is
        // distinguishable from a silent no-op.
        let mut clash = a_site("one");
        clash.interval_secs = 999;
        // Asserting the *variant*, not `.is_err()`. The looser form compiles
        // untouched against a version that cannot tell a refusal from a write
        // failure — which is exactly the bug this story fixes, so a test that
        // cannot see the difference is not pinning it.
        assert!(
            matches!(store.add(clash), Err(AddError::DuplicateId(_))),
            "a duplicate id must be refused as a refusal, not reported as a failed write"
        );

        // Asserting on the reload rather than the in-memory list is what proves
        // the refusal happened before any write.
        let reloaded = load(path).store;
        assert_eq!(reloaded.list().len(), 1, "the refused site must not be persisted");
        assert_eq!(
            reloaded.get("one").unwrap().interval_secs,
            60,
            "the original site must be left exactly as it was"
        );
    }

    #[test]
    fn a_refused_add_changes_neither_the_list_nor_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");

        let mut store = load(path.clone()).store;
        store.add(a_site("one")).unwrap();
        let before = std::fs::read_to_string(&path).unwrap();

        let mut clash = a_site("one");
        clash.interval_secs = 999;
        let refused = store.add(clash);

        match &refused {
            // The payload is the diagnostic, and it earns its place by naming the
            // clashing id: the branch exists for a future importer or restore
            // path, and "some site was refused" is not a useful thing to hand
            // one. It never reaches the user — `add_site` words its own message.
            Err(AddError::DuplicateId(diagnostic)) => assert!(
                diagnostic.contains("one"),
                "the diagnostic must name the clashing id, got: {diagnostic}"
            ),
            other => panic!("the refusal must be tellable from a write failure, got {other:?}"),
        }
        assert_eq!(store.list().len(), 1, "the in-memory list must be untouched");
        assert_eq!(
            store.get("one").unwrap().interval_secs,
            60,
            "and must still hold the original entry rather than the clashing one"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            before,
            "the file must be byte-identical: the refusal is decided before any write"
        );
        assert_eq!(
            file_names(dir.path()),
            vec!["sites.json"],
            "and no staging artifact may appear, because no save was ever attempted"
        );
    }

    #[test]
    fn add_still_accepts_a_distinct_id() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");

        let mut store = load(path.clone()).store;
        store.add(a_site("one")).unwrap();
        store.add(a_site("two")).unwrap();

        let reloaded = load(path).store;
        assert_eq!(reloaded.list().len(), 2);
        assert_eq!(reloaded.list()[0].id, "one", "order is preserved");
    }

    #[test]
    fn a_stale_staging_file_does_not_affect_load_or_the_next_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.json");

        let mut store = load(path.clone()).store;
        store.add(a_site("one")).unwrap();
        std::fs::write(dir.path().join("sites.json.tmp"), "{ garbage from a crashed run").unwrap();

        let outcome = load(path.clone());
        assert_eq!(outcome.store.list().len(), 1, "load reads only the path it was handed");
        assert!(outcome.warning.is_none(), "an orphan is inert, not a corruption signal");

        let mut store = outcome.store;
        store.add(a_site("two")).unwrap();
        assert_eq!(
            load(path).store.list().len(),
            2,
            "the next save reclaims the orphan rather than tripping over it"
        );
    }
}
