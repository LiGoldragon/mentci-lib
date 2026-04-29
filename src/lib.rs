//! mentci-lib — heavy application logic for the mentci surface.
//!
//! Every mentci-* GUI shell consumes this library. The contract
//! is **data out, events in** — MVU-shaped.
//!
//! - [`state::WorkbenchState`] owns the model.
//! - [`view::WorkbenchView`] is the per-frame snapshot the shell
//!   paints.
//! - [`event::UserEvent`] is what the shell sends back when the
//!   user does something.
//! - [`event::EngineEvent`] is what daemon connections raise
//!   (subscription pushes, outcomes, diagnostics, render
//!   replies, connection state changes).
//! - [`cmd::Cmd`] is what the runtime dispatches outside the
//!   library (signal frames to send, nexus-daemon render
//!   requests, timers).
//!
//! The library is the application; each shell is the rendering.

pub mod canvas;
pub mod cmd;
pub mod connection;
pub mod constructor;
pub mod diagnostics;
pub mod error;
pub mod event;
pub mod inspector;
pub mod layout;
pub mod schema;
pub mod state;
pub mod theme;
pub mod view;
pub mod wire;

pub use error::{Error, Result};
pub use state::WorkbenchState;
pub use view::WorkbenchView;
pub use event::{EngineEvent, UserEvent};
pub use cmd::Cmd;
