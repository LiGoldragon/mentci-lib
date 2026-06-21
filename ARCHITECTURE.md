# ARCHITECTURE — mentci-lib

The client-side observability + control model for the Mentci component.
Thin Mentci clients (the mentci CLI and mentci-egui first) consume this
one model so painted client state follows canonical daemon state without
duplicating approval logic.

This crate was re-founded on the LIVE contracts (forensic sub-report 5;
Spirit 7x5z). The original 2026-04-29 design predated the daemon and
`signal-mentci` by ~50 days and was built on a graph-signal transport
(Graph / Node / Edge / Slot / Handshake) that never shipped, a
non-existent dual-daemon `DaemonRole::Criome/Nexus` model, and a
hand-rolled approval vocabulary that `signal-mentci` now owns. The
valuable shape survived the rebase: the MVU model-view-update contract,
edits-as-proposals, and the approval state machine.

## What the crate IS now

```
            ┌──────────────────────────────────────────┐
            │              mentci-lib                   │
            │    CLIENT OBSERVABILITY + CONTROL MODEL    │
            │                                           │
            │  observation::ObservationModel  (MVU)     │
            │    keyed by ComponentSocketKind           │
            │    update(state, event) -> Vec<Cmd>       │
            │    view(state) -> ObservationView         │
            │                                           │
            │  approval::ApprovalModel                  │
            │    pending queue + selection cursor       │
            │    + local subscription fan-out           │
            │    + edits-as-proposals                   │
            │                                           │
            │  render::RenderNota                       │
            │    NOTA-fallback for typed replies/objects│
            │                                           │
            │  decision::CriomeVerdict                  │
            │    typed ApprovalDecision -> criome       │
            │    AuthorizationApprovalDecision mapping  │
            └───────────┬───────────────────────────────┘
                        │ consumes the live contracts
        ┌───────────────┼───────────────┬───────────────┐
        ▼               ▼               ▼               ▼
   signal-mentci   meta-signal-    signal-criome   meta-signal-
   (working wire)   mentci          (AuthRequest-   criome
   ApprovalQuestion (ComponentS-    Slot)           (AuthApproval-
   InterfaceState   ocketKind)                      Decision)
   ProjectedI-fcSt  Configure
   MentciEvent
```

## The MVU contract

The whole client surface is two methods on `ObservationModel`:

- `on_user_event(UserEvent) -> Vec<Cmd>` and
  `on_engine_event(EngineEvent) -> Vec<Cmd>` — the `update` half.
- `view() -> ObservationView` — the pure-data snapshot the shell paints.

Side-effects never run inside the model; a `Cmd` describes a
`signal-mentci` request addressed to a `ComponentSocketKind`, and the
outer runtime owns the `signal-frame` transport that turns it into a
`MentciFrame`. Clients answer approval questions by sending
`AnswerQuestion` to the mentci daemon. The daemon owns the criome bridge,
routes criome-sourced answers by parked slot when it has write authority,
and mirrors that authority as `CriomeAccess` in full interface projections.
Keeping side-effects out is the MVU property: `update` is a pure function
of `(state, event)`.

## Keyed by component socket

`ObservationModel` holds one `SocketObservation` per
`ComponentSocketKind` (`Mentci`, `MetaMentci`, `Criome`, `MetaCriome`
from `meta-signal-mentci` / signal-standard). Each slot carries the
interest it subscribed with, the daemon-minted `SubscriptionToken`, the
latest `ProjectedInterfaceState`, and the connection liveness. A thin
client can mirror daemon-projected read/write capability in these views,
but criome approval submission still goes through the mentci daemon.
`ObservationView::criome_access` is `Some(ReadOnly | ReadWrite)` after a
full Mentci projection is folded; `None` means the client has not learned the
mode and should remain observation-only.

## Approval state machine (kept, rebased)

