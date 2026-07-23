mod check;
mod commands;
mod engine;
mod model;
mod store;

use std::sync::{Arc, Mutex};

use tauri::Manager;

use commands::AppState;
use engine::Engine;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
