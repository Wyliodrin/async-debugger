use super::{read_field_value_string, read_field_value_u64};
use crate::domain::{Task, TaskLocation, TaskStats};
use console_api::tasks::task::Kind;
use console_api::tasks::{self, Stats};
use uuid::Uuid;

pub fn map_to_domain_task(
    app_id: Uuid,
    task: &tasks::Task,
    task_stats_recv: &Stats,
) -> Option<Task> {
    let id = task.id.map(|value| value.id)?;
    let tid = read_field_value_u64(task, "task.id");
    let name = read_field_value_string(task, "task.name").map(|s| s.to_owned());
    let kind = Kind::try_from(task.kind)
        .map(|kind| kind.as_str_name().to_owned())
        .ok();
    let location = task.location.as_ref().map(TaskLocation::from);
    let task_stats = TaskStats::from(task_stats_recv);

    Some(Task {
        app_id,
        id,
        tid,
        name,
        kind,
        location,
        task_stats,
    })
}
