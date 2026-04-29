//! Schema-aware constructor flows.
//!
//! Every editing gesture opens a context-specific flow that
//! knows the schema for the verb being constructed and
//! surfaces the right fields with the right typed options.
//!
//! Constructor-flow principles:
//!
//! - **Pre-show but uncommitted.** Visual preview (e.g. a
//!   dashed wire) appears as soon as the gesture completes;
//!   no signal frame leaves until commit.
//! - **Schema knowledge in mentci-lib.** Field shapes come
//!   from [`crate::schema`]; new variants in signal reach the
//!   GUI through this layer.
//! - **Validity narrows the choices.** When some
//!   source-kind/target-kind/RelationKind combinations are
//!   meaningless, only valid options surface.
//! - **Commit-at-flow-end.** No optimistic UI. Canvas
//!   reflects criome's accept; rejection vanishes the pending
//!   preview and surfaces a diagnostic.
//! - **Equivalence with the agent path.** Whatever an agent
//!   could send via nexus, a human can build via gestures.

use signal::{RelationKind, Slot};

/// At most one constructor flow is active at a time. The
/// variants enumerate every editable shape.
pub enum ActiveConstructor {
    /// Drag-new-box flow — placing a new node.
    NewNode(NewNodeFlow),
    /// Drag-wire flow — creating an edge.
    NewEdge(NewEdgeFlow),
    /// Rename-in-place — changing a record's display name.
    Rename(RenameFlow),
    /// Retract confirm — destructive, surface confirmation.
    Retract(RetractFlow),
    /// AtomicBatch composer — multi-op edits.
    Batch(BatchFlow),
}

/// What the shell renders for the active constructor. Pure
/// data; one variant per [`ActiveConstructor`].
pub enum ConstructorView {
    NewNode(NewNodeView),
    NewEdge(NewEdgeView),
    Rename(RenameView),
    Retract(RetractView),
    Batch(BatchView),
}

// ── New-Node flow ───────────────────────────────────────────

pub struct NewNodeFlow {
    pub graph: Slot,
    pub at_x: f32,
    pub at_y: f32,
    pub kind_choice: Option<String>,
    pub display_name_input: String,
}

pub struct NewNodeView {
    pub kind_choices: Vec<String>,
    pub kind_choice: Option<String>,
    pub display_name_input: String,
    pub commit_enabled: bool,
}

// ── New-Edge flow ───────────────────────────────────────────

pub struct NewEdgeFlow {
    pub from: Slot,
    pub to: Slot,
    pub kind_choice: Option<RelationKind>,
    pub description_input: String,
}

pub struct NewEdgeView {
    pub from_label: String,
    pub to_label: String,
    /// RelationKind variants narrowed by validity per
    /// source/target kind pair. The schema layer narrows;
    /// this view just renders.
    pub kind_choices: Vec<RelationKind>,
    pub kind_choice: Option<RelationKind>,
    pub description_input: String,
    pub commit_enabled: bool,
}

// ── Rename flow ─────────────────────────────────────────────

pub struct RenameFlow {
    pub slot: Slot,
    pub current_name: String,
    pub new_name: String,
    pub expected_rev: signal::Revision,
}

pub struct RenameView {
    pub slot_label: String,
    pub current_name: String,
    pub new_name: String,
    pub commit_enabled: bool,
}

// ── Retract flow ────────────────────────────────────────────

pub struct RetractFlow {
    pub slot: Slot,
    pub expected_rev: signal::Revision,
    pub references_in: Vec<Slot>,
    pub references_out: Vec<Slot>,
}

pub struct RetractView {
    pub slot_label: String,
    pub references_count: usize,
    pub warning: Option<String>,
    pub commit_enabled: bool,
}

// ── Batch flow ──────────────────────────────────────────────

pub struct BatchFlow {
    pub ops: Vec<BatchOp>,
}

pub enum BatchOp {
    // todo!() — populated as the constructor-flow shape lands
}

pub struct BatchView {
    pub op_count: usize,
    pub commit_enabled: bool,
}
