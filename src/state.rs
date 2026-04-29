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

use signal::{
    AuthProof, Body, Edge, EdgeQuery, Frame, Graph, GraphQuery, Node, NodeQuery,
    PatternField, QueryOperation, Records, Request, Slot,
};

use crate::canvas::{CanvasState, CanvasView, KindCanvasState};
use crate::cmd::Cmd;
use crate::connection::{ConnectionState, ConnectionView, DaemonStatus};
use crate::constructor::ActiveConstructor;
use crate::diagnostics::DiagnosticsState;
use crate::event::{EngineEvent, UserEvent};
use crate::inspector::{InspectorState, InspectorView};
use crate::layout::LayoutState;
use crate::theme::ThemeState;
use crate::view::{GraphsNavEntry, GraphsNavKind, GraphsNavView, HeaderView, WorkbenchView};
use crate::wire::WireState;

/// The library's owned model. One per mentci session.
pub struct WorkbenchState {
    /// Two-daemon connection state (criome + nexus-daemon).
    pub connections: ConnectionState,
    /// Current Principal slot (whose tweaks apply).
    pub principal: Slot,
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
    /// query/subscription results, current Graph + Nodes +
    /// Edges in the canvas, etc.).
    pub cache: ModelCache,
}

/// Local cache of records the workbench is currently showing.
///
/// Populated by `on_engine_event(QueryReplied { records, .. })`
/// and (eventually) `SubscriptionPush`. Read by `view()` when
/// composing pane views.
///
/// The wire shape today (`signal::Records::*`) carries typed
/// `Vec<Kind>` without per-record slot annotations. Once
/// records-with-slots is a real wire shape, this cache holds
/// `(Slot, Kind)` pairs so the navigation pane can identify
/// records by slot rather than by vec-position.
#[derive(Default)]
pub struct ModelCache {
    pub graphs: Vec<Graph>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl ModelCache {
    /// Replace the cache slice for whichever kind the records
    /// payload targets. (One QueryReplied = one variant; the
    /// reply maps 1:1 to a single kind's vector.)
    pub fn absorb(&mut self, records: Records) {
        match records {
            Records::Graph(g) => self.graphs = g,
            Records::Node(n) => self.nodes = n,
            Records::Edge(e) => self.edges = e,
        }
    }
}

impl WorkbenchState {
    /// Construct a fresh state with no daemon connections yet.
    /// The runtime opens connections after construction by
    /// dispatching [`Cmd::ConnectCriome`] and
    /// [`Cmd::ConnectNexus`].
    pub fn new(principal: Slot) -> Self {
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
            graphs_nav: self.graphs_nav_view(),
            canvas: self.canvas_view(),
            inspector: InspectorView {
                focused: None,
                pinned: Vec::new(),
            },
            diagnostics: None,
            wire: None,
            constructor: None,
        }
    }

    /// Build the canvas view per current kind-state.
    fn canvas_view(&self) -> CanvasView {
        match &self.canvas.kind_state {
            KindCanvasState::Empty => CanvasView::Empty,
            KindCanvasState::FlowGraph(_kstate) => {
                // The selected Graph's identity. The cache
                // entry is found by the synthetic vec-position
                // slot encoded into focus; this matches what
                // graphs_nav_view emits.
                let focus = match self.canvas.focus {
                    Some(s) => s,
                    None => return CanvasView::Empty,
                };
                let focus_idx: u64 = focus.into();
                let graph = match self.cache.graphs.get(focus_idx as usize) {
                    Some(g) => g.clone(),
                    None => return CanvasView::Empty,
                };
                let view = build_flow_graph_view(graph, &self.cache);
                CanvasView::FlowGraph(view)
            }
        }
    }

