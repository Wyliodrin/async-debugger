//! Convert from `console_api::async_ops::AsyncOp` to our domain `AsyncOp`.

use crate::{backend::domain::async_op::AsyncOp, utils::error::Error as TraceError};
use console_api::async_ops;

/// Maps a protobuf‐style `console_api` AsyncOp into the domain `AsyncOp`.
///
/// Returns `TraceError` if either `id` or `resource_id` is missing.
pub fn map_to_domain_async_op(async_op: &async_ops::AsyncOp) -> Result<AsyncOp, TraceError> {
    match (async_op.id, async_op.resource_id) {
        (Some(id), Some(resource_id)) => Ok(AsyncOp {
            id: id.id,
            resource_id: resource_id.id,
            resource_target: None,
        }),
        (None, Some(_resource_id)) => Err(TraceError::IDNotFound),
        (Some(_id), None) => Err(TraceError::ResourceIDNotFound),
        (None, None) => Err(TraceError::IDAndResourceIDNotFound),
    }
}
