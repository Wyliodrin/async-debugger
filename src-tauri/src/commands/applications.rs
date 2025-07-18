use crate::error::Error;
use crate::state_manager::connection_manager::Connection;
use crate::state_manager::StateManager;
use log::info;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

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
/// If the URL was already registered, returns an [`Error::ApplicationAlreadyConnected`].
/// If URL parsing fails, returns the appropriate [`Error`].
#[tauri::command]
pub async fn applications_add(
    state_manager: State<'_, Arc<StateManager>>,
    title: String,
    url: &str,
) -> Result<Uuid, Error> {
    info!("Received command to add application with title {title} and url {url}");

    let applications = state_manager.current_applications().await;
    for app in applications {
        if app.url().to_string() == url {
            return Err(Error::ApplicationAlreadyConnected(url.into()));
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
    state_manager: State<'_, Arc<StateManager>>,
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
    state_manager: State<'_, Arc<StateManager>>,
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
    state_manager: State<'_, Arc<StateManager>>,
    uuid: Uuid,
) -> Result<(), Error> {
    state_manager.disable_application(uuid).await
}
