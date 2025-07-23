use tauri::{AppHandle, Emitter};
use tonic::transport::Server;
use tonic::{Request, Response, Status};

pub mod debug_proto {
    tonic::include_proto!("async_debugger");
}
use debug_proto::debug_channel_server::{DebugChannel, DebugChannelServer};
use debug_proto::DebugEvent;

pub struct DebugService {
    app_handle: AppHandle,
}

impl DebugService {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

#[tonic::async_trait]
impl DebugChannel for DebugService {
    async fn send_event(&self, request: Request<DebugEvent>) -> Result<Response<()>, Status> {
        let evt = request.into_inner();

        #[derive(serde::Serialize, Clone)]
        struct Payload {
            name: String,
            kind: String,
            ts: i64,
            payload: String,
        }

        let payload = Payload {
            name: evt.name,
            kind: evt.kind,
            ts: evt.ts,
            payload: evt.payload,
        };

        let app = self.app_handle.clone();
        tokio::spawn(async move {
            let _ = app.emit("debug-event", payload);
        });

        Ok(Response::new(()))
    }
}

pub fn start_debug_server(app_handle: &AppHandle) {
    let addr = "127.0.0.1:50051".parse().unwrap();
    let svc = DebugService::new(app_handle.clone());

    tauri::async_runtime::spawn(async move {
        println!("gRPC DebugChannel listening on {}", addr);
        if let Err(e) = Server::builder()
            .add_service(DebugChannelServer::new(svc))
            .serve(addr)
            .await
        {
            eprintln!("gRPC server error: {e}");
        }
    });
}
