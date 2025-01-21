use std::{sync::Arc, time::Duration};
use tauri::{async_runtime, Manager};
use tokio::{sync::mpsc, time::sleep};

mod commands;
mod context;
mod error;

use context::{connection::Event, Context};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

pub fn run() {
    let (event_sender, mut event_receiver) = mpsc::channel(100);
    let shared_context = Arc::new(Context::new(event_sender));
    let context_update_ui = shared_context.clone();
    let context = shared_context.clone();

    tauri::Builder::default()
        .setup(move |app| {
            // FIX: workaround for the compilation error of the tonic crate,
            //      we need to compile using `--release` for now
            let window = app.get_webview_window("main").unwrap();
            window.open_devtools();
            // window.close_devtools();

            let app_handle = app.handle().clone();

            // update ui
            // TODO needs to be improved
            async_runtime::spawn(async move {
                loop {
                    sleep(Duration::from_secs(1)).await;
                    context_update_ui
                        .emit_update_applications(&app_handle)
                        .await;
                    context_update_ui.emit_update_tasks(&app_handle).await;
                }
            });

            // event loop
            async_runtime::spawn(async move {
                while let Some((app_id, event)) = event_receiver.recv().await {
                    match event {
                        Event::Update(update) => {
                            if let Some(task_update) = update.task_update {
                                context.handle_task_update(app_id, task_update).await;

                                // println!("new tasks: {:?}", tasks);
                            }
                        }
                        _ => {}
                    }
                }
            });
            Ok(())
        })
        .manage(shared_context)
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::applications::applications_add
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
