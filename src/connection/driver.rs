//! Connection driver — tokio task that owns one daemon's UDS
//! socket and bridges between the wire and mentci-lib's event
//! model via mpsc channels.
//!
//! One driver per daemon. Each shell holds two:
//! [`DaemonRole::Criome`] and [`DaemonRole::Nexus`]. The
//! driver task runs on a tokio runtime owned by the shell;
//! mentci-lib provides the dialing + state-machine logic, the
//! shell provides the runtime.
//!
//! Lifecycle of a driver:
//!
//! 1. Spawn — [`spawn_driver`] returns a [`ConnectionHandle`]
//!    containing a `events_rx` (engine events flowing OUT to
//!    the shell) and a `cmds_tx` (driver commands flowing IN
//!    from the shell).
//! 2. Connect — the task tries to open the UDS. On success it
//!    emits [`crate::EngineEvent::CriomeConnected`] /
//!    [`crate::EngineEvent::NexusConnected`]. On failure it
//!    emits the corresponding `*Disconnected` event with the
//!    error text and the loop exits.
//! 3. Run — the task loops, awaiting either an inbound frame
//!    on the socket or a [`DriverCmd`] from the shell. Frames
//!    become engine events; commands become writes.
//! 4. Disconnect — when the shell drops the
//!    [`ConnectionHandle`], `cmds_tx` closes and the task
//!    exits, emitting a final `*Disconnected` event before
//!    returning.
//!
//! This module owns dialing + the loop shape. The handshake
//! body and frame I/O are skeleton-as-design today; real
//! wire work lands as criome and nexus-daemon's signal
//! servers are exercised end-to-end.

use std::path::PathBuf;
use tokio::net::UnixStream;
use tokio::runtime::Handle;
use tokio::sync::mpsc;

use crate::EngineEvent;

/// Which daemon this driver targets. Determines which event
/// variants the driver emits and which socket it dials.
#[derive(Copy, Clone, Debug)]
pub enum DaemonRole {
    Criome,
    Nexus,
}

/// What the shell sends *into* a driver.
#[derive(Debug)]
pub enum DriverCmd {
    /// Send a typed signal frame on the wire.
    SendFrame(signal::Frame),
    /// Cleanly close the connection (driver loop exits).
    Disconnect,
}

/// Handle the shell holds. Drains events; sends commands.
pub struct ConnectionHandle {
    pub events_rx: mpsc::UnboundedReceiver<EngineEvent>,
    pub cmds_tx: mpsc::UnboundedSender<DriverCmd>,
}

/// Spawn a driver task on the supplied tokio runtime. Returns
/// immediately with a handle. The driver's first action is to
/// dial the socket.
pub fn spawn_driver(
    runtime: &Handle,
    socket_path: PathBuf,
    role: DaemonRole,
) -> ConnectionHandle {
    let (events_tx, events_rx) = mpsc::unbounded_channel();
    let (cmds_tx, cmds_rx) = mpsc::unbounded_channel();
    runtime.spawn(driver_loop(socket_path, role, events_tx, cmds_rx));
    ConnectionHandle { events_rx, cmds_tx }
}

async fn driver_loop(
    socket_path: PathBuf,
    role: DaemonRole,
    events_tx: mpsc::UnboundedSender<EngineEvent>,
    mut cmds_rx: mpsc::UnboundedReceiver<DriverCmd>,
) {
    // ── Dial ───────────────────────────────────────────────
    let _stream = match UnixStream::connect(&socket_path).await {
        Ok(s) => s,
        Err(e) => {
            let _ = events_tx.send(disconnect_event(
                role,
                format!("connect to {} failed: {e}", socket_path.display()),
            ));
            return;
        }
    };

    // ── Handshake ──────────────────────────────────────────
    // Real handshake (HandshakeRequest / HandshakeReply with
    // ProtocolVersion negotiation) lands as the next iteration.
    // For now: declare a placeholder version so the header
    // chip transitions disconnected → connected and the user
    // sees the dial-and-handshake lifecycle on screen.
    let _ = events_tx.send(connect_event(role, "0.1.0".into()));

    // ── Run ────────────────────────────────────────────────
    // Real frame I/O via [`signal::Frame::encode`] /
    // [`signal::Frame::decode`] over the UDS lands next. For
    // now: only the command channel is consumed; inbound
    // frames are not yet read.
    let mut disconnect_reason: Option<String> = None;
    while let Some(cmd) = cmds_rx.recv().await {
        match cmd {
            DriverCmd::Disconnect => {
                disconnect_reason = Some("disconnect requested".into());
                break;
            }
            DriverCmd::SendFrame(_frame) => {
                // todo!() — real `signal::Frame` write loop lands
                //          alongside criome's signal server.
            }
        }
    }

    // ── Disconnect ─────────────────────────────────────────
    let reason = disconnect_reason
        .unwrap_or_else(|| "shell dropped connection handle".into());
    let _ = events_tx.send(disconnect_event(role, reason));
}

fn connect_event(role: DaemonRole, protocol_version: String) -> EngineEvent {
    match role {
        DaemonRole::Criome => EngineEvent::CriomeConnected { protocol_version },
        DaemonRole::Nexus => EngineEvent::NexusConnected { protocol_version },
    }
}

fn disconnect_event(role: DaemonRole, reason: String) -> EngineEvent {
    match role {
        DaemonRole::Criome => EngineEvent::CriomeDisconnected { reason },
        DaemonRole::Nexus => EngineEvent::NexusDisconnected { reason },
    }
}
