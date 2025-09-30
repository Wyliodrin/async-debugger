use crate::error::Error;
use crate::state_manager::connection_manager::Connection;
use crate::state_manager::StateManager;
use log::info;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tauri::State as TauriState;
use tokio::fs;
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

    let applications = state_manager.current_applications().await;
    for app in applications {
        if app.url().to_string() == url {
            return Err(Error::ApplicationAlreadyConnected(url.into()));
        }
        if app.title() == title {
            return Err(Error::ApplicationAlreadyConnected(title));
        }
    }
    let url = url.try_into()?;
    state_manager.add_application(title, url).await
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
    let apps = state_manager.state.get_current_applications_list().await;
    let app = apps
        .iter()
        .find(|a| a.id() == &uuid)
        .ok_or_else(|| Error::Anyhow(anyhow::anyhow!("App {uuid} not found")))?;

    // ask the existing connection manager to connect.
    let conn: Connection = state_manager
        .connection_manager
        .connect_app(*app.id(), app.url().clone(), app.pid())
        .await?;

    // flip state to Enabled and stash the new connection
    info!("enable_app: {uuid}");
    state_manager.state.enable_app(uuid, conn).await;
    Ok(())
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
pub async fn list_exports(storage_folder: String) -> Result<Vec<ExportEntry>, String> {
    let exports_base = Path::new(&storage_folder).join("exports");
    let mut out: Vec<ExportEntry> = Vec::new();

    let read_dir = match fs::read_dir(&exports_base).await {
        Ok(rd) => rd,
        Err(_) => return Ok(out),
    };

    let mut dir = read_dir;
    while let Some(entry) = dir.next_entry().await.map_err(|e| e.to_string())? {
        let file_type = entry.file_type().await.map_err(|e| e.to_string())?;
        if file_type.is_dir() {
            let folder_name = entry.file_name().to_string_lossy().to_string();
            let title = folder_name.replace('-', " ");
            out.push(ExportEntry {
                app_dir: folder_name,
                title,
            });
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn list_app_timestamps(
    storage_folder: String,
    app_dir: String,
) -> Result<Vec<TimestampEntry>, String> {
    let app_folder = Path::new(&storage_folder).join("exports").join(&app_dir);
    let mut out: Vec<TimestampEntry> = Vec::new();

    let read_dir = match fs::read_dir(&app_folder).await {
        Ok(rd) => rd,
        Err(_) => return Ok(out),
    };

    let mut dir = read_dir;
    while let Some(entry) = dir.next_entry().await.map_err(|e| e.to_string())? {
        let meta = entry.file_type().await.map_err(|e| e.to_string())?;
        if meta.is_dir() {
            let ts_name = entry.file_name().to_string_lossy().to_string();
            let comment_path = app_folder.join(&ts_name).join("comment.txt");
            let comment_preview = match fs::read_to_string(&comment_path).await {
                Ok(s) => {
                    let s = s.trim().to_string();
                    if s.is_empty() {
                        None
                    } else {
                        Some(if s.len() > 200 {
                            s[..200].to_string() + "..."
                        } else {
                            s
                        })
                    }
                }
                Err(_) => None,
            };
            out.push(TimestampEntry {
                ts: ts_name.clone(),
                path: app_folder.join(&ts_name).to_string_lossy().to_string(),
                comment_preview,
            });
        }
    }

    out.sort_by(|a, b| b.ts.cmp(&a.ts));
    Ok(out)
}

#[tauri::command]
pub async fn import_from_export_folder(
    storage_folder: String,
    app_dir: String,
    ts: String,
    state: TauriState<'_, Arc<StateManager>>,
) -> Result<(), String> {
    let base = Path::new(&storage_folder)
        .join("exports")
        .join(&app_dir)
        .join(&ts);

    if !base.exists() {
        return Err(format!(
            "Export folder not found: {}",
            base.to_string_lossy()
        ));
    }
    state
        .inner()
        .state
        .import_from_export_folder(base)
        .await
        .map_err(|e| format!("Import error: {}", e))?;

    Ok(())
}
