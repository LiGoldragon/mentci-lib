//! [`Cmd`] — the side-effects the outer runtime dispatches after an event is
//! folded into the model. Keeping side-effects out of the model is the MVU
//! property: `update` is a pure function of `(state, event) -> (state,
//! Vec<Cmd>)`.
//!
//! Every command carries a live `signal-mentci` request (`MentciRequest`)
//! addressed to a component socket; the runtime owns the transport that turns
//! it into a `signal-frame` `MentciFrame`. The model never speaks the wire
//! itself.

use meta_signal_mentci::ComponentSocketKind;
use signal_mentci::MentciRequest;

/// One side-effect to dispatch. The runtime executes these; the model only
/// describes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    /// Send a `signal-mentci` request to a component socket. The runtime
    /// frames it (`MentciFrame::new(MentciFrameBody::Request { .. })`), dials
    /// the socket from the configured [`ComponentSocketKind`] path, and raises
    /// the reply as a [`crate::event::EngineEvent`].
    SendRequest {
        socket: ComponentSocketKind,
        request: MentciRequest,
    },
}

impl Cmd {
    /// Build a [`Cmd::SendRequest`] addressed to a component socket.
    pub fn send(socket: ComponentSocketKind, request: MentciRequest) -> Self {
        Self::SendRequest { socket, request }
    }
}
