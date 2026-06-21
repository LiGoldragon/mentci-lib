# INTENT — mentci-lib

*What the psyche has explicitly intended for this project.
Synthesised from psyche statements and applicable workspace
constraints; not embellished. `ARCHITECTURE.md` says what
mentci-lib IS; this file says what the psyche wants it to BE.*

## Purpose

`mentci-lib` is the client-side application and state-machine library
for thin Mentci clients. The full component shape is the standard triad:
the `mentci` daemon repository, the `signal-mentci` working signal
contract repository, and the `meta-signal-mentci` meta policy contract
repository. `mentci-lib` is not the daemon repo; it is the heavy library
reused by thin client shells such as `mentci-egui`, the mentci CLI,
future TUI clients, editor integrations, and status surfaces.

## Constraints

- **The contract is MVU-shaped, in five typed shapes.**
  `WorkbenchState` (the model, owned here), `WorkbenchView` (the
  pure-data per-frame snapshot the shell paints), `UserEvent`
  (a closed enum of every gesture the library accepts), `EngineEvent`
  (produced when a daemon pushes), and `Cmd` (the side-effects an
  outer runtime dispatches). The whole surface is
  `update(state, event) → state, Vec<Cmd>` and `view(state) →
  WorkbenchView`; time-travel debugging is a property of that shape.
- **One model keyed by component socket; the multi-socket split is
  hidden from widget code.** mentci-lib owns observations per
  `ComponentSocketKind` (its own canonical state, the meta surface, a
  criome peer); the shell sees one unified `ObservationModel` surface,
  and the per-socket split is revealed only in the header view for the
  introspecting human. *(The earlier framing of this as a fixed criome +
  "nexus-daemon" dual-daemon pair is retired: forensic sub-report 5 found
  the nexus daemon was never built and the daemon speaks `signal-frame`
  `StreamingFrame`, not a graph-signal transport. The durable intent —
  hide the connection split behind one model — holds; the mechanism is
  now component-socket-keyed observations over the live contracts.)*
- **The library holds typed records, never GUI-library types.**
  egui, iced, and Flutter widget types do not appear in this crate;
  rendering primitives live in each shell. The signal vocabulary lives in
  `signal-mentci` / `meta-signal-mentci` and is consumed here, not
  redefined; canonical state and effects are owned by the mentci daemon,
  not here.
- **First-class component triad.** Mentci's runtime home is the future
  `mentci` daemon repository. `signal-mentci` carries the ordinary
  programmable-UI wire vocabulary; `meta-signal-mentci` carries startup
  configuration and reconfiguration. The daemon repository contains the
  daemon, thin CLI, and daemon-local Signal/Nexus/SEMA runtime schemas.
- **Daemon-owned state, client library implementation.** Mentci is a
  daemon-owned programmable UI surface: state changes in the daemon, and
  every UI client paints daemon state. `mentci-lib` owns the client-side
  mirrored model, local approval cursor, NOTA fallback renderer, and
  side-effect descriptions clients dispatch to the daemon; the daemon
  owns persistence, sockets, criome bridge, key-unlock flow, and
  long-lived runtime lifecycle.
- **Programmable client surface.** TUI, CLI, egui, editor integrations,
  status bars, popups, email bridges, and agentic flows are clients over
  the same Mentci daemon state. They subscribe to updates and submit
  responses rather than owning separate approval logic.
- **Closed verdicts; edits are proposals.** A Mentci verdict is a closed
  choice: approve the suggested answer, reject, or defer. If the psyche
  edits the suggested answer, that edit becomes a new typed proposal
  object submitted through the normal criome authorization path; it is not
  carried as an open answer inside the verdict.
- **Criome owns the key store.** Mentci interacts heavily with the local
  criome instance for escalations and key-unlock/use. The key store is a
  criome concern; Mentci presents the human approval/key-unlock surface.
  Thin clients do not open criome sockets. They answer through the mentci
  daemon, and the daemon routes criome-sourced answers by the parked
  `AuthorizationRequestSlot` when it has write authority.

## Stack discipline

- Closed enums; typed `Error` (thiserror); full English words with
  no crate-name prefix on types. Per `primary/skills/naming.md` and
  `primary/skills/rust-discipline.md`.
- The wire vocabulary is consumed from the live `signal-mentci` /
  `meta-signal-mentci` contracts, never redefined. The criome verdict
  conversion uses the live criome contract types as a typed contact point;
  criome socket access is still a daemon concern.

## Scope — today, not eventually

Any "sema" reference here is today's `sema` storage kernel; any
"criome" reference is today's `criome` daemon. The eventual `Sema`
/ `Criome` are broader; this library is a realization step on
today's stack. Re-founded on the live contracts (forensic sub-report
5): the MVU `ObservationModel`, the approval state machine, the
NOTA-fallback renderer, and the closed-decision -> criome verdict
mapping are implemented and green, with mentci-egui and the mentci CLI
consuming the model. Clients submit `AnswerQuestion` to the mentci
daemon; the daemon owns any criome verdict side effect.
Per `primary/ESSENCE.md` §"Today and eventually".

*Source statements live in Spirit intent records and the project's
`ARCHITECTURE.md`. Workspace-shape intent stays in
`primary/INTENT.md` and the named skills above.*
