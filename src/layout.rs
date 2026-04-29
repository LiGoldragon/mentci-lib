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
    UserAsserted { slot: signal::Slot },
}

/// Named layout roles. Concrete fields land as the Layout
/// record kind shape finalises.
pub struct LayoutIntents {
    pub left_nav_width_intent: SizeIntent,
    pub inspector_width_intent: SizeIntent,
    pub diagnostics_height_intent: SizeIntent,
    pub wire_height_intent: SizeIntent,
    pub wire_pane_visible: bool,
    pub tweaks_pane_open: bool,
}

/// Semantic size hint that each shell maps to its native
/// pixel/em system.
#[derive(Debug, Clone, Copy)]
pub enum SizeIntent {
    Narrow,
    Medium,
    Wide,
    /// Concrete pixel hint, when the user has dragged a
    /// splitter to a specific value.
    Pixels(u32),
}
