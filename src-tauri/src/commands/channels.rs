use serde_json::Value;
use tokio::sync::{mpsc, Mutex};

use crate::commands::debug_server::debug_proto::DebugEvent;

#[derive(Default)]
pub struct DebugSenderState {
    inner: Mutex<Option<mpsc::Sender<DebugEvent>>>,
}

#[derive(Debug)]
pub struct RawDebugEvent {
    pub payload: Value,
}

impl Clone for RawDebugEvent {
    fn clone(&self) -> Self {
        RawDebugEvent {
            payload: self.payload.clone(),
        }
    }
}

#[tauri::command]
pub async fn send_debug_event(
    state: tauri::State<'_, DebugSenderState>,
    name: String,
    kind: String,
    payload: String,
) -> Result<(), String> {
    let guard = state.inner.lock().await;
    let sender = guard
        .as_ref()
        .ok_or_else(|| "Debug sender not started. Call start_debug_sender first.".to_string())?;

    let evt = DebugEvent {
        name,
        kind,
        ts: chrono::Utc::now().timestamp_millis(),
        payload,
    };

    sender
        .send(evt)
        .await
        .map_err(|e| format!("failed to enqueue debug event: {}", e))?;

    Ok(())
}
