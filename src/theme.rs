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

use signal::IntentToken;

/// Theme intent currently applied. Derived from the active
/// `Theme` record (or the built-in default while none
/// exists).
pub struct ThemeState {
    /// Whether this is the built-in default or a user theme.
    pub source: ThemeSource,
    /// The semantic palette currently in effect.
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
/// Mirrors the field shape of [`signal::Theme`]; each shell
/// maps these tokens to its native palette.
pub struct ThemeIntents {
    pub bg: IntentToken,
    pub fg: IntentToken,
    pub accent: IntentToken,
    pub selected: IntentToken,
    pub pending: IntentToken,
    pub stale: IntentToken,
    pub rejected: IntentToken,
}

impl ThemeState {
    /// Built-in default — shipped with every shell so the
    /// surface paints something on first connect, before any
    /// Theme record exists.
    pub fn builtin_default() -> Self {
        Self {
            source: ThemeSource::BuiltinDefault,
            intents: ThemeIntents {
                bg: IntentToken::NeutralBg,
                fg: IntentToken::NeutralFg,
                accent: IntentToken::PrimaryAccent,
                selected: IntentToken::PrimaryAccent,
                pending: IntentToken::Pending,
                stale: IntentToken::Stale,
                rejected: IntentToken::Rejected,
            },
        }
    }
}

impl Default for ThemeState {
    fn default() -> Self {
        Self::builtin_default()
    }
}