`ApprovalModel` consumes `signal-mentci`'s OWN approval vocabulary —
`ApprovalQuestion`, `ApprovalDecision`, `ApprovalVerdict`,
`AnswerProposal` — never a duplicate. It owns the *client-side* logic the
contract does not: the pending-question cursor (mirrored from the
daemon's projected queue), local subscription fan-out for thin surfaces
(status bar, popup, approval pane), and the closed-verdict +
edits-as-proposals constructors. The daemon still owns minting question
identifiers and the canonical `InterfaceState`; the model reads the
projected slice.

## Closed-decision -> criome mapping (t00s)

`decision::CriomeVerdict::from_decision(slot, decision)` projects a
closed `ApprovalDecision` onto the `AuthorizationRequestSlot` criome
parked, yielding a `meta-signal-criome` `AuthorizationApprovalDecision`.
This is the named contact point where the two enums meet
(`skills/enum-contact-points.md`). The mapping is reusable by the daemon,
but producing and submitting criome verdicts is not a client-library side
effect.

## NOTA-fallback rendering (xlrk)

`render::RenderNota` is a blanket affordance: any object that projects
itself to NOTA becomes a labeled `RenderedObject`. A thin client paints
whatever purpose-built view it has and falls back to the typed object's
NOTA projection for everything else — the same path agents use. With the
`nota-text` feature this is the real `nota-next` projection; without it
the model still compiles and falls back to `Debug`.

## Universal-client seed (introspect::IntrospectClient)

The first proof that mentci-lib is a *universal* client, not a single-triad
one: `introspect::IntrospectClient` talks to a SECOND component — the
introspect daemon — through introspect's own `signal-introspect` contract.
It owns the introspection-query socket path and a typed `component_trace`
method that sends `IntrospectionRequest::ComponentTrace(ComponentTraceQuery)`
over a length-prefixed `signal-frame` (the generated codec, no hand-rolled
framing) and returns the typed `ComponentTrace` reply. The wire shape mirrors
the introspect daemon's own in-tree client exactly: request is
`encode_length_prefixed`; the daemon answers with a 4-byte length prefix
wrapping a bare frame archive, decoded with `IntrospectionFrame::decode`.

The mentci MVU core (`ObservationModel`) stays keyed by `signal-mentci`
exclusively — introspect is a query surface, not canonical mentci state, so
it lives as its own data-bearing noun a shell calls off-thread and renders
through `RenderNota`, the same path the shell already uses for daemon replies.
When the next slice folds introspect observations into a model, this client is
the transport it dispatches. The introspect failures flow through the same
crate `Error` (`IntrospectSocket`, `UnexpectedIntrospectFrame`).

## Stack discipline

Closed enums; one typed `Error` enum (`thiserror`); full English words
with no crate-name prefix; methods on data-bearing types, no free
functions; schema-emitted contract types are the nouns and behavior
attaches to them. Per `~/primary/skills/rust-discipline.md`.

## Adoption

- **mentci CLI and mentci-egui** consume `ObservationModel` +
  `RenderNota` today: each shell
  holds the model, feeds typed replies in as `EngineEvent`s, and renders
  every reply through the shared renderer. The shell owns no approval
  logic or per-socket state of its own.
- **mentci daemon** owns canonical state and effects: its `state.rs`
  builds the contract types, its criome bridge observes parked questions
  and submits verdicts, and its projected interface state is what clients
  mirror. It may reuse the typed verdict mapping from this crate, while
  persistence, sockets, key-unlock, and lifecycle remain daemon-local.

## Scaffold note (designer prototype)

This branch consumes a matching `signal-mentci` feature branch by local
`[patch]` because the shared model needs public READERS on projected
interface state (`pending_questions`, `panes`, `notification`,
`suggested_answer`, `context`) the operator's main does not yet expose.
Those readers attach to the schema-emitted nouns in `signal-mentci`'s
hand-written `lib.rs`. When the operator merges the readers, the patch
collapses and the plain git dependency resolves with no other change.
