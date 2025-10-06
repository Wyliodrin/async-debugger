use crate::backend::core::StateManager;
use crate::utils::error::Error;
use log::info;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tauri::State as TauriState;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct ExportEntry {
    pub app_dir: String,
    pub title: String,
}

#[derive(Serialize, Deserialize)]
pub struct TimestampEntry {
    pub ts: String,
    pub path: String,
    pub comment_preview: Option<String>,
}

/// Add a new application to be monitored.
///
/// This Tauri-exposed command checks whether an application
/// with the given URL is already registered; if it is not,
/// it converts the URL to [`Url`] and delegates to the
/// [`StateManager`] to store the new application record.
///
/// # Arguments
///
/// * `state_manager` – shared application state manager  
/// * `title` – a human-readable title for the application  
/// * `url` – the process endpoint URL; must be parseable into `Url`
///
/// # Returns
///
/// On success, returns the newly created application's [`Uuid`].
/// If the URL or title was already registered, returns an [`Error::ApplicationAlreadyConnected`].
/// If URL parsing fails, returns the appropriate [`Error`].
#[tauri::command]
pub async fn applications_add(
    state_manager: TauriState<'_, Arc<StateManager>>,
    title: String,
    url: &str,
) -> Result<Uuid, Error> {
    info!("Received command to add application with title {title} and url {url}");
    state_manager.add_application_if_absent(title, url).await
}

/// Delete a monitored application by its UUID.
///
/// # Arguments
///
/// * `state_manager` – shared application state manager  
/// * `uuid` – unique identifier of the application to delete
///
/// # Returns
///
/// Returns `Ok(())` on success, or an [`Error`] on failure.
#[tauri::command]
pub async fn delete_application(
    state_manager: TauriState<'_, Arc<StateManager>>,
    uuid: Uuid,
) -> Result<(), Error> {
    let _ = state_manager.delete_application(uuid).await;
    Ok(())
}

/// Enable an application (i.e. start its connection).
///
/// Finds the application in the current store, asks the
/// connection manager to establish a connection, and
/// sets the application state to `Enabled`.
///
/// # Arguments
///
/// * `state_manager` – shared application state manager  
/// * `uuid` – identifier of the application to enable
///
/// # Errors
///
/// Returns [`Error::Anyhow`] if the app isn’t found or
/// if the connection manager fails to connect.
#[tauri::command]
pub async fn enable_app(
    state_manager: TauriState<'_, Arc<StateManager>>,
    uuid: Uuid,
) -> Result<(), Error> {
    state_manager.enable_app(uuid).await
}

/// Disable a previously enabled application (i.e. tear down its connection).
///
/// # Arguments
///
/// * `state_manager` – shared application state manager  
/// * `uuid` – identifier of the application to disable
///
/// # Returns
///
/// Returns `Ok(())` or an [`Error`] if something goes wrong.
#[tauri::command]
pub async fn disable_app(
    state_manager: TauriState<'_, Arc<StateManager>>,
    uuid: Uuid,
) -> Result<(), Error> {
    state_manager.disable_application(uuid).await
}

/// Edit an application
///
/// # Arguments
///
/// * `state_manager` – shared application state manager
/// * `uuid` - identifier of the application to edit
/// * `app_title` - the new title of the application
/// * `app_url` - the new url of the application
/// * `old_title` - the current title of the application that needs to be edited
///
/// # Returns
///
/// On succes, returns the new Uuid of the application
#[tauri::command]
pub async fn edit_application(
    state_manager: TauriState<'_, Arc<StateManager>>,
    uuid: Uuid,
    app_title: String,
    app_url: String,
    old_title: String,
) -> Result<Uuid, Error> {
    state_manager
        .edit_application(uuid, app_title, app_url, old_title)
        .await
}

/// Update an app’s PID in state and emit it to the frontend.
///
/// # Arguments
/// * `state_manager` – shared application state manager
/// * `app_handle` – handle for emitting Tauri events
/// * `uuid` – application identifier
/// * `new_pid` – freshly discovered process ID
#[tauri::command]
pub async fn update_app_pid(
    state_manager: TauriState<'_, Arc<StateManager>>,
    app_handle: tauri::AppHandle,
    uuid: Uuid,
    new_pid: u32,
) -> Result<(), Error> {
    state_manager.state.handle_pid_changed(uuid, new_pid).await;

    state_manager.emit_update_pid(&app_handle, uuid).await;

    Ok(())
}

/// Fetch the current PID for a given application.
///
/// Frontend can call this after seeing a “pid‐changed” message
/// or poll at intervals.
#[tauri::command]
pub async fn get_app_pid(
    state_manager: TauriState<'_, Arc<StateManager>>,
    uuid: Uuid,
) -> Result<u32, Error> {
    state_manager
        .state
        .get_pid_for(uuid)
        .await
        .ok_or_else(|| Error::Anyhow(anyhow::anyhow!("App {uuid} not found")))
}

#[tauri::command]
pub async fn export_app_instance(
    title: String,
    name: String,
    state: TauriState<'_, Arc<StateManager>>,
) -> Result<String, String> {
    let state = state.inner();
    state
        .state
        .export_app_instance(title, name)
        .await
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| format!("Export error: {}", e))
}

#[tauri::command]
pub async fn list_exports(
    state_manager: TauriState<'_, Arc<StateManager>>,
    storage_folder: String,
) -> Result<Vec<ExportEntry>, String> {
    let exports_base = Path::new(&storage_folder).join("exports");
    state_manager.list_exports_from_base(&exports_base).await
}

#[tauri::command]
pub async fn list_app_timestamps(
    storage_folder: String,
    app_dir: String,
    state_manager: TauriState<'_, Arc<StateManager>>,
) -> Result<Vec<TimestampEntry>, String> {
    state_manager.app_timestamps(storage_folder, app_dir).await
}

#[tauri::command]
pub async fn import_from_export_folder(
    storage_folder: String,
    app_dir: String,
    ts: String,
    state: TauriState<'_, Arc<StateManager>>,
) -> Result<(), String> {
    state
        .import_from_exp_folder(storage_folder, app_dir, ts)
        .await
}
