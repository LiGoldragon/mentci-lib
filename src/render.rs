//! NOTA-fallback rendering (the `xlrk` render-typed-replies-and-unknown-objects
//! behavior).
//!
//! A thin client paints whatever purpose-built view it has, and falls back to
//! the typed object's NOTA projection for everything it does not yet
//! special-case. mentci-egui currently does exactly this by hand for every
//! reply; re-founded here it is a shared affordance so every client (and the
//! daemon's own introspection surface) renders the same way.
//!
//! The renderer is content-agnostic: anything that knows how to project itself
//! to NOTA can become a [`RenderedObject`]. With the `nota-text` feature this
//! is the real `nota-next` projection; without it the model still compiles and
//! falls back to the `Debug` shape so the shared core builds with or without
//! the codec.

/// Where a rendered object came from, so the shell can label the block. A
/// typed enum, not a free-text tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderOrigin {
    /// A request the client sent.
    Request,
    /// A reply a daemon returned.
    Reply,
    /// A subscription event a daemon pushed.
    Event,
    /// Projected interface state.
    ProjectedState,
}

impl RenderOrigin {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Reply => "reply",
            Self::Event => "event",
            Self::ProjectedState => "projected state",
        }
    }
}

/// One rendered object: the origin that labels it plus its NOTA body. This is
/// the unit a shell drops into a scrollable transcript when it has no
/// purpose-built view for the object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedObject {
    origin: RenderOrigin,
    body: String,
}

impl RenderedObject {
    pub fn origin(&self) -> &RenderOrigin {
        &self.origin
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn into_body(self) -> String {
        self.body
    }
}

/// Project a typed object to a [`RenderedObject`] as NOTA. Implemented for
/// every contract type through the blanket impl below; a client calls
/// `reply.render_nota(RenderOrigin::Reply)` on any reply, event, or unknown
/// object and gets a uniformly-rendered block back.
pub trait RenderNota {
    fn render_nota(&self, origin: RenderOrigin) -> RenderedObject;
}

#[cfg(feature = "nota-text")]
impl<Object> RenderNota for Object
where
    Object: nota_next::NotaEncode,
{
    fn render_nota(&self, origin: RenderOrigin) -> RenderedObject {
        RenderedObject {
            origin,
            body: nota_next::NotaEncode::to_nota(self),
        }
    }
}

#[cfg(not(feature = "nota-text"))]
impl<Object> RenderNota for Object
where
    Object: core::fmt::Debug,
{
    fn render_nota(&self, origin: RenderOrigin) -> RenderedObject {
        RenderedObject {
            origin,
            body: format!("{self:?}"),
        }
    }
}
