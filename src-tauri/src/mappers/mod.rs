//! Central re‐exports and utility functions for mapping console_api data
//! into our domain models.

pub(crate) mod async_ops;
pub(crate) mod poll;
pub(crate) mod resources;
pub(crate) mod tasks;

use crate::error::Error as TraceError;
use console_api::{
    field::{Name, Value},
    tasks::Task as ConsoleTask,
    Field,
};
use log::error;
use tokio::fs::read_to_string;

/// Find a given named field in a console API task.
fn find_field(task: &ConsoleTask, field_name: impl AsRef<str>) -> Option<&Field> {
    task.fields.iter().find(|field| {
        if let Some(Name::StrName(ref s)) = field.name {
            s == field_name.as_ref()
        } else {
            false
        }
    })
}

/// Read a `u64` value from a named field in a task.
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

fn read_field_value_usize(task: &ConsoleTask, field_name: impl AsRef<str>) -> Option<usize> {
    if let Some(field) = find_field(task, field_name) {
        match field.value {
            Some(Value::U64Val(value)) => Some(value as usize),
            _ => None,
        }
    } else {
        None
    }
}

/// Read a string value from a named field in a task.
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

/// Asynchronously reads an entire file into a `String`.
///
/// # Errors
///
/// Returns `TraceError::PathNotFound(filename)` if the file can’t be opened.
pub async fn read_file(filename: &str) -> Result<String, TraceError> {
    read_to_string(filename).await.map_err(|err| {
        error!("Failed to load {filename} ({err:?})");
        TraceError::PathNotFound(filename.to_string())
    })
}
