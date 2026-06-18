# ARCHITECTURE — mentci-lib

Shared application and state-machine logic for the Mentci interaction
surface. The future `mentci` daemon and every thin Mentci client consume
this library.

> **Scope.** Any "sema" reference here is today's `sema` library
> (rename pending → `sema-db`); any "criome" reference is today's
> `criome` daemon. The eventual `Sema` / `Criome` are broader; this
> library is a realization step on today's stack. See
> `~/primary/ESSENCE.md` §"Today and eventually".

## Role in the sema-ecosystem

```
              ┌──────────────────────────────────┐
              │          mentci-lib              │
              │                                  │
              │  SHARED APPLICATION LOGIC        │
              │   (workbench state machines,     │
              │    constructor flows, schema     │
              │    knowledge, theme/layout       │
              │    interpretation, approval      │
              │    subscriptions)                │
              │                                  │
              │  EXPOSES                         │
              │   • WorkbenchView (data out)     │
              │   • UserEvent / EngineEvent      │
              │     (data in)                    │
              │   • Cmd (side-effects to         │
              │     dispatch externally)         │
              └──────┬───────────────────────────┘
                     │
                     │ thin contract
                     │
            ┌────────┼────────┐
            ▼        ▼        ▼
       mentci component triad
       ├─ mentci daemon repo
       ├─ signal-mentci contract
       └─ meta-signal-mentci contract
             │
             │ view/event/cmd model
             ▼
       mentci-egui mentci-tui mentci-cli
       (thin)     (thin)      (thin + FFI shim)
                     │
                     │ signal (rkyv)
                     ▼
                 ┌──────────┐    ┌──────────────┐
                 │  criome  │    │ nexus-daemon │
                 │ (state)  │    │ (rendering   │
                 │          │    │  service)    │
                 └──────────┘    └──────────────┘
```

Mentci is a first-class component triad. The `mentci` daemon repository
will hold the daemon, thin CLI, and daemon-local Signal/Nexus/SEMA
runtime schemas. `signal-mentci` will hold the ordinary programmable-UI
wire vocabulary; `meta-signal-mentci` will hold startup configuration
and reconfiguration vocabulary.

mentci-lib owns the typed state machines that the future Mentci daemon
will host. UI shells remain thin clients over daemon state: they paint
`WorkbenchView`, send `UserEvent`, and receive pushed state updates.
The daemon owns persistence, socket lifecycle, subscriptions, and the
long-lived criome connection.

## The contract — MVU shaped

The library defines four typed shapes:

- **`WorkbenchState`** — owned by mentci-lib; the model. Holds
  per-pane sub-states, the active constructor flow (if any),
  connection state, the principal whose tweaks are applied.
- **`WorkbenchView`** — derived from state; the snapshot the
  shell paints each frame (or each change). Pure data.
- **`UserEvent`** — produced by the shell when the user does
  something. Closed enum of every gesture mentci-lib accepts.
- **`EngineEvent`** — produced internally when a daemon
  pushes (subscription update, outcome arrival, diagnostic,
  nexus rendering reply, connection state change).
- **`Cmd`** — produced by `update`; describes side-effects the
  outer runtime dispatches (send a signal frame, ask
  nexus-daemon to render a payload, schedule a timer, publish
  approval-state updates to subscribed clients).

The `update(state, event) → state, Vec<Cmd>` and
`view(state) → WorkbenchView` functions are the entire
surface. Time-travel debugging (record the event log; replay)
is a property of the shape.

## Boundaries

Owns:

- Workbench state machines (per-pane, per-flow).
- Shared connection-state records used by the daemon and clients.
- Subscription registration + push demultiplexing.
- Approval-state subscription and delivery mechanics for the Mentci
  daemon's programmable UI clients.
- Schema knowledge that informs constructor flows (compile-time
  today via `signal` types; record-driven once a future schema
  catalogue lands in criome's records database).
- Per-kind canvas renderers that produce kind-specific
  view-state for the shell to paint.
- Theme + layout interpretation — translates `Theme`,
  `Layout`, and related records into semantic-intent
  view-state the shell maps to its native palette.
- Constructor-flow logic for every editing verb.

Future daemon-owned state (workbench history, recall last-opened
workbench, per-user layout preferences, approval queue, client
subscriptions) lives behind the Mentci daemon. The in-memory library
state is the shared model and test surface; durable persistence lands in
the `mentci` daemon through typed SEMA/redb storage once the daemon
exists.

Does not own:

- The Mentci wire contracts — `signal-mentci` and
  `meta-signal-mentci`; this library consumes the generated types once
  those repositories exist.
- Criome authorization/key-store state — owned by criome. Mentci has its
  own UI state; criome owns the key store and signing authority Mentci
  asks to use.
- Any rendering primitives — those live in each shell.
- Any GUI-library types — egui, iced, Flutter widgets, etc.,
  do not appear in this crate.

## Code map

```
src/
├── lib.rs           — module entry + re-exports
├── error.rs         — Error enum (typed; thiserror)
├── state.rs         — WorkbenchState (the model)
├── view.rs          — WorkbenchView (per-frame snapshot)
├── event.rs         — UserEvent + EngineEvent
├── cmd.rs           — Cmd (side-effects to dispatch)
├── connection.rs    — CriomeLink + NexusLink (dual-daemon)
├── canvas/
│   ├── mod.rs       — CanvasView dispatch + per-kind renderer
│   │                  trait
│   └── flow_graph.rs — first canvas renderer (Graph + Node +
│                       Edge → flow-graph view-state)
├── constructor.rs   — schema-aware action flows for verbs
│                      (drag-new-box, drag-wire, rename,
│                      retract, batch)
├── schema.rs        — schema knowledge (signal types →
│                      constructor-flow descriptions); compile-
│                      time today, sema-driven later
├── inspector.rs     — inspector view-state (slot detail +
│                      history)
├── diagnostics.rs   — diagnostics view-state
├── wire.rs          — wire pane view-state (signal frames)
├── theme.rs         — theme record interpretation
└── layout.rs        — layout record interpretation
```

All bodies are `todo!()` skeleton-as-design; types are pinned.

## Cross-cutting context

- Project intent:
  lore/INTENTION.md
- Project-wide architecture:
  criome/ARCHITECTURE.md
- The first design report:
  workspace/reports/111-first-mentci-ui-introspection-2026-04-29.md
- The first GUI shell:
  mentci-egui

## Status

**Running model, component triad pending.** The approval queue and
subscription state are implemented and tested in mentci-lib. The
`mentci` daemon repository, `signal-mentci`, `meta-signal-mentci`,
TUI/CLI socket protocol, and criome key-unlock integration are the next
production slices.
