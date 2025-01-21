use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

use crate::context::Context;
use crate::error::Error;

#[tauri::command]
pub async fn applications_add(
    context: State<'_, Arc<Context>>,
    title: String,
    url: &str,
) -> Result<Uuid, Error> {
    let url = url.try_into()?;
    Ok(context.add_application(title, url).await)
}
