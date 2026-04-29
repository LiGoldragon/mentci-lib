//! Flow-graph canvas renderer.
//!
//! Renders a `Graph` record + its `Contains`-edge member
//! `Node`s + `Flow`/`DependsOn`/etc. `Edge`s as the typed
//! flow-graph view (boxes-and-edges).
//!
//! The first canvas renderer to ship. Other kinds (astro
//! chart, timelines, maps) follow the same pattern in their
//! own submodules.

use signal::{Edge, Graph, Node, RelationKind, Slot};

/// Per-flow-graph canvas state. Held inside
/// [`super::KindCanvasState::FlowGraph`].
pub struct FlowGraphCanvasState {
    pub graph: Slot,
    /// Pending preview during a drag-wire flow (dashed wire
    /// before commit).
    pub pending_wire: Option<PendingWire>,
}

/// In-flight drag-wire preview — visualised but not
/// committed. Goes away when the constructor flow commits or
/// cancels.
pub struct PendingWire {
    pub from: Slot,
    pub to_slot_or_pos: PendingWireTarget,
}

pub enum PendingWireTarget {
    /// Mouse-following — no target node yet.
    FreeFloating { x: f32, y: f32 },
    /// Hovered onto a target node — kind picker about to open.
    Hovered { onto: Slot },
}

/// What the shell paints. Pure data.
pub struct FlowGraphView {
    pub graph: Graph,
    pub nodes: Vec<RenderedNode>,
    pub edges: Vec<RenderedEdge>,
    pub pending_wire: Option<RenderedPending>,
}

/// One node, ready to paint. Position + state colour + kind
/// glyph + display name. The shell maps `kind_glyph` and
/// `state_intent` to its native visual idiom.
pub struct RenderedNode {
    pub slot: Slot,
    pub at: (f32, f32),
    pub kind_glyph: KindGlyph,
    pub state_intent: NodeStateIntent,
    pub display_name: String,
}

/// One edge, ready to paint.
pub struct RenderedEdge {
    pub slot: Slot,
    pub from: Slot,
    pub to: Slot,
    pub relation_intent: RelationKind,
    pub state_intent: EdgeStateIntent,
}

/// Pending preview wire (dashed).
pub struct RenderedPending {
    pub from: (f32, f32),
    pub to: (f32, f32),
}

/// Glyph for a node-kind. Concrete glyph mapping lives in
/// [`crate::theme`]; here we carry the abstract intent.
#[derive(Debug, Clone, Copy)]
pub enum KindGlyph {
    Source,
    Transformer,
    Sink,
    Junction,
    Supervisor,
    /// Schema added a kind we don't have a glyph for yet.
    Unknown,
}

/// Per-node state colour intent.
#[derive(Debug, Clone, Copy)]
pub enum NodeStateIntent {
    /// Saved, current, no in-flight edits.
    Stable,
    /// User started editing; criome hasn't accepted yet.
    Pending,
    /// Subscription push expected but not arrived; current
    /// view may be stale.
    Stale,
    /// Last write was rejected; cleared on next successful
    /// write to this slot.
    Rejected,
}

/// Per-edge state colour intent. Mirrors node states.
#[derive(Debug, Clone, Copy)]
pub enum EdgeStateIntent {
    Stable,
    Pending,
    Stale,
    Rejected,
}

pub struct FlowGraphRenderer;

impl super::CanvasRenderer for FlowGraphRenderer {
    type State = FlowGraphCanvasState;
    type View = FlowGraphView;

    fn render(_state: &Self::State) -> Self::View {
        todo!()
    }
}
