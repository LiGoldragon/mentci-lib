//! Canvas state + per-kind renderer dispatch.
//!
//! The canvas is *the* visualization pane. What it renders
//! depends on the selection's kind. mentci-lib holds a
//! per-kind renderer that produces kind-specific view-state;
//! the shell paints what the renderer says to paint.
//!
//! The first kind that ships is the flow-graph view in
//! [`flow_graph`]. Future kinds (astrological chart,
//! timelines, maps, calendars, statistical plots) each get
//! their own renderer module.

use signal::Slot;

pub mod flow_graph;

/// State the canvas pane carries between events.
pub struct CanvasState {
    /// Slot currently focused (the Graph being viewed,
    /// usually).
    pub focus: Option<Slot>,
    /// Viewport (pan + zoom) — lives locally for now;
    /// graduates to records once the LayoutPersistence kind
    /// matures.
    pub viewport: Viewport,
    /// Active per-kind renderer state — held abstractly here,
    /// resolved per kind by the renderer trait.
    pub kind_state: KindCanvasState,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            focus: None,
            viewport: Viewport::default(),
            kind_state: KindCanvasState::Empty,
        }
    }
}

/// Per-kind state. Variants map 1:1 to renderer impls.
pub enum KindCanvasState {
    Empty,
    FlowGraph(flow_graph::FlowGraphCanvasState),
    // future: AstroChart, Timeline, Map, …
}

/// The canvas snapshot the shell paints. Variants 1:1 with
/// per-kind renderers.
pub enum CanvasView {
    Empty,
    FlowGraph(flow_graph::FlowGraphView),
    // future: AstroChart(AstroChartView), …
}

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub center_x: f32,
    pub center_y: f32,
    pub zoom: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self { center_x: 0.0, center_y: 0.0, zoom: 1.0 }
    }
}

/// A canvas renderer for one record-kind family.
///
/// Implementors live in submodules of `canvas/`. Adding a new
/// kind = adding a submodule + extending [`KindCanvasState`]
/// and [`CanvasView`] with the new variant.
pub trait CanvasRenderer {
    type State;
    type View;

    /// Derive the kind-specific view-state from the records
    /// in scope plus this kind's canvas state.
    fn render(state: &Self::State /* + records, theme, layout */) -> Self::View;
}
