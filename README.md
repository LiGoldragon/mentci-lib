# mentci-lib

The shared observability + control model for the mentci component.

mentci is the human-facing surface for criome's approval flow and the
daemon's programmable UI state. The first client is `mentci-egui`; later
family members (`mentci-iced`, `mentci-flutter`, a TUI, a status bar, …)
use other GUI libraries. **All of them — and the daemon itself — share
this model**, so canonical daemon state and painted client state cannot
drift. It carries the MVU `ObservationModel` keyed by component socket,
the approval state machine over `signal-mentci`'s vocabulary, the
edits-as-proposals flow, the DOTOS-fallback renderer, and the
closed-decision -> criome verdict mapping. Each shell is thin: it renders
the data this model produces and forwards events back.

The contract is **data out, events in** — the shape that
ports cleanly across egui (immediate-mode), iced (literal
Elm-architecture), Flutter (declarative), and any future
shell.

See `ARCHITECTURE.md`. Project-wide
context: criome/ARCHITECTURE.md.
Project intent: lore/INTENTION.md.

## Status

**Skeleton-as-design.** Type signatures pinned; bodies are
`todo!()`. Lands as the first mentci-egui is wired.

## License

[License of Non-Authority](LICENSE.md).
