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

    pub fn add(&mut self, site: Site) -> Result<(), String> {
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

    fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(&self.sites)
            .map_err(|e| format!("Could not serialize sites: {e}"))?;
        std::fs::write(&self.path, json)
            .map_err(|e| format!("Could not write sites.json: {e}"))
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
}
