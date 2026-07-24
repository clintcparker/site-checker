mod check;
mod commands;
mod engine;
mod model;
mod store;

use std::sync::{Arc, Mutex};

use tauri::Manager;
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

use commands::AppState;
use engine::Engine;

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

            let store = Arc::new(Mutex::new(loaded.store));
            let engine = Engine::new(app.handle().clone(), Arc::clone(&store));

            let sites = store.lock().unwrap().list();
            engine.start_all(sites);

            app.manage(AppState {
                store,
                engine,
                warning: Mutex::new(loaded.warning),
            });

            // On by default, registered once. A marker file distinguishes
            // "first run" from "the user deliberately turned this off", so
            // unchecking the box sticks across restarts.
            let marker = app.path().app_config_dir()?.join("autostart.initialized");
            if !marker.exists() {
                let _ = app.autolaunch().enable();
                if let Some(parent) = marker.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&marker, b"");
            }

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
