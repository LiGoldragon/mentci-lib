//! Typed `Error` for the client mentci model. One enum per crate
//! (`skills/rust/errors.md`).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    /// A reply arrived for a component socket the model never opened an
    /// observation on, so it has no projection slot to fold it into.
    #[error("no observation open for component socket {socket}")]
    UnobservedSocket { socket: ComponentSocketLabel },

    /// A verdict or proposal named a question that is not in the pending
    /// queue the model currently holds.
    #[error("question not pending: {question}")]
    UnknownQuestion { question: String },

    /// A retract referenced a subscription token the model has no record of.
    #[error("subscription token not held: {token}")]
    UnknownSubscription { token: String },

    /// A daemon-minted token was expected on this reply but the daemon
    /// returned a shape that carried none.
    #[error("daemon reply carried no subscription token where one was required")]
    MissingSubscriptionToken,

    /// The model was asked to fold a reply variant into a slice of state it
    /// does not own.
    #[error("model invariant violated: {reason}")]
    InvariantViolated { reason: String },

    /// The introspect-query socket connect / write / read failed. This is the
    /// universal-client seed reaching a second component (introspect); the
    /// transport carries the typed I/O detail rather than a String catch-all.
    #[error("introspect socket I/O failed: {detail}")]
    IntrospectSocket { detail: String },

    /// The introspect daemon answered with a frame shape that is not an
    /// accepted single-operation reply — a rejection, a streaming frame, or an
    /// unexpected reply variant. Named so the caller can distinguish a wire
    /// fault from a transport fault.
    #[error("introspect daemon returned an unexpected frame: {detail}")]
    UnexpectedIntrospectFrame { detail: String },
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Self {
        Self::IntrospectSocket {
            detail: source.to_string(),
        }
    }
}

impl From<signal_frame::FrameError> for Error {
    fn from(source: signal_frame::FrameError) -> Self {
        Self::UnexpectedIntrospectFrame {
            detail: source.to_string(),
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;

/// A human-readable component-socket label, used in error messages so the
/// `Error` enum carries no GUI types and stays `Display`-clean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentSocketLabel(String);

impl ComponentSocketLabel {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl core::fmt::Display for ComponentSocketLabel {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<meta_signal_mentci::ComponentSocketKind> for ComponentSocketLabel {
    fn from(kind: meta_signal_mentci::ComponentSocketKind) -> Self {
        use meta_signal_mentci::ComponentSocketKind;
        Self::new(match kind {
            ComponentSocketKind::Mentci => "Mentci",
            ComponentSocketKind::MetaMentci => "MetaMentci",
            ComponentSocketKind::Criome => "Criome",
            ComponentSocketKind::MetaCriome => "MetaCriome",
            ComponentSocketKind::Introspect => "Introspect",
            ComponentSocketKind::MetaIntrospect => "MetaIntrospect",
        })
    }
}
