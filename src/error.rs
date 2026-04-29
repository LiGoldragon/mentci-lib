//! Typed Error enum per crate convention.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("not connected to criome")]
    CriomeDisconnected,

    #[error("not connected to nexus-daemon")]
    NexusDisconnected,

    #[error("schema knowledge missing for kind: {kind}")]
    UnknownKind { kind: String },

    #[error("invalid event for current state: {reason}")]
    InvalidEvent { reason: String },

    #[error("constructor flow rejected by validation: {reason}")]
    FlowRejected { reason: String },

    #[error("subscription not registered: {sub_id}")]
    UnknownSubscription { sub_id: u64 },

    #[error("internal invariant violated: {reason}")]
    InternalInvariant { reason: String },
}

pub type Result<T> = core::result::Result<T, Error>;
