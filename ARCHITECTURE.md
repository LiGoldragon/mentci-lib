# ARCHITECTURE — mentci-lib

Heavy application logic for the mentci interaction surface.
The library every mentci-* GUI shell consumes.

## Role in the sema-ecosystem

```
              ┌──────────────────────────────────┐
              │          mentci-lib              │
              │                                  │
              │  ALL APPLICATION LOGIC           │
              │   (workbench state machines,     │
              │    constructor flows, schema     │
              │    knowledge, theme/layout       │
              │    interpretation, dual-daemon   │
              │    connection management)        │
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
       mentci-egui mentci-iced mentci-flutter
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

mentci-lib owns both daemon connections. The shell sees a
unified "engine" surface; the dual-daemon split is hidden
from widget code (and revealed in the header view for the
introspecting human).

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
  nexus-daemon to render a payload, schedule a timer).

The `update(state, event) → state, Vec<Cmd>` and
`view(state) → WorkbenchView` functions are the entire
surface. Time-travel debugging (record the event log; replay)
is a property of the shape.

## Boundaries

Owns:

- Workbench state machines (per-pane, per-flow).
- Connection management for both daemons (criome, nexus-daemon).
- Subscription registration + push demultiplexing.
- Schema knowledge that informs constructor flows (compile-time
  today via `signal` types; record-driven once a future schema
  catalogue lands in criome's records database).
- Per-kind canvas renderers that produce kind-specific
  view-state for the shell to paint.
- Theme + layout interpretation — translates `Theme`,
  `Layout`, and related records into semantic-intent
  view-state the shell maps to its native palette.
- Constructor-flow logic for every editing verb.

Future shell-owned state (workbench history, recall last-opened
workbench, per-user layout preferences) lives in mentci-lib's own
`sema`-managed redb file — opened either inline through `sema`
directly or through a future `mentci-sema` typed-table layer (the
choice follows the same dimensionality test as criome's typed
tables: inline if the table set stays compact, separate crate if
it grows). This is library use of `sema`; no daemon, no shared
database. Per
`~/primary/reports/designer/92-sema-as-database-library-architecture-revamp.md`.

Does not own:

- The signal protocol — lives in
  signal; this
  library consumes it.
- Sema state — owned by criome.
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

**Skeleton-as-design.** Lands alongside mentci-egui's first
running surface.
