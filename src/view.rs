//! [`WorkbenchView`] — the per-frame snapshot the shell paints.
//!
//! Pure data, derived from [`WorkbenchState`] by the
//! [`crate::state::WorkbenchState::view`] method. The shell
//! reads this and paints; it does not reach into the state.

use crate::canvas::CanvasView;
use crate::connection::ConnectionView;
use crate::constructor::ConstructorView;
use crate::diagnostics::DiagnosticsView;
use crate::inspector::InspectorView;
use crate::wire::WireView;

/// What the shell paints this frame. Optional panes are
/// `None` when they should not be visible.
pub struct WorkbenchView {
    /// Top header — daemon connection states, toggle buttons.
    pub header: HeaderView,
    /// Left navigation pane — list of Graphs, Tweaks, etc.
    pub graphs_nav: GraphsNavView,
    /// Centre canvas — kind-driven; renderer chosen by
    /// selection.
    pub canvas: CanvasView,
    /// Right inspector — selected slot detail + history.
    pub inspector: InspectorView,
    /// Diagnostics strip — present only when ≥1 unread.
    pub diagnostics: Option<DiagnosticsView>,
    /// Wire pane — present only when user-toggled on.
    pub wire: Option<WireView>,
    /// Active constructor flow (drag-wire modal, rename
    /// in-place, retract confirm, batch composer). At most
    /// one at a time.
    pub constructor: Option<ConstructorView>,
}

/// Header showing both daemon connections and global toggles.
pub struct HeaderView {
    pub criome: ConnectionView,
    pub nexus: ConnectionView,
    pub wire_toggled_on: bool,
    pub tweaks_open: bool,
}

/// Left navigation. Lists Graphs the user has access to,
/// recently visited slots, and a pinned set.
pub struct GraphsNavView {
    pub graphs: Vec<GraphsNavEntry>,
    pub selected_slot: Option<signal::Slot>,
}

/// One entry in the GraphsNav.
pub struct GraphsNavEntry {
    pub slot: signal::Slot,
    pub display_name: String,
    pub kind: GraphsNavKind,
}

pub enum GraphsNavKind {
    Graph,
    Tweaks,
    PinnedSlot,
    RecentEdit,
}
