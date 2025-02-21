pub(crate) mod tasks;

use crate::error::Error as TraceError;
use console_api::{
    field::{Name, Value},
    tasks::Task as ConsoleTask,
    Field,
};
use log::error;
use tokio::fs::read_to_string;

// UTILS METHODS (could be moved in a dedicated module)

fn find_field(task: &ConsoleTask, field_name: impl AsRef<str>) -> Option<&Field> {
    task.fields.iter().find(|field| {
        if let Some(Name::StrName(ref s)) = field.name {
            s == field_name.as_ref()
        } else {
            false
        }
    })
}

fn read_field_value_u64(task: &ConsoleTask, field_name: impl AsRef<str>) -> Option<u64> {
    if let Some(field) = find_field(task, field_name) {
        match field.value {
            Some(Value::U64Val(value)) => Some(value),
            _ => None,
        }
    } else {
        None
    }
}

fn read_field_value_string(task: &ConsoleTask, field_name: impl AsRef<str>) -> Option<&str> {
    if let Some(field) = find_field(task, field_name) {
        match field.value {
            Some(Value::DebugVal(ref value)) | Some(Value::StrVal(ref value)) => Some(value),
            _ => None,
        }
    } else {
        None
    }
}

pub async fn read_file(filename: &str) -> Result<String, TraceError> {
    read_to_string(filename).await.map_err(|err| {
        error!("Failed to load {filename} ({err:?})");
        TraceError::PathNotFound(filename.to_string())
    })
}
