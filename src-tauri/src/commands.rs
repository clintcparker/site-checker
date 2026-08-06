use std::sync::Mutex;

use tauri::State;

use crate::engine::Engine;
use crate::lock::{self, SharedStore};
use crate::model::{clamp_interval, normalize_url, Site};

pub struct AppState {
    pub store: SharedStore,
    pub engine: Engine,
    /// Set at startup when sites.json could not be read. Read once by the UI.
    pub warning: Mutex<Option<String>>,
}

fn empty_to_none(label: Option<String>) -> Option<String> {
    label
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
}

#[tauri::command]
pub fn list_sites(state: State<'_, AppState>) -> Vec<Site> {
    state.store.lock().list()
}

#[tauri::command]
pub fn get_warning(state: State<'_, AppState>) -> Option<String> {
    // Recovers silently, on purpose: this slot holds one `Option<String>` that
    // is taken once, and a warning *about the warning channel* would name no
    // consequence the user could act on (FR-003).
    lock::recover(&state.warning).0.take()
}

#[tauri::command]
pub fn add_site(
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

    let write = state.store.lock().add(site.clone());
    state.store.warn_on_write_failure(write);

    state.engine.start(site.clone());
    Ok(site)
}

#[tauri::command]
pub fn update_site(
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

    let write = state.store.lock().update(site.clone());
    state.store.warn_on_write_failure(write);

    // Only this site's timer restarts; every other site is untouched.
    state.engine.reschedule(site.clone());
    Ok(site)
}

#[tauri::command]
pub fn delete_site(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.engine.stop(&id);
    let write = state.store.lock().delete(&id);
    state.store.warn_on_write_failure(write);
    Ok(())
}

use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub fn get_autostart(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

/// Returns the state actually in effect afterwards, so the checkbox can
/// correct itself if the OS refused the change.
#[tauri::command]
pub fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|e| e.to_string())?;
    manager.is_enabled().map_err(|e| e.to_string())
}
