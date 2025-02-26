use log::info;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

use crate::error::Error;
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
    state_manager.delete_connection(uuid).await;

    Ok(())
}

// pub async fn enable_app(context: State<'_, Arc<Context>>) {}

#[tauri::command]
pub async fn disable_app(
    state_manager: State<'_, Arc<StateManager>>,
    uuid: Uuid,
) -> Result<(), Error> {
    state_manager.disable_application(uuid).await
}
