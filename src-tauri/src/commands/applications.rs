use crate::error::Error;
use crate::state_manager::connection_manager::Connection;
use crate::state_manager::StateManager;
use log::info;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

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

#[tauri::command]
pub async fn delete_application(
    state_manager: State<'_, Arc<StateManager>>,
    uuid: Uuid,
) -> Result<(), Error> {
    let _ = state_manager.delete_application(uuid).await;

    Ok(())
}

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

    // ask the existing connection manager to connect.  That connection manager has already been built with the SpySender and 2 channels
    let conn: Connection = state_manager
        .connection_manager
        .connect_app(*app.id(), app.url().clone(), app.pid())
        .await?;

    // flip state to Enabled and stash the new connection
    info!("enable_app: {uuid}");
    state_manager.state.enable_app(uuid, conn).await;
    Ok(())
}

#[tauri::command]
pub async fn disable_app(
    state_manager: State<'_, Arc<StateManager>>,
    uuid: Uuid,
) -> Result<(), Error> {
    state_manager.disable_application(uuid).await
}

#[tauri::command]
pub async fn remove_task(
    state_manager: State<'_, Arc<StateManager>>,
    task_id: String,
) -> Result<(), Error> {
    state_manager.state.stop_task(&task_id).await;
    Ok(())
}
