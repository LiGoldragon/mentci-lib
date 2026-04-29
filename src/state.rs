//! [`WorkbenchState`] — the model owned by mentci-lib.
//!
//! Every shell wraps an instance of this. The shell never reads
//! fields directly; it calls [`WorkbenchState::view`] to get a
//! [`WorkbenchView`] snapshot, paints it, and forwards
//! [`UserEvent`]s back via [`WorkbenchState::on_user_event`].
//!
//! Engine pushes from criome and reply arrivals from
//! nexus-daemon are surfaced as [`EngineEvent`]s through
//! [`WorkbenchState::on_engine_event`].
//!
//! Both event entrypoints return `Vec<Cmd>` — side-effects the
//! outer runtime dispatches (send a signal frame, ask
//! nexus-daemon to render).

use crate::canvas::{CanvasState, CanvasView, KindCanvasState};
use crate::cmd::Cmd;
use crate::connection::{ConnectionState, ConnectionView, DaemonStatus};
use crate::constructor::ActiveConstructor;
use crate::diagnostics::DiagnosticsState;
use crate::event::{EngineEvent, UserEvent};
use crate::inspector::{InspectorState, InspectorView};
use crate::layout::LayoutState;
use crate::theme::ThemeState;
use crate::view::{GraphsNavView, HeaderView, WorkbenchView};
use crate::wire::WireState;

/// The library's owned model. One per mentci session.
pub struct WorkbenchState {
    /// Two-daemon connection state (criome + nexus-daemon).
    pub connections: ConnectionState,
    /// Current Principal slot (whose tweaks apply).
    pub principal: signal::Slot,
    /// Theme intent derived from the active Theme record.
    pub theme: ThemeState,
    /// Layout intent derived from the active Layout record.
    pub layout: LayoutState,
    /// Per-pane state.
    pub canvas: CanvasState,
    pub inspector: InspectorState,
    pub diagnostics: DiagnosticsState,
    pub wire: WireState,
    /// Constructor flow currently active (drag-wire, rename,
    /// retract confirm, batch edit). At most one at a time.
    pub active_constructor: Option<ActiveConstructor>,
    /// Cached records the shell may need (recent
    /// subscription pushes, current Graph + Nodes + Edges in
    /// the canvas, etc.).
    pub cache: ModelCache,
}

/// Local cache of records the workbench is currently showing.
/// Concrete shape grows as subscriptions wire up.
#[derive(Default)]
pub struct ModelCache {}

impl WorkbenchState {
    /// Construct a fresh state with no daemon connections yet.
    /// The runtime opens connections after construction by
    /// dispatching [`Cmd::ConnectCriome`] and
    /// [`Cmd::ConnectNexus`].
    pub fn new(principal: signal::Slot) -> Self {
        Self {
            connections: ConnectionState::new(),
            principal,
            theme: ThemeState::builtin_default(),
            layout: LayoutState::builtin_default(),
            canvas: CanvasState::default(),
            inspector: InspectorState::default(),
            diagnostics: DiagnosticsState::default(),
            wire: WireState::default(),
            active_constructor: None,
            cache: ModelCache::default(),
        }
    }

    /// Derive the per-frame snapshot. Pure; takes `&self`.
    pub fn view(&self) -> WorkbenchView {
        WorkbenchView {
            header: HeaderView {
                criome: ConnectionView {
                    label: "criome".to_string(),
                    status: self.connections.criome.status.clone(),
                    version: self.connections.criome.protocol_version.clone(),
                    note: self.connections.criome.last_disconnect_reason.clone(),
                },
                nexus: ConnectionView {
                    label: "nexus".to_string(),
                    status: self.connections.nexus.status.clone(),
                    version: self.connections.nexus.protocol_version.clone(),
                    note: self.connections.nexus.last_disconnect_reason.clone(),
                },
                wire_toggled_on: self.layout.intents.wire_pane_visible,
                tweaks_open: self.layout.intents.tweaks_pane_open,
            },
            graphs_nav: GraphsNavView {
                graphs: Vec::new(),
                selected_slot: None,
            },
            canvas: match &self.canvas.kind_state {
                KindCanvasState::Empty => CanvasView::Empty,
                KindCanvasState::FlowGraph(_) => {
                    // Real rendering lands when records arrive
                    // via subscription.
                    CanvasView::Empty
                }
            },
            inspector: InspectorView {
                focused: None,
                pinned: Vec::new(),
            },
            diagnostics: None,
            wire: None,
            constructor: None,
        }
    }

    /// Apply a user-originated gesture. Returns the side-effect
    /// commands the runtime should dispatch.
    pub fn on_user_event(&mut self, ev: UserEvent) -> Vec<Cmd> {
        match ev {
            UserEvent::ToggleWirePane => {
                self.layout.intents.wire_pane_visible =
                    !self.layout.intents.wire_pane_visible;
                Vec::new()
            }
            UserEvent::ToggleTweaksPane => {
                self.layout.intents.tweaks_pane_open =
                    !self.layout.intents.tweaks_pane_open;
                Vec::new()
            }
            UserEvent::ReconnectCriome => vec![Cmd::ConnectCriome],
            UserEvent::ReconnectNexus => vec![Cmd::ConnectNexus],
            // Every other event is unhandled in this skeleton
            // pass; bodies fill in as the wire wires up.
            _ => Vec::new(),
        }
    }

    /// Apply an engine-originated event (push, outcome,
    /// diagnostic, render reply, connection state change).
    /// Returns the side-effect commands the runtime should
    /// dispatch.
    pub fn on_engine_event(&mut self, ev: EngineEvent) -> Vec<Cmd> {
        match ev {
            EngineEvent::CriomeConnected { protocol_version } => {
                self.connections.criome.status = DaemonStatus::Connected;
                self.connections.criome.protocol_version = Some(protocol_version);
                self.connections.criome.last_disconnect_reason = None;
                Vec::new()
            }
            EngineEvent::CriomeDisconnected { reason } => {
                self.connections.criome.status = DaemonStatus::Disconnected;
                self.connections.criome.protocol_version = None;
                self.connections.criome.last_disconnect_reason = Some(reason);
                Vec::new()
            }
            EngineEvent::NexusConnected { protocol_version } => {
                self.connections.nexus.status = DaemonStatus::Connected;
                self.connections.nexus.protocol_version = Some(protocol_version);
                self.connections.nexus.last_disconnect_reason = None;
                Vec::new()
            }
            EngineEvent::NexusDisconnected { reason } => {
                self.connections.nexus.status = DaemonStatus::Disconnected;
                self.connections.nexus.protocol_version = None;
                self.connections.nexus.last_disconnect_reason = Some(reason);
                Vec::new()
            }
            // All other engine events unhandled in this
            // skeleton pass; bodies fill in as the wire wires
            // up.
            _ => Vec::new(),
        }
    }
}
