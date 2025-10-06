use crate::backend::core::warnings::TaskWarnings;
use crate::backend::core::StateManager;
use crate::utils::error::Error;
use std::sync::Arc;
use tauri::State;

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
    warnings: TaskWarnings,
) -> Result<(), Error> {
    state_manager
        .state
        .edit_state_task(
            task_id,
            task_name.clone(),
            task_color.clone(),
            app_name.clone(),
            warnings.clone(),
        )
        .await
}
