use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{Emitter, State};

use crate::engine::Engine;
use crate::model::{clamp_interval, normalize_url, Site};
use crate::store::Store;

pub struct AppState {
    pub store: Arc<Mutex<Store>>,
    pub engine: Engine,
    /// Set at startup when sites.json could not be read. Read once by the UI.
    pub warning: Mutex<Option<String>>,
}

#[derive(Clone, Serialize)]
struct StoreWarning {
    message: String,
}

/// A write failure must not lose the user's edit. The in-memory change stands
/// and the UI shows a banner.
fn warn_on_write_failure(app: &tauri::AppHandle, result: Result<(), String>) {
    if let Err(message) = result {
        let _ = app.emit("store-warning", StoreWarning { message });
    }
}

fn empty_to_none(label: Option<String>) -> Option<String> {
    label
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
}

#[tauri::command]
pub fn list_sites(state: State<'_, AppState>) -> Vec<Site> {
    state.store.lock().unwrap().list()
}

#[tauri::command]
pub fn get_warning(state: State<'_, AppState>) -> Option<String> {
    state.warning.lock().unwrap().take()
}

#[tauri::command]
pub fn add_site(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    url: String,
    label: Option<String>,
    interval_secs: u64,
) -> Result<Site, String> {
    // A bad URL is rejected outright and nothing is persisted.
    let url = normalize_url(&url)?;

    let site = Site {
        id: uuid::Uuid::new_v4().to_string(),
        url,
        label: empty_to_none(label),
        interval_secs: clamp_interval(interval_secs),
        method_override: None,
    };

    let write = state.store.lock().unwrap().add(site.clone());
    warn_on_write_failure(&app, write);

    state.engine.start(site.clone());
    Ok(site)
}

#[tauri::command]
pub fn update_site(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
    url: String,
    label: Option<String>,
    interval_secs: u64,
) -> Result<Site, String> {
    let url = normalize_url(&url)?;

    let existing = state
        .store
        .lock()
        .unwrap()
        .get(&id)
        .ok_or_else(|| "That site no longer exists".to_string())?;

    // A changed URL invalidates what we learned about HEAD support.
    let method_override = if existing.url == url {
        existing.method_override
    } else {
        None
    };

    let site = Site {
        id,
        url,
        label: empty_to_none(label),
        interval_secs: clamp_interval(interval_secs),
        method_override,
    };

    let write = state.store.lock().unwrap().update(site.clone());
    warn_on_write_failure(&app, write);

    // Only this site's timer restarts; every other site is untouched.
    state.engine.reschedule(site.clone());
    Ok(site)
}

#[tauri::command]
pub fn delete_site(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.engine.stop(&id);
    let write = state.store.lock().unwrap().delete(&id);
    warn_on_write_failure(&app, write);
    Ok(())
}