    /// Build the GraphsNav pane view from the cache. Until
    /// records-with-slots lands, the slot field is synthetic
    /// (vec-position cast to u64) — display is correct;
    /// selection-by-slot would round-trip wrong, so the shell
    /// must not assume the slot is durable.
    fn graphs_nav_view(&self) -> GraphsNavView {
        let graphs = self
            .cache
            .graphs
            .iter()
            .enumerate()
            .map(|(idx, g)| GraphsNavEntry {
                slot: Slot::from(idx as u64),
                display_name: g.title.clone(),
                kind: GraphsNavKind::Graph,
            })
            .collect();
        GraphsNavView {
            graphs,
            selected_slot: self.canvas.focus,
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
            UserEvent::SelectGraph { slot } => {
                // Focus the graph on the canvas. The next view
                // derivation builds a FlowGraphView from cache.
                self.canvas.focus = Some(slot);
                self.canvas.kind_state =
                    KindCanvasState::FlowGraph(crate::canvas::flow_graph::FlowGraphCanvasState {
                        graph: slot,
                        pending_wire: None,
                    });
                Vec::new()
            }
            UserEvent::SelectSlot { slot } => {
                // Generic selection — focuses the inspector.
                // Canvas focus stays on the current Graph.
                self.inspector.focused = Some(slot);
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
                // Auto-fetch the canonical kinds so the
                // workbench paints something on connect. Live
                // updates land when criome's Subscribe handler
                // ships (M2-side work); until then this is a
                // one-shot snapshot per connect — not a poll
                // loop. Push-not-pull discipline preserved.
                vec![
                    self.criome_query(QueryOperation::Graph(GraphQuery {
                        title: PatternField::Wildcard,
                    })),
                    self.criome_query(QueryOperation::Node(NodeQuery {
                        name: PatternField::Wildcard,
                    })),
                    self.criome_query(QueryOperation::Edge(EdgeQuery {
                        from: PatternField::Wildcard,
                        to: PatternField::Wildcard,
                        kind: PatternField::Wildcard,
                    })),
                ]
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
            EngineEvent::QueryReplied { req_id: _, records } => {
                self.cache.absorb(records);
                Vec::new()
            }
            // All other engine events unhandled in this
            // skeleton pass; bodies fill in as the wire wires
            // up.
            _ => Vec::new(),
        }
    }

    /// Helper: build a [`Cmd::SendCriome`] carrying a
    /// `Query` request frame. Auth is `SingleOperator` for
    /// the MVP; real BLS/quorum proofs land with the authz
    /// model.
    #[allow(dead_code)]
    fn criome_query(&self, op: QueryOperation) -> Cmd {
        let frame = Frame {
            principal_hint: Some(self.principal),
            auth_proof: Some(AuthProof::SingleOperator),
            body: Body::Request(Request::Query(op)),
        };
        Cmd::SendCriome { frame }
    }
}

/// Compose a [`crate::canvas::flow_graph::FlowGraphView`]
/// from one Graph and the workbench cache.
///
/// Until records-with-slots lands on the wire, edge endpoints
/// can't be resolved to specific Node records — `Edge.from`
/// and `Edge.to` are sema slots, while cached Nodes only carry
/// synthetic vec-position slots. For the first-paint cycle
/// the renderer paints every cached Node in a deterministic
/// horizontal row and lays out edges by hashing their
/// `from`/`to` slots into row positions. Visual placeholder;
/// the real layout uses NodePlacement records once they're
/// flowing through the wire.
fn build_flow_graph_view(
    graph: signal::Graph,
    cache: &ModelCache,
) -> crate::canvas::flow_graph::FlowGraphView {
    use crate::canvas::flow_graph::{
        EdgeStateIntent, FlowGraphView, KindGlyph, NodeStateIntent, RenderedEdge, RenderedNode,
    };

    let nodes_per_row = 8usize.max(1);
    let cell_w = 140.0_f32;
    let cell_h = 90.0_f32;
    let origin_x = 60.0_f32;
    let origin_y = 60.0_f32;

    let rendered_nodes: Vec<RenderedNode> = cache
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let col = (i % nodes_per_row) as f32;
            let row = (i / nodes_per_row) as f32;
            RenderedNode {
                slot: signal::Slot::from(i as u64),
                at: (origin_x + col * cell_w, origin_y + row * cell_h),
                kind_glyph: KindGlyph::Unknown,
                state_intent: NodeStateIntent::Stable,
                display_name: n.name.clone(),
            }
        })
        .collect();

    // Map edge endpoint slots into the row of rendered nodes
    // by modular indexing. Honest fallback until slot wire
    // shape ships.
    let n = rendered_nodes.len().max(1) as u64;
    let rendered_edges: Vec<RenderedEdge> = cache
        .edges
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let from_u: u64 = e.from.into();
            let to_u: u64 = e.to.into();
            RenderedEdge {
                slot: signal::Slot::from(i as u64),
                from: signal::Slot::from(from_u % n),
                to: signal::Slot::from(to_u % n),
                relation_intent: e.kind,
                state_intent: EdgeStateIntent::Stable,
            }
        })
        .collect();

    FlowGraphView {
        graph,
        nodes: rendered_nodes,
        edges: rendered_edges,
        pending_wire: None,
    }
}
