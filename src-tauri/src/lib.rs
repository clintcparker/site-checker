mod check;
mod commands;
mod engine;
mod lock;
mod model;
mod store;

use std::sync::Mutex;

use tauri::Manager;
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

use commands::AppState;
use engine::Engine;
use lock::SharedStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .setup(|app| {
            // ~/Library/Application Support/com.clintparker.site-checker/
            let path = app.path().app_config_dir()?.join("sites.json");
            let loaded = store::load(path);

            let store = SharedStore::new(app.handle().clone(), loaded.store);
            let engine = Engine::new(app.handle().clone(), store.clone());

            // A warning raised by *this* lock would be dropped: the window's JS
            // has not registered its `store-warning` listener yet and Tauri
            // events have no replay. That is inert rather than a missing case —
            // the store was constructed on the line above and cannot have been
            // poisoned yet. Noted so a future reader doesn't take it for a gap.
            let sites = store.lock().list();
            engine.start_all(sites);

            let mut warning = loaded.warning;

            // On by default, registered once. A marker file distinguishes
            // "first run" from "the user deliberately turned this off", so
            // unchecking the box sticks across restarts.
            let marker = app.path().app_config_dir()?.join("autostart.initialized");
            if !marker.exists() {
                if let Err(e) = app.autolaunch().enable() {
                    // Store-load warning takes precedence if one already
                    // occurred; don't clobber it with the less-critical
                    // autostart message.
                    warning.get_or_insert_with(|| format!(
                        "Could not turn on Launch at login ({e}). Tick the box below to try again."
                    ));
                }
                // The marker is written whether or not enable() succeeded,
                // so a later deliberate untick is never re-enabled on the
                // next launch.
                if let Some(parent) = marker.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&marker, b"");
            }

            app.manage(AppState {
                store,
                engine,
                warning: Mutex::new(warning),
            });

            Ok(())
        })
        // Closing the window quits the app. Without this, macOS keeps the
        // process alive with no window, which the spec explicitly rules out.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                window.app_handle().exit(0);
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_sites,
            commands::get_warning,
            commands::add_site,
            commands::update_site,
            commands::delete_site,
            commands::get_autostart,
            commands::set_autostart,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
