# INTENT — mentci-lib

*What the psyche has explicitly intended for this project.
Synthesised from psyche statements and applicable workspace
constraints; not embellished. `ARCHITECTURE.md` says what
mentci-lib IS; this file says what the psyche wants it to BE.*

## Purpose

`mentci-lib` is the shared application and state-machine library for
the Mentci component. The full component shape is the standard triad:
the `mentci` daemon repository, the `signal-mentci` working signal
contract repository, and the `meta-signal-mentci` meta policy contract
repository. `mentci-lib` is not the daemon repo; it is the heavy library
reused by the daemon and by thin client shells such as `mentci-egui`,
future TUI/CLI clients, editor integrations, and status surfaces.

## Constraints

- **The contract is MVU-shaped, in five typed shapes.**
  `WorkbenchState` (the model, owned here), `WorkbenchView` (the
  pure-data per-frame snapshot the shell paints), `UserEvent`
  (a closed enum of every gesture the library accepts), `EngineEvent`
  (produced when a daemon pushes), and `Cmd` (the side-effects an
  outer runtime dispatches). The whole surface is
  `update(state, event) → state, Vec<Cmd>` and `view(state) →
  WorkbenchView`; time-travel debugging is a property of that shape.
- **The dual-daemon split is hidden from widget code.** mentci-lib
  owns both daemon connections (criome for state, nexus-daemon for
  rendering); the shell sees one unified engine surface (and the
  split is revealed only in the header view for the introspecting
  human).
- **The library holds typed records, never GUI-library types.**
  egui, iced, and Flutter widget types do not appear in this crate;
  rendering primitives live in each shell. The signal protocol
  lives in `signal` and is consumed here, not redefined; Sema state
  is owned by criome, not here.
- **First-class component triad.** Mentci's runtime home is the future
  `mentci` daemon repository. `signal-mentci` carries the ordinary
  programmable-UI wire vocabulary; `meta-signal-mentci` carries startup
  configuration and reconfiguration. The daemon repository contains the
  daemon, thin CLI, and daemon-local Signal/Nexus/SEMA runtime schemas.
- **Daemon-owned state, shared library implementation.** Mentci is a
  daemon-owned programmable UI surface: state changes in the daemon, and
  every UI client paints daemon state. `mentci-lib` owns the typed state
  machines and subscription model that the daemon and thin clients share;
  the daemon owns persistence, sockets, key-unlock flow, and long-lived
  runtime lifecycle.
- **Programmable client surface.** TUI, CLI, egui, editor integrations,
  status bars, popups, email bridges, and agentic flows are clients over
  the same Mentci daemon state. They subscribe to updates and submit
  responses rather than owning separate approval logic.
- **Criome owns the key store.** Mentci interacts heavily with the local
  criome instance for escalations and key-unlock/use. The key store is a
  criome concern; Mentci presents the human approval/key-unlock surface.

## Stack discipline

- Closed enums; typed `Error` (thiserror); full English words with
  no crate-name prefix on types. Per `primary/skills/naming.md` and
  `primary/skills/rust-discipline.md`.
- Schema knowledge that informs constructor flows is compile-time
  today (via `signal` types) and record-driven once a future schema
  catalogue lands in criome's records database.

## Scope — today, not eventually

Any "sema" reference here is today's `sema` storage kernel; any
"criome" reference is today's `criome` daemon. The eventual `Sema`
/ `Criome` are broader; this library is a realization step on
today's stack. Currently skeleton-as-design: types are pinned,
bodies are `todo!()`; it lands alongside `mentci-egui`'s first
running surface. Per `primary/ESSENCE.md` §"Today and eventually".

*Source statements live in Spirit intent records and the project's
`ARCHITECTURE.md`. Workspace-shape intent stays in
`primary/INTENT.md` and the named skills above.*
