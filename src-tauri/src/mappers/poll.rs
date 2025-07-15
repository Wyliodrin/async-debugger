use crate::domain::poll::Poll;
use console_api::resources::PollOp;
pub fn map_to_domain_poll(poll: &PollOp) -> Option<Poll> {
    let resource_id = match poll.resource_id {
        Some(id) => Some(id.id),
        None => None,
    };
    let resource_name = Some("Unknown".into());

    let name = poll.name.clone();

    let task_id = match poll.task_id {
        Some(id) => Some(id.id),
        None => None,
    };
    let task_name = Some("No Name".into());

    let is_ready = poll.is_ready;

    Some(Poll {
        app_name: None,
        poll_type: name,
        resource_id,
        resource_name,
        task_id,
        task_name,
        task_color: None,
        is_ready,
        location: None,
        received_at: None,
    })
}
