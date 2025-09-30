//! Convert from `console_api::resources::PollOp` to our domain `Poll`.

use crate::domain::poll::Poll;
use console_api::resources::PollOp;

/// Map fields from the upstream PollOp into our `Poll`.
/// Always returns `Some(Poll)` as we provide defaults for missing data.
pub fn map_to_domain_poll(poll: &PollOp) -> Option<Poll> {
    let resource_id = poll.resource_id.map(|id| id.id);

    let resource_name = Some("Unknown".into());

    let name = poll.name.clone();

    let task_id = poll.task_id.map(|id| id.id);

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
