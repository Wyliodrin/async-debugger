use crate::error::Error;
use crate::state_manager::StateManager;
use std::sync::Arc;
use tauri::State;

/// Remove a running or stopped task by its ID.
///
/// # Arguments
///
/// * `state_manager` – shared application state manager  
/// * `task_id` – the string identifier of the task
///
/// # Returns
///
/// Returns `Ok(())` on success, or an [`Error`] on failure.
#[tauri::command]
pub async fn remove_task(
    state_manager: State<'_, Arc<StateManager>>,
    task_id: String,
) -> Result<(), Error> {
    state_manager.state.stop_task(&task_id).await;
    Ok(())
}

/// Edit an existing task’s metadata (name, color, associated app).
///
/// # Arguments
///
/// * `state_manager` – shared application state manager  
/// * `task_id` – numeric ID of the task  
/// * `task_name` – new name for the task  
/// * `task_color` – new color code for the task  
/// * `app_name` – (optional) name of the application this task belongs to
///
/// # Returns
///
/// Returns `Ok(())` or an [`Error`].
#[tauri::command]
pub async fn edit_task(
    state_manager: State<'_, Arc<StateManager>>,
    task_id: u64,
    task_name: String,
    task_color: String,
    app_name: String,
) -> Result<(), Error> {
    state_manager
        .state
        .rename_task(
            task_id,
            task_name.clone(),
            task_color.clone(),
            app_name.clone(),
        )
        .await;
    Ok(())
}
