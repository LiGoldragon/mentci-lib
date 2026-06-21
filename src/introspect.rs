//! [`IntrospectClient`] — the universal-client seed: mentci-lib reaching a
//! *second* component, the introspect daemon, over its own `signal-introspect`
//! contract.
//!
//! Until now mentci-lib spoke only `signal-mentci` (its triad's working
//! signal). This module is the first proof of the universal-client direction:
//! the same client library talks to introspect through introspect's typed
//! contract, with no hand-rolled framing — it builds an `IntrospectionFrame`,
//! length-prefixes it through the generated `signal-frame` codec, and decodes
//! the typed `ComponentTrace` reply.
//!
//! The mentci MVU core ([`crate::observation::ObservationModel`]) stays keyed
//! by `signal-mentci` exclusively; introspect is a query surface, not canonical
//! mentci state, so it lives as its own data-bearing noun the shell calls
//! off-thread and renders through [`crate::render::RenderNota`] — exactly how
//! the egui shell already calls the mentci daemon client. When the next slice
//! folds introspect observations into a model, this client is the transport it
//! dispatches.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use signal_frame::{
    ExchangeIdentifier, ExchangeLane, LaneSequence, Reply, Request, SessionEpoch, SubReply,
};
use signal_introspect::{
    ComponentTrace, ComponentTraceQuery, IntrospectionFrame, IntrospectionFrameBody,
    IntrospectionReply, IntrospectionRequest, IntrospectionTarget, TraceEventName,
};
use signal_persona::EngineIdentifier;

use crate::error::{Error, Result};

/// A blocking client for the introspect daemon's introspection-query socket.
/// Owns the socket path and the maximum reply size it will read; every query
/// method dials a fresh connection, mirroring the per-request connection model
/// the mentci daemon client uses.
///
/// This is the data-bearing noun the verb "query introspect" belongs to. The
/// shell holds one and calls its typed methods; the typed reply it returns is
/// what a shell renders or (in the next slice) folds into a model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntrospectClient {
    socket: PathBuf,
    maximum_reply_bytes: usize,
}

impl IntrospectClient {
    const DEFAULT_MAXIMUM_REPLY_BYTES: usize = 16 * 1024 * 1024;

    /// Construct a client bound to an introspection-query socket path.
    pub fn new(socket: impl Into<PathBuf>) -> Self {
        Self {
            socket: socket.into(),
            maximum_reply_bytes: Self::DEFAULT_MAXIMUM_REPLY_BYTES,
        }
    }

    /// The socket path this client dials, for a shell to display.
    pub fn socket(&self) -> &PathBuf {
        &self.socket
    }

    /// Query the introspect daemon for one component's actor-boundary trace,
    /// optionally narrowed to a single event name. Sends
    /// [`IntrospectionRequest::ComponentTrace`] over a length-prefixed
    /// `signal-frame` and returns the typed [`ComponentTrace`] reply.
    pub fn component_trace(
        &self,
        engine: EngineIdentifier,
        component: IntrospectionTarget,
        event_name: Option<TraceEventName>,
    ) -> Result<ComponentTrace> {
        let query = ComponentTraceQuery::new(engine, component, event_name);
        match self.submit(IntrospectionRequest::ComponentTrace(query))? {
            IntrospectionReply::ComponentTrace(trace) => Ok(trace),
            other => Err(Error::UnexpectedIntrospectFrame {
                detail: format!("expected ComponentTrace reply, got {other:?}"),
            }),
        }
    }

    /// Send a typed `signal-introspect` request and return the typed reply. The
    /// wire shape mirrors the introspect daemon's own in-tree client: the
    /// request is `encode_length_prefixed` (4-byte big-endian length + bare
    /// archive); the daemon answers with the same length-prefix wrapping a bare
    /// frame archive, so the body is decoded with `IntrospectionFrame::decode`.
    fn submit(&self, request: IntrospectionRequest) -> Result<IntrospectionReply> {
        let mut stream = UnixStream::connect(&self.socket)?;
        let frame = IntrospectionFrame::new(IntrospectionFrameBody::Request {
            exchange: self.exchange(),
            request: Request::from_payload(request),
        });
        stream.write_all(&frame.encode_length_prefixed()?)?;
        stream.flush()?;
        self.read_reply(&mut stream)
    }

