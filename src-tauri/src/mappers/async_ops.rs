//! Convert from `console_api::async_ops::AsyncOp` to our domain `AsyncOp`.

use crate::domain::async_op::AsyncOp;
use console_api::async_ops;

/// Maps a protobuf‐style `console_api` AsyncOp into the domain `AsyncOp`.
///
/// Returns `None` if either `id` or `resource_id` is missing.
pub fn map_to_domain_async_op(async_op: &async_ops::AsyncOp) -> Option<AsyncOp> {
    match (async_op.id, async_op.resource_id) {
        (Some(id), Some(resource_id)) => Some(AsyncOp {
            id: id.id,
            resource_id: resource_id.id,
            resource_target: None,
        }),
        _ => None,
    }
}
