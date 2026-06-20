//! mentci-lib — the shared observability + control model for the mentci
//! programmable-UI component.
//!
//! Re-founded on the LIVE contracts (Spirit 7x5z, forensic sub-report 5). The
//! original design predated the daemon and `signal-mentci` by ~50 days and was
//! built on a graph-signal transport that never existed; this crate keeps the
//! valuable shape — MVU, edits-as-proposals, the approval state machine — and
//! rebases it onto what is real now:
//!
//! - [`observation::ObservationModel`] — the MVU core, keyed by component
//!   socket ([`meta_signal_mentci::ComponentSocketKind`]). `update(state,
//!   event) -> (state, Vec<Cmd>)` and `view(state) -> ObservationView`.
//! - [`approval::ApprovalModel`] — the approval state machine + edits-as-
//!   proposals, expressed over `signal-mentci`'s OWN approval vocabulary
//!   (`ApprovalQuestion` / `ApprovalDecision` / `ApprovalVerdict` /
//!   `AnswerProposal`), never a duplicate.
//! - [`render::RenderNota`] — the NOTA-fallback renderer (the
//!   render-typed-replies-and-unknown-objects behavior).
//! - [`decision::CriomeVerdict`] — the closed-decision -> criome verdict
//!   mapping (Spirit t00s), consuming the real `signal-criome` /
//!   `meta-signal-criome` types.
//!
//! The library is the application; each shell (mentci-egui first) is the
//! rendering. The daemon shares the same model so its canonical state and the
//! clients' painted state cannot drift.

pub mod approval;
pub mod cmd;
pub mod decision;
pub mod error;
pub mod event;
pub mod observation;
pub mod render;

/// The component-socket roster the model is keyed by, re-exported from
/// `meta-signal-mentci` so consumers address sockets through the shared
/// surface rather than depending on the meta contract directly.
pub use meta_signal_mentci::ComponentSocketKind;

pub use cmd::Cmd;
pub use decision::{CriomeDecision, CriomeVerdict};
pub use error::{Error, Result};
pub use event::{EngineEvent, SocketLiveness, UserEvent};
pub use observation::{
    ObservationModel, ObservationView, PresentableQuestion, SocketObservation, SocketView,
};
pub use render::{RenderNota, RenderOrigin, RenderedObject};
