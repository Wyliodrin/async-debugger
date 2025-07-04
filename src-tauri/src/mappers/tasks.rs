use super::{read_field_value_string, read_field_value_u64};
use crate::domain::{Task, TaskState};
use console_api::tasks;
use console_api::tasks::task::Kind;
use uuid::Uuid;

pub fn map_to_domain_task(_app_id: Uuid, task: &tasks::Task) -> Option<Task> {
    let app_name = task
        .location
        .as_ref()
        .map(|loc| loc.to_string());

    let id = task.id.as_ref().map(|v| v.id)?;
    let tid = read_field_value_u64(task, "task.id");
    let name = read_field_value_string(task, "task.name").map(|s| s.to_owned());
    let kind = Kind::try_from(task.kind)
        .map(|k| k.as_str_name().to_owned())
        .ok();

    Some(Task {
        app_name,
        id,
        tid,
        name,
        kind,
        state: TaskState::Running,
    })
}
