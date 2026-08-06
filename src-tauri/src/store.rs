use std::io::Write;
use std::path::PathBuf;

use crate::model::Site;

pub struct Store {
    path: PathBuf,
    sites: Vec<Site>,
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
        Ok(sites) => LoadOutcome {
            store: Store { path, sites },
            warning: None,
        },
        Err(e) => LoadOutcome {
            store: Store { path, sites: Vec::new() },
            warning: Some(format!(
                "sites.json is not valid JSON ({e}). Starting with an empty list; \
                 the existing file has been left alone."
            )),
        },
    }
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
    /// Note the seam this opens for callers, because it is not obvious from
    /// either side: `warn_on_write_failure` in `commands.rs` documents its `Err`
    /// channel as "the in-memory change stands and the UI shows a banner",
    /// which holds for a failed disk write but not for this branch, where
    /// nothing was applied at all. That is the safer direction — nothing
    /// persisted and nothing in memory — and the branch is unreachable while
    /// `add_site` mints a fresh v4 UUID for every site. It exists so the
    /// invariant lives at the layer that owns it rather than being a property
    /// of one caller's id generator, which an importer or restore path would
    /// reopen.
    pub fn add(&mut self, site: Site) -> Result<(), String> {
        if self.sites.iter().any(|s| s.id == site.id) {
            return Err(format!("A site with id {} already exists", site.id));
        }
        self.sites.push(site);
        self.save()
    }

    /// Replace the site with a matching id, preserving list order. A site that
    /// is not present is a no-op, still followed by a save.
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
    use crate::model::Method;

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

        assert!(store.add(a_site("two")).is_err(), "the save must report the failure");
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
        assert!(store.add(clash).is_err(), "a duplicate id must be refused");

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
