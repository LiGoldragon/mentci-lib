//! Dual-daemon connection management.
//!
//! mentci-lib holds two persistent connections — to criome
//! (signal: editing, queries, subscriptions) and to
//! nexus-daemon (signal: signal↔nexus rendering only). The
//! shell sees a unified "engine" surface; the dual-daemon
//! split is hidden from widget code and revealed only in the
//! header view.

/// Live state of both daemon connections.
pub struct ConnectionState {
    pub criome: PerDaemonState,
    pub nexus: PerDaemonState,
}

/// One daemon's state. Used twice in [`ConnectionState`].
pub struct PerDaemonState {
    pub status: DaemonStatus,
    pub protocol_version: Option<String>,
    /// Reason the connection ended, if applicable. Surfaced
    /// to the user — auto-reconnect is rejected.
    pub last_disconnect_reason: Option<String>,
}

/// Lifecycle stages for one daemon. Surfaced to the user in
/// the header.
#[derive(Debug, Clone)]
pub enum DaemonStatus {
    Disconnected,
    Connecting,
    Handshaking,
    Connected,
}

/// View of one daemon's connection. Pure data; the shell
/// reads this from [`crate::view::HeaderView`].
#[derive(Debug, Clone)]
pub struct ConnectionView {
    pub label: String,
    pub status: DaemonStatus,
    pub version: Option<String>,
    pub note: Option<String>,
}

impl ConnectionState {
    pub fn new() -> Self {
        Self {
            criome: PerDaemonState::disconnected(),
            nexus: PerDaemonState::disconnected(),
        }
    }
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::new()
    }
}

impl PerDaemonState {
    pub fn disconnected() -> Self {
        Self {
            status: DaemonStatus::Disconnected,
            protocol_version: None,
            last_disconnect_reason: None,
        }
    }
}
