//! Layout record interpretation.
//!
//! A `Layout` record carries pane-visibility, pane-sizes, and
//! related semantic-intent positioning. Each shell maps these
//! to its native layout idioms.
//!
//! Until the user's first Layout assertion, mentci-lib uses
//! built-in defaults.

/// Layout intent currently applied. Derived from the active
/// `Layout` record (or the built-in default while none
/// exists).
pub struct LayoutState {
    pub source: LayoutSource,
    pub intents: LayoutIntents,
}

pub enum LayoutSource {
    BuiltinDefault,
    UserAsserted { slot: signal::Slot<signal::Layout> },
}

/// Named layout roles. Concrete fields mirror
/// [`signal::Layout`]; the workbench shell maps each named
/// `SizeIntent` to a native pixel/em value via its built-in
/// mapping table.
pub struct LayoutIntents {
    pub left_nav_width_intent: SizeIntent,
    pub inspector_width_intent: SizeIntent,
    pub diagnostics_height_intent: SizeIntent,
    pub wire_height_intent: SizeIntent,
    pub wire_pane_visible: bool,
    pub tweaks_pane_open: bool,
}

/// Semantic size hint that each shell maps to its native
/// pixel/em system. Mirrors `signal::SizeIntent`. Intent-only;
/// pixel overrides (when wired) live as a separate
/// `pixel_override: Option<u32>` field on the relevant
/// pane-state, not as a SizeIntent variant.
#[derive(Debug, Clone, Copy)]
pub enum SizeIntent {
    Narrow,
    Medium,
    Wide,
}

impl LayoutState {
    /// Built-in default — shipped with every shell.
    pub fn builtin_default() -> Self {
        Self {
            source: LayoutSource::BuiltinDefault,
            intents: LayoutIntents {
                left_nav_width_intent: SizeIntent::Narrow,
                inspector_width_intent: SizeIntent::Medium,
                diagnostics_height_intent: SizeIntent::Narrow,
                wire_height_intent: SizeIntent::Narrow,
                wire_pane_visible: false,
                tweaks_pane_open: false,
            },
        }
    }
}

impl Default for LayoutState {
    fn default() -> Self {
        Self::builtin_default()
    }
}
