use std::sync::Mutex;

use tauri::State;

use crate::engine::Engine;
use crate::lock::{self, SharedStore};
use crate::model::{clamp_interval, normalize_url, Site};
use crate::store::{AddError, Replaced};

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

    // The two failures owe the user opposite answers, so this is where they part.
    //
    // A refusal means nothing was applied anywhere — no row may appear and no
    // timer may start, so it returns *above* `engine.start`. It raises no banner
    // either: the message below is the whole story, and a second "could not be
    // saved" alongside it would contradict it.
    //
    // A write failure keeps every one of today's behaviours: the change stands
    // in memory, the row appears, checks begin, and the banner says it could not
    // be saved.
    // Bound to a `let` so the store guard drops here rather than living on as a
    // `match` scrutinee temporary — those survive to the end of the match, which
    // would hold the store lock across the banner emit below. Not a deadlock
    // today, but the same lock-ordering hazard `update_site` is careful about,
    // and it costs one line to not have to reason about it.
    let stored = state.store.lock().add(site.clone());

    match stored {
        Err(AddError::DuplicateId(_)) => {
            return Err(
                "That site was not added — the list already has an entry with the same \
                 identity. Nothing was changed."
                    .to_string(),
            )
        }
        Err(AddError::Write(message)) => state.store.warn_on_write_failure(Err(message)),
        Ok(()) => {}
    }

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
    // A bad URL is rejected before anything else happens, exactly as before.
    let url = normalize_url(&url)?;

    // One lock, one scope. `Store::replace` reads the current entry, decides the
    // learned request method from it, writes, and saves under a single borrow —
    // the decision that used to live here moved there so no second edit can land
    // between the read and the write.
    //
    // The guard is bound to a name inside an explicit scope so it is *provably*
    // released before `reschedule` below. Left as a `let ... else` temporary it
    // would live to the end of the statement, holding the store lock across a
    // scheduling call. Not a deadlock today — but a lock-ordering hazard nobody
    // should have to re-derive later, and this costs two braces.
    let replaced = {
        let mut store = state.store.lock();
        store.replace(
            &id,
            url,
            empty_to_none(label),
            clamp_interval(interval_secs),
        )
    };

    let Replaced { site, write } =
        replaced.ok_or_else(|| "That site no longer exists".to_string())?;

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
