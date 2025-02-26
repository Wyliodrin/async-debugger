mod commands;
mod domain;
mod error;
mod infra;
mod mappers;
mod state_manager;
mod ui_manager;

use state_manager::StateManager;
use std::{sync::Arc, time::Duration};
use tauri::{async_runtime, Manager};
use tokio::{task, time::sleep};

pub async fn run() {
    // Load context
    let (state_manager, updates_receiver) = StateManager::new()
        .await
        // TODO: should we panic here or disable the persistency?
        .unwrap_or_else(|err| panic!("Cannot start application due to {err:?}"));

    let shared_state = Arc::new(state_manager);

    // Start job
    let state_manager = shared_state.clone();
    task::spawn(async move {
        state_manager.run(updates_receiver).await;
    });

    // Clone for ui_updates
    let ui_state_manager = shared_state.clone();
    tauri::Builder::default()
        .manage(shared_state)
        .setup(move |app| {
            // FIX: workaround for the compilation error of the tonic crate,
            //      we need to compile using `--release` for now
            let window = app.get_webview_window("main").unwrap();

            // Open dev tools if in debug build
            #[cfg(debug_assertions)]
            {
                window.open_devtools();
            }

            let app_handle = app.handle().clone();

            // update ui once per second
            // TODO could be improved
            async_runtime::spawn(async move {
                loop {
                    sleep(Duration::from_secs(1)).await;
                    ui_state_manager.emit_update_applications(&app_handle).await;
                    ui_state_manager.emit_update_tasks(&app_handle).await;
                }
            });

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::applications::applications_add,
            commands::applications::delete_application,
            commands::applications::disable_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