    fn read_reply(&self, stream: &mut UnixStream) -> Result<IntrospectionReply> {
        let mut prefix = [0_u8; 4];
        stream.read_exact(&mut prefix)?;
        let length = u32::from_be_bytes(prefix) as usize;
        if length > self.maximum_reply_bytes {
            return Err(Error::UnexpectedIntrospectFrame {
                detail: format!(
                    "reply length {length} exceeds maximum {}",
                    self.maximum_reply_bytes
                ),
            });
        }
        let mut bytes = vec![0_u8; length];
        stream.read_exact(&mut bytes)?;
        match IntrospectionFrame::decode(&bytes)?.into_body() {
            IntrospectionFrameBody::Reply { reply, .. } => self.reply_output(reply),
            other => Err(Error::UnexpectedIntrospectFrame {
                detail: format!("{other:?}"),
            }),
        }
    }

    /// Extract the typed `IntrospectionReply` from an accepted single-operation
    /// reply head — the same projection the introspect daemon's in-tree client
    /// performs.
    fn reply_output(&self, reply: Reply<IntrospectionReply>) -> Result<IntrospectionReply> {
        match reply {
            Reply::Accepted { per_operation, .. } => match per_operation.into_head() {
                SubReply::Ok(payload) => Ok(payload),
                other => Err(Error::UnexpectedIntrospectFrame {
                    detail: format!("{other:?}"),
                }),
            },
            Reply::Rejected { reason } => Err(Error::UnexpectedIntrospectFrame {
                detail: format!("rejected: {reason:?}"),
            }),
        }
    }

    fn exchange(&self) -> ExchangeIdentifier {
        ExchangeIdentifier::new(
            SessionEpoch::new(1),
            ExchangeLane::Connector,
            LaneSequence::new(1),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use signal_frame::NonEmpty;
    use signal_introspect::{ComponentTraceEvent, TraceLayer, TraceSequence};

    /// The client builds a ComponentTrace request frame that decodes back to the
    /// exact typed query it was given — proving the universal-client seed frames
    /// the introspect operation with the generated codec, not a hand-rolled one.
    #[test]
    fn component_trace_request_frame_round_trips_through_the_introspect_codec() {
        let client = IntrospectClient::new("/tmp/introspect.socket");
        let query = ComponentTraceQuery::new(
            EngineIdentifier::new("prototype"),
            IntrospectionTarget::Signal,
            Some(TraceEventName::new("SignalAdmitted")),
        );
        let frame = IntrospectionFrame::new(IntrospectionFrameBody::Request {
            exchange: client.exchange(),
            request: Request::from_payload(IntrospectionRequest::ComponentTrace(query.clone())),
        });

        let bytes = frame.encode_length_prefixed().expect("encode request frame");
        // The daemon strips the 4-byte length prefix then decodes the bare body.
        let body = &bytes[4..];
        let decoded = IntrospectionFrame::decode(body).expect("decode request frame");
        match decoded.into_body() {
            IntrospectionFrameBody::Request { request, .. } => {
                assert_eq!(
                    request.payloads().clone().into_head(),
                    IntrospectionRequest::ComponentTrace(query)
                );
            }
            other => panic!("expected request frame, got {other:?}"),
        }
    }

    /// The client projects an accepted ComponentTrace reply frame — built the
    /// way the introspect daemon builds it — back to the typed `ComponentTrace`,
    /// so the egui shell renders real events, not a wire artifact.
    #[test]
    fn accepted_component_trace_reply_projects_to_the_typed_reply() {
        let client = IntrospectClient::new("/tmp/introspect.socket");
        let engine = EngineIdentifier::new("prototype");
        let events = vec![ComponentTraceEvent::new(
            engine.clone(),
            IntrospectionTarget::Signal,
            TraceLayer::Signal,
            TraceEventName::new("SignalAdmitted"),
            TraceSequence::new(1),
        )];
        let trace = ComponentTrace::new(engine, IntrospectionTarget::Signal, events.clone());
        let reply = Reply::committed(NonEmpty::single(SubReply::Ok(
            IntrospectionReply::ComponentTrace(trace.clone()),
        )));

        match client.reply_output(reply).expect("project accepted reply") {
            IntrospectionReply::ComponentTrace(returned) => {
                assert_eq!(returned, trace);
                assert_eq!(returned.events(), events.as_slice());
            }
            other => panic!("expected ComponentTrace reply, got {other:?}"),
        }
    }
}
