# ARCHITECTURE — mentci-lib

The shared observability + control model for the Mentci component. The
`mentci` daemon and every thin Mentci client (mentci-egui first) consume
this one model so canonical daemon state and painted client state cannot
drift.

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
            │   SHARED OBSERVABILITY + CONTROL MODEL    │
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
            │    closed ApprovalDecision -> criome      │
            │    AuthorizationApprovalDecision (t00s)   │
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

The whole surface is two methods on `ObservationModel`:

- `on_user_event(UserEvent) -> Vec<Cmd>` and
  `on_engine_event(EngineEvent) -> Vec<Cmd>` — the `update` half.
- `view() -> ObservationView` — the pure-data snapshot the shell paints.

Side-effects never run inside the model; a `Cmd` describes a
`signal-mentci` request addressed to a `ComponentSocketKind`, and the
outer runtime owns the `signal-frame` transport that turns it into a
`MentciFrame`. Keeping side-effects out is the MVU property:
`update` is a pure function of `(state, event)`.

## Keyed by component socket

`ObservationModel` holds one `SocketObservation` per
`ComponentSocketKind` (`Mentci`, `MetaMentci`, `Criome`, `MetaCriome`
from `meta-signal-mentci` / signal-standard). Each slot carries the
interest it subscribed with, the daemon-minted `SubscriptionToken`, the
latest `ProjectedInterfaceState`, and the connection liveness. Mentci
can observe its own canonical state AND a criome peer on independent
connections, folding each into its own slot.

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
This is the one place the two enums meet (`skills/enum-contact-points.md`);
the daemon's `criome_bridge` currently holds a private copy of the same
match, which collapses onto this shared mapping at integration.

## NOTA-fallback rendering (xlrk)

`render::RenderNota` is a blanket affordance: any object that projects
itself to NOTA becomes a labeled `RenderedObject`. A thin client paints
whatever purpose-built view it has and falls back to the typed object's
NOTA projection for everything else — the same path agents use. With the
`nota-text` feature this is the real `nota-next` projection; without it
the model still compiles and falls back to `Debug`.

## Stack discipline

Closed enums; one typed `Error` enum (`thiserror`); full English words
with no crate-name prefix; methods on data-bearing types, no free
functions; schema-emitted contract types are the nouns and behavior
attaches to them. Per `~/primary/skills/rust-discipline.md`.

## Adoption

- **mentci-egui** consumes `ObservationModel` + `RenderNota` today: it
  holds the model, feeds typed replies in as `EngineEvent`s, and renders
  every reply through the shared renderer. The shell owns no approval
  logic or per-socket state of its own.
- **mentci daemon** adopts next: its `state.rs` already builds the same
  contract types; the shared move is to let the daemon's verdict path use
  `decision::CriomeVerdict` instead of `criome_bridge`'s private
  `map_decision`, and to let any introspection surface reuse
  `RenderNota`. The daemon keeps persistence, sockets, key-unlock, and
  lifecycle; mentci-lib owns the typed model both sides share.

## Scaffold note (designer prototype)

This branch consumes a matching `signal-mentci` feature branch by local
`[patch]` because the shared model needs public READERS on projected
interface state (`pending_questions`, `panes`, `notification`,
`suggested_answer`, `context`) the operator's main does not yet expose.
Those readers attach to the schema-emitted nouns in `signal-mentci`'s
hand-written `lib.rs`. When the operator merges the readers, the patch
collapses and the plain git dependency resolves with no other change.
