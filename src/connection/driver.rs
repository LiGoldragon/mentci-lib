//! Connection driver — tokio task that owns one daemon's UDS
//! socket and bridges between the wire and mentci-lib's event
//! model via mpsc channels.
//!
//! One driver per daemon. Each shell holds two:
//! [`DaemonRole::Criome`] and [`DaemonRole::Nexus`]. The
//! driver task runs on a tokio runtime owned by the shell;
//! mentci-lib provides the dialing + handshake + frame-I/O
//! state machine, the shell provides the runtime.
//!
//! Lifecycle of a driver:
//!
//! 1. Spawn — [`spawn_driver`] returns a [`ConnectionHandle`]
//!    containing a `events_rx` (engine events flowing OUT to
//!    the shell) and a `cmds_tx` (driver commands flowing IN
//!    from the shell).
//! 2. Connect — the task tries to open the UDS. On failure it
//!    emits the appropriate `*Disconnected` event with the
//!    error text and the loop exits.
//! 3. Handshake — sends a `Request::Handshake` frame, awaits
//!    `Reply::HandshakeAccepted` (or `HandshakeRejected`). On
//!    accept, emits the corresponding `*Connected` event with
//!    the server's protocol version. On reject or any error,
//!    emits a `*Disconnected` event.
//! 4. Run — the task `tokio::select!`s between reading the
//!    next inbound frame and receiving a [`DriverCmd`] from
//!    the shell. Inbound frames become engine events; outbound
//!    cmds become writes.
//! 5. Disconnect — when the shell drops the
//!    [`ConnectionHandle`] (or the socket errors), the task
//!    exits, emitting a final `*Disconnected` event with the
//!    reason.

use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UnixStream;
use tokio::runtime::Handle;
use tokio::sync::mpsc;

use signal::{
    Body, Frame, HandshakeRequest, OutcomeMessage, Records, Reply, Request,
    SIGNAL_PROTOCOL_VERSION,
};

use crate::event::FrameDirection;
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
    SendFrame(Frame),
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
/// dial the socket, then exchange a Handshake.
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
    let stream = match UnixStream::connect(&socket_path).await {
        Ok(s) => s,
        Err(e) => {
            let _ = events_tx.send(disconnect_event(
                role,
                format!("connect to {} failed: {e}", socket_path.display()),
            ));
            return;
        }
    };
    let (mut read_half, mut write_half) = stream.into_split();

    // ── Handshake ──────────────────────────────────────────
    let handshake_frame = Frame {
        principal_hint: None,
        auth_proof: None,
        body: Body::Request(Request::Handshake(HandshakeRequest {
            client_version: SIGNAL_PROTOCOL_VERSION,
            client_name: client_name_for(role),
        })),
    };
    if let Err(e) = write_frame(&mut write_half, &handshake_frame).await {
        let _ = events_tx.send(disconnect_event(role, format!("send handshake: {e}")));
        return;
    }
    let _ = events_tx.send(EngineEvent::FrameSeen {
        direction: FrameDirection::Out,
        frame: handshake_frame,
    });

    let reply = match read_frame(&mut read_half).await {
        Ok(f) => f,
        Err(e) => {
            let _ = events_tx.send(disconnect_event(
                role,
                format!("read handshake reply: {e}"),
            ));
            return;
        }
    };
    let _ = events_tx.send(EngineEvent::FrameSeen {
        direction: FrameDirection::In,
        frame: reply.clone(),
    });

    let server_version_str = match reply.body {
        Body::Reply(Reply::HandshakeAccepted(h)) => format!(
            "{}.{}.{}",
            h.server_version.major, h.server_version.minor, h.server_version.patch
        ),
        Body::Reply(Reply::HandshakeRejected(reason)) => {
            let _ = events_tx.send(disconnect_event(
                role,
                format!("handshake rejected: {reason:?}"),
            ));
            return;
        }
        other => {
            let _ = events_tx.send(disconnect_event(
                role,
                format!("expected handshake reply; got {other:?}"),
            ));
            return;
        }
    };
    let _ = events_tx.send(connect_event(role, server_version_str));

    // ── Run ────────────────────────────────────────────────
    // Sequential req-id; first post-handshake frame is req#1.
    // Replies pair to requests by FIFO position on the wire.
    let mut next_req_id: u64 = 1;
    let mut next_reply_id: u64 = 1;
    let mut disconnect_reason: Option<String> = None;

    loop {
        tokio::select! {
            read_result = read_frame(&mut read_half) => {
                match read_result {
                    Ok(frame) => {
                        let _ = events_tx.send(EngineEvent::FrameSeen {
                            direction: FrameDirection::In,
                            frame: frame.clone(),
                        });
                        emit_inbound_typed(&events_tx, frame, &mut next_reply_id);
                    }
                    Err(e) => {
                        disconnect_reason = Some(format!("read: {e}"));
                        break;
                    }
                }
            }
            cmd_opt = cmds_rx.recv() => {
                match cmd_opt {
                    None => {
                        disconnect_reason = Some("shell dropped connection handle".into());
                        break;
                    }
                    Some(DriverCmd::Disconnect) => {
                        disconnect_reason = Some("disconnect requested".into());
                        break;
                    }
                    Some(DriverCmd::SendFrame(frame)) => {
                        let _ = events_tx.send(EngineEvent::FrameSeen {
                            direction: FrameDirection::Out,
                            frame: frame.clone(),
                        });
                        if let Err(e) = write_frame(&mut write_half, &frame).await {
                            disconnect_reason = Some(format!("write: {e}"));
                            break;
                        }
                        next_req_id += 1;
                    }
                }
            }
        }
    }

    // ── Disconnect ─────────────────────────────────────────
    let reason = disconnect_reason.unwrap_or_else(|| "loop ended".into());
    let _ = events_tx.send(disconnect_event(role, reason));
    let _ = next_req_id; // keep linter quiet — req-id pairing
                        // matures alongside subscription work.
}

