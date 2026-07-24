use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};

use crate::check::{build_client, check_url};
use crate::model::{Method, Site, StatusEvent};
use crate::store::Store;

/// Upper bound on the startup offset. Keeps N sites on a shared interval from
/// all firing on the same second without delaying the first result much.
const MAX_JITTER_SECS: u64 = 10;

/// A cheap handle. The spawned per-site tasks need shared ownership of the
/// engine's guts, so those live behind an `Arc` and every public method takes
/// a plain `&self`.
pub struct Engine {
    inner: Arc<Inner>,
}

struct Inner {
    app: AppHandle,
    client: reqwest::Client,
    store: Arc<Mutex<Store>>,
    tasks: Mutex<HashMap<String, tauri::async_runtime::JoinHandle<()>>>,
}

impl Engine {
    pub fn new(app: AppHandle, store: Arc<Mutex<Store>>) -> Self {
        Self {
            inner: Arc::new(Inner {
                app,
                client: build_client(),
                store,
                tasks: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub fn start_all(&self, sites: Vec<Site>) {
        for site in sites {
            self.start(site);
        }
    }

    /// Spawn the recurring check for one site. Replaces any task already
    /// running for that id.
    pub fn start(&self, site: Site) {
        let inner = Arc::clone(&self.inner);
        let id = site.id.clone();

        // One critical section: a concurrent start for the same id cannot
        // slip between the abort and the insert and leave an untracked task
        // running forever.
        let mut tasks = self.inner.tasks.lock().unwrap();
        if let Some(handle) = tasks.remove(&id) {
            handle.abort();
        }
        let handle = tauri::async_runtime::spawn(async move {
            inner.run_site(site).await;
        });
        tasks.insert(id, handle);
    }

    /// Aborts the task. A check already in flight may still emit one last
    /// event; the UI ignores events for sites it no longer has.
    pub fn stop(&self, id: &str) {
        if let Some(handle) = self.inner.tasks.lock().unwrap().remove(id) {
            handle.abort();
        }
    }

    pub fn reschedule(&self, site: Site) {
        self.start(site);
    }
}

impl Inner {
    async fn run_site(&self, site: Site) {
        // The offset is applied once, before the first check. Because the loop
        // sleeps a full interval after each check, every subsequent check keeps
        // the same offset.
        let jitter_ceiling_ms = site.interval_secs.min(MAX_JITTER_SECS) * 1000;
        let jitter_ms = rand::random_range(0..=jitter_ceiling_ms);
        tokio::time::sleep(Duration::from_millis(jitter_ms)).await;

        let interval = Duration::from_secs(site.interval_secs);
        let mut method_override = site.method_override;

        loop {
            let outcome = check_url(&self.client, &site.url, method_override).await;

            if outcome.used_get_fallback {
                method_override = Some(Method::Get);
                self.persist_get_fallback(&site.id);
            }

            let event = StatusEvent {
                id: site.id.clone(),
                state: outcome.state,
                checked_at: now_millis(),
                reason: outcome.reason,
            };
            let _ = self.app.emit("site-status", event);

            tokio::time::sleep(interval).await;
        }
    }

    /// Record that this site needs GET so future launches skip the HEAD probe.
    /// The lock is taken and released synchronously — never held across an
    /// `.await`, which would make this task non-`Send`.
    fn persist_get_fallback(&self, id: &str) {
        let mut store = self.store.lock().unwrap();
        if let Some(mut site) = store.get(id) {
            site.method_override = Some(Method::Get);
            // A write failure here is not worth a banner: the in-memory value
            // holds for this session and the next check re-discovers it.
            let _ = store.update(site);
        }
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
