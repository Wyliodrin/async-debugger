use super::{read_field_value_string, read_field_value_u64};
use crate::domain::Task;
use console_api::tasks;
use console_api::tasks::task::Kind;
use uuid::Uuid;

pub fn map_to_domain_task(app_id: Uuid, task: &tasks::Task) -> Option<Task> {
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
