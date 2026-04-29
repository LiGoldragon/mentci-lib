//! Diagnostics pane state + view.
//!
//! Every Outcome that's not `Ok`, every Reply carrying a
//! Diagnostic, lands here in chronological order with a
//! permanent jump-link to the slot or batch concerned. The
//! pane is shown only when the list is non-empty; failed
//! writes also overlay on the canvas at the affected node.

use signal::Slot;

pub struct DiagnosticsState {
    pub entries: Vec<DiagnosticEntry>,
}

pub struct DiagnosticsView {
    pub entries: Vec<DiagnosticEntryView>,
    pub unread_count: usize,
}

pub struct DiagnosticEntry {
    pub id: u64,
    pub timestamp_iso: String,
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub suggestion: Option<String>,
    pub jump_target: Option<Slot>,
    pub read: bool,
}

pub struct DiagnosticEntryView {
    pub id: u64,
    pub timestamp_iso: String,
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub suggestion: Option<String>,
    pub jump_target: Option<Slot>,
}

#[derive(Debug, Clone, Copy)]
pub enum DiagnosticSeverity {
    Ok,
    Warning,
    Error,
}
