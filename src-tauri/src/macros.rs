use crate::commands::debug_server::debug_proto::debug_channel_client::DebugChannelClient;
use std::sync::Arc;
use tokio::sync::{Mutex as TokioMutex, OnceCell};
use tonic::transport::Channel;

static DEBUG_CLIENT: OnceCell<Arc<TokioMutex<DebugChannelClient<Channel>>>> = OnceCell::const_new();

pub async fn init_debug_client() -> anyhow::Result<Arc<TokioMutex<DebugChannelClient<Channel>>>> {
    if let Some(client) = DEBUG_CLIENT.get() {
        return Ok(client.clone());
    }

    let client = DebugChannelClient::connect("http://127.0.0.1:50051").await?;
    let arc = Arc::new(TokioMutex::new(client));
    DEBUG_CLIENT.set(arc.clone()).unwrap();
    Ok(arc)
}

#[macro_export]
macro_rules! debug_event {
    (
        $client:expr,
        payload: $payload:expr
    ) => {{
        let proto_evt = {
            use async_debugger::commands::debug_server::debug_proto::DebugEvent;
            use chrono::Utc;
            DebugEvent {
                name: "client".to_string(),
                kind: "mpsc".to_string(),
                ts: Utc::now().timestamp_millis(),
                payload: serde_json::to_string(&$payload).unwrap(),
            }
        };

        let client = $client.clone();
        tokio::spawn(async move {
            let mut guard = client.lock().await;
            if let Err(e) = guard.send_event(proto_evt).await {
                eprintln!("DebugChannel gRPC send failed: {}", e);
            }
        });
    }};
}
