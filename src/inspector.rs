//! Inspector pane state + view.
//!
//! Selected slot's complete state. Two stacked sections:
//! current state at the top, full change-log history below.
//! Every history entry's arrow scrubs the canvas backward to
//! that point in time.

use signal::{AnyKind, Hash, Revision, Slot};

/// State the inspector carries between events.
#[derive(Default)]
pub struct InspectorState {
    pub focused: Option<Slot<AnyKind>>,
    pub pinned: Vec<Slot<AnyKind>>,
    /// History pagination cursor for the focused slot's log.
    pub history_offset: usize,
}

/// What the shell paints.
pub struct InspectorView {
    pub focused: Option<FocusedSlotView>,
    pub pinned: Vec<PinnedSlotView>,
}

/// Detail for the currently focused slot.
pub struct FocusedSlotView {
    pub slot: Slot<AnyKind>,
    pub kind: String,
    pub display_name: String,
    pub rev: Revision,
    pub hash: Hash,
    pub last_write: WriteSummary,
    pub references_in: usize,
    pub references_out: usize,
    /// Rendered nexus form for the current record content.
    /// `None` while the render request is in flight or if
    /// nexus-daemon is down (in which case the shell hides
    /// the line).
    pub as_nexus: Option<String>,
    pub history: Vec<HistoryEntry>,
}

/// Compact pinned-slot summary. Selecting expands it into
/// the focused position.
pub struct PinnedSlotView {
    pub slot: Slot<AnyKind>,
    pub display_name: String,
    pub kind: String,
    pub rev: Revision,
}

/// One entry in the change-log.
pub struct HistoryEntry {
    pub rev: Revision,
    pub timestamp_iso: String,
    pub op_label: String,
    pub principal_label: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub hash_before: Option<Hash>,
    pub hash_after: Hash,
}

pub struct WriteSummary {
    pub timestamp_iso: String,
    pub op_label: String,
    pub principal_label: String,
}
