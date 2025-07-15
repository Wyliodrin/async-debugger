use super::{read_field_value_string, read_field_value_u64};
use crate::domain::{Task, TaskState};
use console_api::tasks;
use console_api::tasks::task::Kind;
use uuid::Uuid;

pub fn map_to_domain_task(_app_id: Uuid, task: &tasks::Task) -> Option<Task> {
    let id = task.id.as_ref().map(|v| v.id)?;
    let tid = read_field_value_u64(task, "task.id");
    let name = read_field_value_string(task, "task.name").map(|s| s.to_owned());
    let kind = Kind::try_from(task.kind)
        .map(|k| k.as_str_name().to_owned())
        .ok();
    let location = task.location.as_ref().map(|loc| {
        let full_path = loc.file();
        let path = std::path::Path::new(full_path);

        let directory = path.parent().and_then(|p| p.to_str()).unwrap_or("");
        let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");

        format!(
            "{}{}<b>{}</b>:<b>{}</b>:{}",
            directory,
            std::path::MAIN_SEPARATOR,
            filename,
            loc.line(),
            loc.column()
        )
    });

    Some(Task {
        app_name: None,
        id,
        tid,
        name,
        color: None,
        kind,
        state: TaskState::Running,
        runtime: None,
        scheduled: None,
        idle: None,
        busy: None,
        location,
        created_at: None,
    })
}
