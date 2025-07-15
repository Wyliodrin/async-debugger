use crate::error::Error;
use crate::state_manager::StateManager;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn remove_task(
    state_manager: State<'_, Arc<StateManager>>,
    task_id: String,
) -> Result<(), Error> {
    state_manager.state.stop_task(&task_id).await;
    Ok(())
}

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
