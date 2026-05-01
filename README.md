# mentci-lib

Heavy application logic for the mentci interaction surface.

mentci is the human-facing surface for working with criome's
sema. The first incarnation is an introspection workbench
(`mentci-egui`); later family members (`mentci-iced`,
`mentci-flutter`, …) use other GUI libraries. **All of them
share this library**, which carries every piece of
application logic — workbench state, view derivation,
gesture-to-signal action flows, dual-daemon connection
management, schema-aware constructor flows, per-kind canvas
renderers, theme and layout interpretation. Each GUI shell
is thin: it renders the data this library produces and
forwards events back.

The contract is **data out, events in** — the shape that
ports cleanly across egui (immediate-mode), iced (literal
Elm-architecture), Flutter (declarative), and any future
shell.

See [`ARCHITECTURE.md`](ARCHITECTURE.md). Project-wide
context: criome/ARCHITECTURE.md.
Project intent: lore/INTENTION.md.

## Status

**Skeleton-as-design.** Type signatures pinned; bodies are
`todo!()`. Lands as the first mentci-egui is wired.

## License

[License of Non-Authority](LICENSE.md).
