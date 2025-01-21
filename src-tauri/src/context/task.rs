use std::sync::Arc;

use console_api::{
    field::{Name, Value},
    tasks::{self, task::Kind, TaskUpdate},
    Field,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use super::Context;

#[derive(Serialize, Clone, Debug)]
pub struct Task {
    pub app_id: Uuid,
    pub id: u64,
    pub tid: Option<u64>,
    pub name: Option<String>,
    pub kind: Option<String>,
}

impl Task {
    pub fn id(&self) -> String {
        format!("{}.{}", self.app_id, self.id)
    }
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

pub fn read_task(app_id: Uuid, task: &tasks::Task) -> Option<Task> {
    let id = task.id.map(|value| value.id)?;
    let tid = read_field_value_u64(task, "task.id");
    let name = read_field_value_string(task, "task.name").map(|s| s.to_owned());
    let kind = Kind::try_from(task.kind)
        .map(|kind| kind.as_str_name().to_owned())
        .ok();
    Some(Task {
        app_id,
        id,
        tid,
        name,
        kind,
    })
}

impl Context {
    pub async fn handle_task_update(&self, app_id: Uuid, task_update: TaskUpdate) {
        for task in task_update.new_tasks {
            if let Some(task) = read_task(app_id, &task) {
                self.tasks.write().await.insert(task.id(), Arc::new(task));
            }
        }

        for (tid, updated_task) in task_update.stats_update {
            if updated_task.dropped_at.is_some() {
                self.tasks
                    .write()
                    .await
                    .remove(&format!("{}.{}", app_id, tid));
            }
        }
    }

    pub async fn emit_update_tasks(&self, app_handle: &AppHandle) {
        app_handle
            .emit(
                "update:tasks",
                self.tasks
                    .read()
                    .await
                    .iter()
                    .map(|(_, value)| value)
                    .collect::<Vec<&Arc<Task>>>(),
            )
            .ok();
    }

    pub async fn emit_update_applications(&self, app_handle: &AppHandle) {
        app_handle
            .emit("update:applications", self.applications().await)
            .ok();
    }
}
