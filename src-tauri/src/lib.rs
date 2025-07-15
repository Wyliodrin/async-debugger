mod commands;
mod common;
mod domain;
mod error;
mod infra;
mod mappers;
mod state_manager;

use state_manager::StateManager;
use std::{sync::Arc, time::Duration};
use tauri::{async_runtime, Emitter, Manager};
use tokio::{task, time::sleep};

pub async fn run() {
    // Load context
    let (state_manager, updates_receiver, spy_rx) = StateManager::new()
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

    //Clone for spy_updates
    let spy_state = shared_state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
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
                    ui_state_manager.emit_update_resources(&app_handle).await;
                    ui_state_manager.emit_update_polls(&app_handle).await;
                    ui_state_manager.emit_update_tasks_op(&app_handle).await;
                }
            });

            let mut spy_rx = spy_rx;
            let window_clone = window.clone();
            async_runtime::spawn(async move {
                while let Some((id, spy_evt)) = spy_rx.recv().await {
                    let mut payload = serde_json::json!({
                    "id":    id.to_string(),
                    "event": spy_evt,
                    });
                    if let Some(app_name) = spy_state.state.get_application_name_by_id(&id).await {
                        payload = serde_json::json!({
                        "id":    app_name,
                        "event": spy_evt,
                        });
                    }
                    if let Err(e) = window_clone.emit("spy:event", payload) {
                        eprintln!("failed to emit spy:event: {e:?}");
                    }
                }
            });

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::applications::applications_add,
            commands::applications::delete_application,
            commands::applications::disable_app,
            commands::applications::enable_app,
            commands::tasks::remove_task,
            commands::tasks::edit_task,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
