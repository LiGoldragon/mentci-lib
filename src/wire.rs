//! Wire pane state + view.
//!
//! Every signal frame seen on this connection, both
//! directions, at typed-variant level. User-toggled. When on,
//! frames carry an "[as nexus]" line rendered via the
//! nexus-daemon rendering service — same path agents use.
//!
//! For the first mentci-ui this pane shows frames from *this
//! connection only*. Engine-wide wire-tap (per criome ARCH
//! future work) lands later as an additive toggle.

use crate::event::FrameDirection;

pub struct WireState {
    pub paused: bool,
    pub filter: WireFilter,
    pub frames: Vec<WireEntry>,
}

pub struct WireView {
    pub paused: bool,
    pub frames: Vec<WireEntryView>,
}

#[derive(Debug, Clone, Default)]
pub struct WireFilter {
    pub direction: Option<FrameDirection>,
    pub verb_name: Option<String>,
}

pub struct WireEntry {
    pub timestamp_iso: String,
    pub direction: FrameDirection,
    pub req_id: Option<u64>,
    pub verb_summary: String,
    pub typed_payload_summary: String,
    /// Rendered nexus text — `None` while in flight or if
    /// nexus-daemon is down.
    pub as_nexus: Option<String>,
}

pub struct WireEntryView {
    pub timestamp_iso: String,
    pub direction: FrameDirection,
    pub req_id: Option<u64>,
    pub verb_summary: String,
    pub typed_payload_summary: String,
    pub as_nexus: Option<String>,
}
