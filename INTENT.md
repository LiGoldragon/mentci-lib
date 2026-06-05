# INTENT — mentci-lib

*What the psyche has explicitly intended for this project.
Synthesised from psyche statements and applicable workspace
constraints; not embellished. `ARCHITECTURE.md` says what
mentci-lib IS; this file says what the psyche wants it to BE.*

## Purpose

`mentci-lib` is the heavy application-logic library for the mentci
interaction surface — the library every `mentci-*` GUI shell
(`mentci-egui`, `mentci-iced`, `mentci-flutter`) consumes. ALL
application logic lives here: workbench state machines, constructor
flows, schema knowledge, theme/layout interpretation, and the
dual-daemon connection management. The shells stay thin.

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
- **Library use of state, never a daemon or shared database.**
  Future shell-owned state lives in mentci-lib's own `sema`-managed
  redb file (inline, or a future `mentci-sema` typed-table layer
  per the same dimensionality test criome uses) — library use of
  `sema`, no daemon, no shared database.

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
