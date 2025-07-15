use crate::state_manager::connection_manager::Event;
use prost::Message;
use serde::Serialize;
use tauri::async_runtime;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "variant", content = "data")]
pub enum SpyEvent {
    Connecting,
    Connected,
    Update(Vec<u8>),
    ApplicationUpdated(SpyAppUpdate),
    Error(String),
    Disconnected,
}

#[derive(Clone, Debug, Serialize)]
pub struct SpyAppUpdate {
    pub cpu_usage: Option<f32>,
    pub memory_usage: u64,
    pub process_status: String,
}

impl From<Event> for SpyEvent {
    fn from(e: Event) -> Self {
        match e {
            Event::Connecting => SpyEvent::Connecting,
            Event::Connected => SpyEvent::Connected,

            Event::Update(upd) => {
                let buf = upd.encode_to_vec();
                SpyEvent::Update(buf)
            }

            Event::ApplicationUpdated(app_upd) => {
                let spy = SpyAppUpdate {
                    cpu_usage: app_upd.cpu_usage,
                    memory_usage: app_upd.memory_usage,
                    process_status: format!("{:?}", app_upd.process_status),
                };
                SpyEvent::ApplicationUpdated(spy)
            }

            Event::Error(err) => SpyEvent::Error(err.to_string()),
            Event::Disconnected => SpyEvent::Disconnected,
        }
    }
}

#[derive(Clone)]
pub struct SpySender {
    inner: async_runtime::Sender<(Uuid, Event)>,
    spy: async_runtime::Sender<(Uuid, SpyEvent)>,
}

impl SpySender {
    pub fn new(
        inner: async_runtime::Sender<(Uuid, Event)>,
        spy: async_runtime::Sender<(Uuid, SpyEvent)>,
    ) -> Self {
        Self { inner, spy }
    }

    // Async send that duplicates payloads
    pub async fn send(
        &self,
        (id, evt): (Uuid, Event),
    ) -> Result<(), mpsc::error::SendError<(Uuid, Event)>> {
        self.inner.send((id, evt)).await?;
        //let spy_evt = SpyEvent::from(evt);
        // we ignore any error on the spy side
        //let _ = self.spy.send((id, spy_evt)).await;
        Ok(())
    }

    // Non-blocking variant
    // pub fn try_send(
    //     &self,
    //     (id, evt): (Uuid, Event),
    // ) -> Result<(), mpsc::error::TrySendError<(Uuid, Event)>> {
    //     self.inner.try_send((id, evt.clone()))?;
    //     let spy_evt = SpyEvent::from(evt);
    //     let _ = self.spy.try_send((id, spy_evt));
    //     Ok(())
    // }
}
