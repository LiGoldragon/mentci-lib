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
use crate::constructor::{
    ActiveConstructor, ConstructorView, NewNodeFlow, NewNodeView,
};
use crate::diagnostics::DiagnosticsState;
use crate::event::{ConstructorField, EngineEvent, UserEvent};
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
/// Each record is stored with its sema slot — the typed wire
/// shape (`signal::Records::*`) now carries `Vec<(Slot, Kind)>`
/// per criome's records-with-slots rollout. That slot is the
/// real cross-record identity; edge endpoints can be resolved
/// to specific Node entries by slot lookup.
#[derive(Default)]
pub struct ModelCache {
    pub graphs: Vec<(Slot, Graph)>,
    pub nodes: Vec<(Slot, Node)>,
    pub edges: Vec<(Slot, Edge)>,
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

    /// Find the position of a Node by slot. `O(n)` for now —
    /// indexed lookups land if profiling demands.
    pub fn node_position_by_slot(&self, slot: Slot) -> Option<usize> {
        self.nodes.iter().position(|(s, _)| *s == slot)
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
            constructor: self.active_constructor.as_ref().map(constructor_view_for),
        }
    }

    /// Build the canvas view per current kind-state.
    fn canvas_view(&self) -> CanvasView {
        match &self.canvas.kind_state {
            KindCanvasState::Empty => CanvasView::Empty,
            KindCanvasState::FlowGraph(_kstate) => {
                let focus = match self.canvas.focus {
                    Some(s) => s,
                    None => return CanvasView::Empty,
                };
                let graph = match self
                    .cache
                    .graphs
                    .iter()
                    .find(|(s, _)| *s == focus)
                    .map(|(_, g)| g.clone())
                {
                    Some(g) => g,
                    None => return CanvasView::Empty,
                };
                let view = build_flow_graph_view(graph, &self.cache);
                CanvasView::FlowGraph(view)
            }
        }
    }

    /// Build the GraphsNav pane view from the cache. Each
    /// entry carries the record's real sema slot; selection
    /// round-trips correctly to the engine.
    fn graphs_nav_view(&self) -> GraphsNavView {
        let graphs = self
            .cache
            .graphs
            .iter()
            .map(|(slot, g)| GraphsNavEntry {
                slot: *slot,
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
            UserEvent::OpenNewNodeFlow => {
                // The constructor needs *some* graph context;
                // pick the focused one (or the first cached
                // graph as a fallback). If neither exists, the
                // gesture is a no-op until the user selects a
                // graph.
                let graph_slot = self.canvas.focus.or_else(|| {
                    self.cache.graphs.first().map(|(s, _)| *s)
                });
                if let Some(graph) = graph_slot {
                    self.active_constructor = Some(ActiveConstructor::NewNode(NewNodeFlow {
                        graph,
                        at_x: 0.0,
                        at_y: 0.0,
                        kind_choice: None,
                        display_name_input: String::new(),
                    }));
                }
                Vec::new()
            }
            UserEvent::ConstructorFieldChanged { field } => {
                if let Some(ActiveConstructor::NewNode(flow)) =
                    self.active_constructor.as_mut()
                {
                    if let ConstructorField::Text { field_name, value } = field {
                        if field_name == "name" {
                            flow.display_name_input = value;
                        }
                    }
                }
                Vec::new()
            }
            UserEvent::ConstructorCommit => {
                let cmd = self.commit_active_constructor();
                cmd.into_iter().collect()
            }
            UserEvent::ConstructorCancel => {
                self.active_constructor = None;
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

    /// Commit whatever constructor is active. Closes the flow
    /// and returns a Cmd that sends the appropriate Assert /
    /// Mutate / Retract frame to criome. After the engine's
    /// outcome arrives, the canvas re-queries; the new record
    /// shows up in the cache and the view paints it.
    fn commit_active_constructor(&mut self) -> Option<Cmd> {
        let active = self.active_constructor.take()?;
        match active {
            ActiveConstructor::NewNode(flow) => {
                if flow.display_name_input.is_empty() {
                    // Empty name — keep the flow open by
                    // restoring it. The shell could surface a
                    // hint; the model just refuses to commit.
                    self.active_constructor =
                        Some(ActiveConstructor::NewNode(flow));
                    return None;
                }
                let assert = signal::AssertOperation::Node(signal::Node {
                    name: flow.display_name_input,
                });
                let frame = Frame {
                    principal_hint: Some(self.principal),
                    auth_proof: Some(AuthProof::SingleOperator),
                    body: Body::Request(Request::Assert(assert)),
                };
                Some(Cmd::SendCriome { frame })
            }
            // Other constructor flows commit when their bodies
            // wire up. Putting them back into the slot avoids
            // dropping in-progress state.
            other => {
                self.active_constructor = Some(other);
                None
            }
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
/// Each cached Node carries its real sema slot; each cached
/// Edge's `from`/`to` slots resolve to specific cached Nodes
/// via slot lookup. The horizontal-grid layout is the
/// first-paint placeholder until `NodePlacement` records flow
/// through the wire.
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
        .map(|(i, (slot, n))| {
            let col = (i % nodes_per_row) as f32;
            let row = (i / nodes_per_row) as f32;
            RenderedNode {
                slot: *slot,
                at: (origin_x + col * cell_w, origin_y + row * cell_h),
                kind_glyph: KindGlyph::Unknown,
                state_intent: NodeStateIntent::Stable,
                display_name: n.name.clone(),
            }
        })
        .collect();

    // Edge endpoints now reference real sema slots; resolution
    // is direct cache lookup. Edges whose endpoints aren't in
    // the cached node set are rendered with their slot
    // unchanged — the paint layer drops them when the lookup
    // fails.
    let rendered_edges: Vec<RenderedEdge> = cache
        .edges
        .iter()
        .map(|(slot, e)| RenderedEdge {
            slot: *slot,
            from: e.from,
            to: e.to,
            relation_intent: e.kind,
            state_intent: EdgeStateIntent::Stable,
        })
        .collect();

    FlowGraphView {
        graph,
        nodes: rendered_nodes,
        edges: rendered_edges,
        pending_wire: None,
    }
}

/// Project the active constructor into its renderable view.
fn constructor_view_for(active: &ActiveConstructor) -> ConstructorView {
    match active {
        ActiveConstructor::NewNode(flow) => ConstructorView::NewNode(NewNodeView {
            // Concrete kind choices come from the schema layer
            // when it lands. For the first runnable: a single
            // "Node" placeholder so commit can fire.
            kind_choices: vec!["Node".to_string()],
            kind_choice: flow.kind_choice.clone(),
            display_name_input: flow.display_name_input.clone(),
            commit_enabled: !flow.display_name_input.is_empty(),
        }),
        ActiveConstructor::NewEdge(flow) => ConstructorView::NewEdge(crate::constructor::NewEdgeView {
            from_label: format!("slot {}", u64::from(flow.from)),
            to_label: format!("slot {}", u64::from(flow.to)),
            kind_choices: Vec::new(),
            kind_choice: flow.kind_choice,
            description_input: flow.description_input.clone(),
            commit_enabled: flow.kind_choice.is_some(),
        }),
        ActiveConstructor::Rename(flow) => ConstructorView::Rename(crate::constructor::RenameView {
            slot_label: format!("slot {}", u64::from(flow.slot)),
            current_name: flow.current_name.clone(),
            new_name: flow.new_name.clone(),
            commit_enabled: flow.new_name != flow.current_name && !flow.new_name.is_empty(),
        }),
        ActiveConstructor::Retract(flow) => ConstructorView::Retract(crate::constructor::RetractView {
            slot_label: format!("slot {}", u64::from(flow.slot)),
            references_count: flow.references_in.len() + flow.references_out.len(),
            warning: None,
            commit_enabled: true,
        }),
        ActiveConstructor::Batch(flow) => ConstructorView::Batch(crate::constructor::BatchView {
            op_count: flow.ops.len(),
            commit_enabled: !flow.ops.is_empty(),
        }),
    }
}
