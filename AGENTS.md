# Agent instructions — mentci-lib

You **MUST** read AGENTS.md at `github:ligoldragon/lore` — the workspace contract.

## Repo role

The shared observability + control model for the mentci component, consumed by the daemon and every thin client. Holds the MVU `ObservationModel` keyed by component socket, the approval state machine over `signal-mentci`'s vocabulary, the edits-as-proposals flow, the NOTA-fallback renderer, and the closed-decision -> criome verdict mapping.

Re-founded on the live contracts (forensic sub-report 5); the model and its egui consumer build and test green. See `ARCHITECTURE.md`.
