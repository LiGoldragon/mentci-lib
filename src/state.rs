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

use crate::canvas::CanvasState;
use crate::cmd::Cmd;
use crate::connection::ConnectionState;
use crate::constructor::ActiveConstructor;
use crate::diagnostics::DiagnosticsState;
use crate::event::{EngineEvent, UserEvent};
use crate::inspector::InspectorState;
use crate::layout::LayoutState;
use crate::theme::ThemeState;
use crate::view::WorkbenchView;
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
pub struct ModelCache {
    // todo!() — shape lands once subscriptions are wired
}

impl WorkbenchState {
    /// Construct a fresh state with no daemon connections yet.
    /// The runtime opens connections after construction by
    /// dispatching [`Cmd::ConnectCriome`] and
    /// [`Cmd::ConnectNexus`].
    pub fn new(_principal: signal::Slot) -> Self {
        todo!()
    }

    /// Derive the per-frame snapshot. Pure; takes `&self`.
    pub fn view(&self) -> WorkbenchView {
        todo!()
    }

    /// Apply a user-originated gesture. Returns the side-effect
    /// commands the runtime should dispatch.
    pub fn on_user_event(&mut self, _ev: UserEvent) -> Vec<Cmd> {
        todo!()
    }

    /// Apply an engine-originated event (push, outcome,
    /// diagnostic, render reply, connection state change).
    /// Returns the side-effect commands the runtime should
    /// dispatch.
    pub fn on_engine_event(&mut self, _ev: EngineEvent) -> Vec<Cmd> {
        todo!()
    }
}
