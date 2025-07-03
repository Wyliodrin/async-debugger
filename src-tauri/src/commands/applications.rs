use log::info;
use tokio::sync::mpsc;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

use crate::error::Error;
use crate::state_manager::connection_manager::{Connection, ConnectionManager};
use crate::state_manager::StateManager;

#[tauri::command]
pub async fn applications_add(
    state_manager: State<'_, Arc<StateManager>>,
    title: String,
    url: &str,
) -> Result<Uuid, Error> {
    info!("Received command to add application with title {title} and url {url}");

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
    info!("enable_app: {uuid}");

    // 1) look up the URL from state
    let (updates_sender, _updates_receiver) = mpsc::channel(100);
    let apps = state_manager.state.get_current_applications_list().await;
    let app = apps.iter()
        .find(|a| a.id() == &uuid)
        .ok_or_else(|| Error::Anyhow(anyhow::anyhow!("App {uuid} not found")))?;

    // 2) reconnect
    let manager = ConnectionManager::new(updates_sender);
    let conn: Connection = 
        manager
        .connect_app(*app.id(), app.url().clone(), app.pid())
        .await?;

    // 3) flip state to Enabled and stash the new connection
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
