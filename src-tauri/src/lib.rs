use std::{collections::HashMap, error::Error, time::Duration};

use console_api::{
    field::{Name, Value},
    instrument::{instrument_client::InstrumentClient, InstrumentRequest, Update},
    tasks::{self, task::Kind},
    Field,
};
use serde::Serialize;
use tauri::{async_runtime, Emitter};
use tonic::{transport::Endpoint, Streaming};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[derive(Serialize, Clone, Debug)]
struct Task {
    id: u64,
    tid: Option<u64>,
    name: Option<String>,
    kind: Option<String>,
}

fn find_field(task: &tasks::Task, field_name: impl AsRef<str>) -> Option<&Field> {
    task.fields.iter().find(|field| {
        if let Some(Name::StrName(ref s)) = field.name {
            s == field_name.as_ref()
        } else {
            false
        }
    })
}

fn read_field_value_u64(task: &tasks::Task, field_name: impl AsRef<str>) -> Option<u64> {
    if let Some(field) = find_field(task, field_name) {
        match field.value {
            Some(Value::U64Val(value)) => Some(value),
            _ => None,
        }
    } else {
        None
    }
}

fn read_field_value_string(task: &tasks::Task, field_name: impl AsRef<str>) -> Option<&str> {
    if let Some(field) = find_field(task, field_name) {
        match field.value {
            Some(Value::DebugVal(ref value)) | Some(Value::StrVal(ref value)) => Some(value),
            _ => None,
        }
    } else {
        None
    }
}

fn read_task(task: &tasks::Task) -> Option<Task> {
    let id = task.id.map(|value| value.id)?;
    let tid = read_field_value_u64(task, "task.id");
    let name = read_field_value_string(task, "task.name").map(|s| s.to_owned());
    let kind = Kind::try_from(task.kind)
        .map(|kind| kind.as_str_name().to_owned())
        .ok();
    Some(Task {
        id,
        tid,
        name,
        kind,
    })
}

async fn connect() -> Result<Box<Streaming<Update>>, Box<dyn Error + Send + Sync>> {
    let endpoint = Endpoint::from_static("http://localhost:6669");
    let channel = endpoint.connect().await?;
    let mut client = InstrumentClient::new(channel);
    let update_request = tonic::Request::new(InstrumentRequest {});
    Ok(Box::new(
        client.watch_updates(update_request).await?.into_inner(),
    ))
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();
            async_runtime::spawn(async move {
                loop {
                    match connect().await {
                        Ok(mut update_stream) => {
                            let mut tasks = HashMap::new();
                            while let Ok(m) = update_stream.message().await {
                                let Some(update) = m else { continue };
                                println!("update {:?}", update.now);
                                if let Some(task_update) = update.task_update {
                                    for task in task_update.new_tasks {
                                        if let Some(task) = read_task(&task) {
                                            tasks.insert(task.id, task);
                                        }
                                    }

                                    for (tid, updated_task) in task_update.stats_update {
                                        if updated_task.dropped_at.is_some() {
                                            tasks.remove(&tid);
                                        }
                                    }

                                    println!("new tasks: {:?}", tasks);
                                    app_handle
                                        .emit(
                                            "task-update",
                                            tasks
                                                .iter()
                                                .map(|(_, value)| value)
                                                .collect::<Vec<&Task>>(),
                                        )
                                        .ok();
                                }

                                // if let Some(async_op) = update.async_op_update {
                                //     println!("{:#?}", async_op);
                                // }
                            }
                            println!("update stteam disconnected");
                        }
                        Err(error) => {
                            eprintln!("{:?}", error);
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    }
                }
            });
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
