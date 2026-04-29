//! Theme record interpretation.
//!
//! A `Theme` record carries semantic-intent palette names —
//! `selected`, `stale`, `rejected`, `pending`, `bg`, `fg`,
//! `accent`, etc. Each shell maps these names to its native
//! palette. Themes describe *intent* not *appearance*; the
//! same Theme record is renderable in egui, iced, and Flutter
//! shells without changes.
//!
//! Until the user's first Theme assertion, mentci-lib uses
//! built-in defaults so a fresh sema renders a usable
//! workbench on first connect.

/// Theme intent currently applied. Derived from the active
/// `Theme` record (or the built-in default while none
/// exists).
pub struct ThemeState {
    /// Whether this is the built-in default or a user theme.
    pub source: ThemeSource,
    /// Concrete intent slots — the palette of named roles.
    /// Populated once the Theme record kind is wired and
    /// genesis defaults exist.
    pub intents: ThemeIntents,
}

/// Where the current theme comes from.
pub enum ThemeSource {
    /// No Theme record asserted yet — using the built-in
    /// fallback.
    BuiltinDefault,
    /// A `Theme` record from sema.
    UserAsserted { slot: signal::Slot },
}

/// Named palette roles — the semantic intents themes carry.
/// Concrete fields land as the Theme record kind shape
/// finalises; the contract here is "named intent slots".
pub struct ThemeIntents {
    // todo!() — populated when the Theme kind lands in signal
}