/// Translate an inbound reply frame into the appropriate
/// typed engine event(s). FrameSeen has already been emitted
/// by the caller; this adds the higher-level events the
/// model wants.
fn emit_inbound_typed(
    events_tx: &mpsc::UnboundedSender<EngineEvent>,
    frame: Frame,
    next_reply_id: &mut u64,
) {
    let body = match frame.body {
        Body::Reply(r) => r,
        Body::Request(_) => {
            // Server sending a Request to the client is not
            // part of the M0 protocol shape. Surface as a
            // FrameSeen-only event (already emitted) and move
            // on.
            return;
        }
    };

    match body {
        Reply::HandshakeAccepted(_) | Reply::HandshakeRejected(_) => {
            // Spurious handshake reply post-handshake; ignore.
        }
        Reply::Outcome(outcome) => {
            let req_id = *next_reply_id;
            *next_reply_id += 1;
            let _ = events_tx.send(EngineEvent::OutcomeArrived {
                req_id,
                outcome,
            });
        }
        Reply::Outcomes(outcomes) => {
            let req_id = *next_reply_id;
            *next_reply_id += 1;
            for outcome in outcomes {
                let _ = events_tx.send(EngineEvent::OutcomeArrived {
                    req_id,
                    outcome: outcome_to_message(outcome),
                });
            }
        }
        Reply::Records(records) => {
            let req_id = *next_reply_id;
            *next_reply_id += 1;
            // For now every Records reply is QueryReplied; once
            // subscriptions are wired the driver will track
            // sub-ids and emit SubscriptionPush instead when
            // the position belongs to a subscription rather
            // than a one-shot query.
            let _ = events_tx.send(EngineEvent::QueryReplied {
                req_id,
                records,
            });
        }
    }
}

/// Pass-through wrapper. Kept as a function so the
/// `Outcome(...)` vs `Outcomes(Vec<...>)` shape difference is
/// visible at the call site.
fn outcome_to_message(o: OutcomeMessage) -> OutcomeMessage {
    o
}

async fn read_frame(read: &mut OwnedReadHalf) -> std::io::Result<Frame> {
    let mut length_bytes = [0u8; 4];
    read.read_exact(&mut length_bytes).await?;
    let length = u32::from_be_bytes(length_bytes) as usize;
    let mut frame_bytes = vec![0u8; length];
    read.read_exact(&mut frame_bytes).await?;
    Frame::decode(&frame_bytes).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{e:?}"))
    })
}

async fn write_frame(write: &mut OwnedWriteHalf, frame: &Frame) -> std::io::Result<()> {
    let bytes = frame.encode();
    let length = u32::try_from(bytes.len())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "frame too large"))?;
    write.write_all(&length.to_be_bytes()).await?;
    write.write_all(&bytes).await?;
    Ok(())
}

fn client_name_for(role: DaemonRole) -> String {
    match role {
        DaemonRole::Criome => "mentci-egui".to_string(),
        DaemonRole::Nexus => "mentci-egui".to_string(),
    }
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
