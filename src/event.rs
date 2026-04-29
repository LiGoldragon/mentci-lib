//! [`UserEvent`] and [`EngineEvent`] — the two event types the
//! workbench accepts.
//!
//! Closed enums; one variant per kind of event. Adding a new
//! gesture or engine push grows the appropriate enum.

use signal::{Frame, Slot};

/// What the shell forwards when the user does something.
#[derive(Debug, Clone)]
pub enum UserEvent {
    // ── selection ──────────────────────────────────────────
    /// Pick a Graph in the GraphsNav.
    SelectGraph { slot: Slot },
    /// Pick any slot — Node, Edge, or another record kind.
    SelectSlot { slot: Slot },
    /// Pin a slot to keep it visible across selections.
    PinSlot { slot: Slot },
    /// Unpin a previously pinned slot.
    UnpinSlot { slot: Slot },

    // ── canvas: flow-graph gestures ────────────────────────
    /// Open the new-node constructor flow. Triggered by an
    /// explicit "+ node" affordance for now; drag-to-create
    /// lands as a richer gesture in a later iteration.
    OpenNewNodeFlow,
    /// Begin dragging a new box onto the canvas. Position +
    /// kind picked from the palette.
    BeginDragNewBox { graph: Slot, kind: NodeKindHint, at: CanvasPos },
    /// Mouse moves while dragging.
    UpdateDragNewBox { at: CanvasPos },
    /// Drop the new box. Triggers the constructor flow modal.
    DropDragNewBox { at: CanvasPos },

    /// Begin dragging a wire from a node.
    BeginDragWire { from: Slot, at: CanvasPos },
    /// Mouse moves while dragging the wire end.
    UpdateDragWire { at: CanvasPos },
    /// Drop the wire on a target. Triggers the edge constructor flow.
    DropDragWire { onto: Slot },

    // ── canvas: any-kind gestures ──────────────────────────
    /// Move a node on the canvas. Generates a Mutate to its
    /// position record on commit.
    MoveNode { slot: Slot, to: CanvasPos },
    /// Pan the canvas viewport.
    PanCanvas { delta: CanvasDelta },
    /// Zoom the canvas viewport.
    ZoomCanvas { factor: f32, anchor: CanvasPos },
    /// Scrub time on a kind whose canvas is time-anchored
    /// (astrological chart, timeline, calendar, …).
    ScrubTime { delta: TimeDelta },

    // ── constructor flow ───────────────────────────────────
    /// Update a field in the current constructor flow modal.
    ConstructorFieldChanged { field: ConstructorField },
    /// Commit the current constructor flow.
    ConstructorCommit,
    /// Cancel the current constructor flow.
    ConstructorCancel,

    // ── inspector / rename ─────────────────────────────────
    /// Begin editing a record's display name in place.
    BeginRename { slot: Slot },
    /// Commit a renamed value.
    CommitRename { slot: Slot, new_name: String, expected_rev: signal::Revision },
    /// Cancel a rename in progress.
    CancelRename,

    /// Retract a record (Node, Edge, …). Surfaces a confirm
    /// flow before the verb is sent.
    RequestRetract { slot: Slot },

    // ── pane toggles ───────────────────────────────────────
    ToggleWirePane,
    ToggleTweaksPane,
    PauseWire,
    ResumeWire,

    // ── diagnostics ────────────────────────────────────────
    ClearDiagnostics,
    JumpToDiagnosticTarget { diagnostic_id: u64 },

    // ── wire pane ──────────────────────────────────────────
    SetWireFilter { filter: WireFilter },

    // ── connection management ──────────────────────────────
    /// User asked to reconnect a dropped daemon.
    ReconnectCriome,
    ReconnectNexus,
}

/// What the runtime raises when something arrives from
/// outside (daemon push, reply, connection event, timer).
#[derive(Debug, Clone)]
pub enum EngineEvent {
    // ── connection lifecycle ───────────────────────────────
    CriomeConnected { protocol_version: String },
    CriomeDisconnected { reason: String },
    NexusConnected { protocol_version: String },
    NexusDisconnected { reason: String },

    // ── criome traffic ─────────────────────────────────────
    /// A subscription delivered records.
    SubscriptionPush { sub_id: u64, records: signal::Records },
    /// An outcome arrived for a previously sent edit.
    OutcomeArrived { req_id: u64, outcome: signal::OutcomeMessage },
    /// A typed query reply arrived.
    QueryReplied { req_id: u64, records: signal::Records },
    /// A diagnostic carried in any reply.
    DiagnosticEmitted { diagnostic: signal::Diagnostic },
    /// Frame seen on the wire (every direction).
    FrameSeen { direction: FrameDirection, frame: Frame },

    // ── nexus-daemon traffic ───────────────────────────────
    /// A nexus rendering arrived for a previously dispatched
    /// render request.
    NexusRendered { ticket: u64, text: String },
    /// A nexus parse arrived (used for completeness; humans
    /// don't author nexus, but the path stays exercised).
    NexusParsed { ticket: u64, frame: Frame },
}

/// Direction a frame moved on the wire.
#[derive(Debug, Clone, Copy)]
pub enum FrameDirection {
    Out,
    In,
    SubscriptionPush,
}

/// Which canvas position type we use. Concrete shape lands as
/// the canvas wires up.
#[derive(Debug, Clone, Copy)]
pub struct CanvasPos {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct CanvasDelta {
    pub dx: f32,
    pub dy: f32,
}

/// Time delta for scrub gestures on time-anchored canvas
/// kinds. Concrete shape — duration, signed direction —
/// lands as the first time-anchored kind is wired.
#[derive(Debug, Clone, Copy)]
pub struct TimeDelta {
    pub seconds: f64,
}

/// Hint of which node-kind the user is creating. Concrete
/// values come from the schema knowledge in [`crate::schema`].
#[derive(Debug, Clone)]
pub struct NodeKindHint {
    pub name: String,
}

/// A field-change event during a constructor flow. The
/// `field_name` namespaces it per pane; the active flow knows
/// which fields it owns.
#[derive(Debug, Clone)]
pub enum ConstructorField {
    /// Free-text input.
    Text { field_name: String, value: String },
    /// Selection from a closed-enum variant list.
    EnumChoice { field_name: String, variant: String },
}

/// Wire-pane filter expression. Concrete shape lands when
/// filter UI is built.
#[derive(Debug, Clone, Default)]
pub struct WireFilter {
    pub direction: Option<FrameDirection>,
    pub verb_name: Option<String>,
}
