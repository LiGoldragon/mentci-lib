//! [`Cmd`] — side-effect descriptions returned from
//! `update`. The outer runtime executes these.
//!
//! Cmds are *intent*; the runtime is the executor. Keeping
//! side-effects out of the model preserves the MVU
//! property — `update` is a pure function of `(state, event)`.

use signal::Frame;

/// One side-effect to dispatch.
#[derive(Debug, Clone)]
pub enum Cmd {
    /// Send a signal frame to criome.
    SendCriome { frame: Frame },
    /// Send a signal frame to nexus-daemon. Used for
    /// rendering requests (signal-payload → nexus-text) and
    /// the very rare parse-text-from-input case.
    SendNexus { frame: Frame },

    /// Open the criome connection.
    ConnectCriome,
    /// Open the nexus-daemon connection.
    ConnectNexus,
    /// Close + drop the criome connection.
    DisconnectCriome,
    /// Close + drop the nexus-daemon connection.
    DisconnectNexus,

    /// Register a subscription on criome.
    Subscribe { query: signal::Request },
    /// Unsubscribe from criome.
    Unsubscribe { sub_id: u64 },

    /// Ask nexus-daemon to render a typed payload as nexus
    /// text. Reply arrives as [`crate::event::EngineEvent::NexusRendered`].
    RenderViaNexus { ticket: u64, payload: NexusRenderRequest },

    /// Schedule a timer that fires once after `ms` milliseconds.
    /// Used sparingly — the workbench is push-driven, not
    /// poll-driven.
    SetTimer { ms: u64, tag: TimerTag },
}

/// What kind of payload to render via nexus-daemon.
/// Concrete shape lands when the nexus-daemon's render-only
/// API is wired.
#[derive(Debug, Clone)]
pub enum NexusRenderRequest {
    /// Render a complete record.
    Record { kind: String, content_hash: signal::Hash },
    /// Render a typed verb body.
    Verb {
        verb_name: String,
        // typed payload bytes; rkyv-encoded
        bytes: Vec<u8>,
    },
}

/// Tag identifying which timer fired. Each scheduled timer
/// matches against this on its callback.
#[derive(Debug, Clone)]
pub enum TimerTag {
    Reconnect,
    Custom { name: String },
}
