pub mod commands;
mod common;
mod domain;
mod error;
mod infra;
pub mod macros;
mod mappers;
mod state_manager;

use state_manager::StateManager;
use std::{sync::Arc, time::Duration};
use tauri::{async_runtime, Manager};
use tokio::{task, time::sleep};

use crate::commands::channels::DebugSenderState;

/// Boots and runs the Tauri application, setting up state, background tasks,
/// and UI event loops.
///
/// This function:
/// 1. Initializes the in‐memory and on‐disk application state via `StateManager`.
/// 2. Spawns a background task to process state updates from `StateManager`.
/// 3. Configures Tauri with plugins, IPC commands, and two periodic loops:
///    - A UI update loop that pushes fresh state every second.
///
/// Any failure to initialize persistence will cause a panic.
pub async fn run() {
    // Load the shared application context: state manager plus channels for updates.
    let (state_manager, updates_receiver) = StateManager::new()
        .await
        // TODO: decide whether to panic or degrade gracefully if setup fails
        .unwrap_or_else(|err| panic!("Cannot start application due to {err:?}"));

    // Wrap state manager in an Arc for safe sharing across tasks.
    let shared_state = Arc::new(state_manager);

    // Spawn the core background worker that consumes update messages and applies them.
    {
        let state_manager = shared_state.clone();
        task::spawn(async move {
            state_manager.run(updates_receiver).await;
        });
    }

    // Prepare clones for the two different asynchronous loops below.
    let ui_state_manager = shared_state.clone();

    // Build and configure the Tauri application.
    tauri::Builder::default()
        // Install the standard dialog and shell plugins.
        .plugin(tauri_plugin_dialog::init())
        // Make our shared state available via Tauri’s state‐injection API.
        .manage(shared_state)
        .manage(DebugSenderState::default())
        .setup(move |app| {
            // Workaround: Tonic crate may fail to compile in debug mode unless we
            // explicitly release–build. We keep this comment until upstream fixes it.
            let window = app.get_webview_window("main").unwrap();

            // Automatically open DevTools when running in debug mode.
            #[cfg(debug_assertions)]
            window.open_devtools();

            let app_handle = app.handle().clone();
            crate::commands::debug_server::start_debug_server(&app_handle);
            // Periodically emit UI updates once per second.
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

            Ok(())
        })
        // Register our custom command handlers for IPC invocations from the UI.
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::applications::applications_add,
            commands::applications::delete_application,
            commands::applications::disable_app,
            commands::applications::enable_app,
            commands::applications::edit_application,
            commands::tasks::remove_task,
            commands::tasks::edit_task,
            commands::channels::send_debug_event,
        ])
        // Launch the Tauri event loop with our generated context.
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
