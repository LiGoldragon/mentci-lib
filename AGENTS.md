# Agent instructions — mentci-lib

You **MUST** read AGENTS.md at `github:ligoldragon/lore` — the workspace contract.

## Repo role

The client-side observability + control model for thin mentci clients. Holds the MVU `ObservationModel` keyed by component socket, the approval state machine over `signal-mentci`'s vocabulary, the edits-as-proposals flow, the DOTOS-fallback renderer, and the typed closed-decision -> criome verdict mapping that the daemon can reuse while keeping criome socket access daemon-owned.

Re-founded on the live contracts (forensic sub-report 5); the model and its egui consumer build and test green. See `ARCHITECTURE.md`.

## Protos estate status

Stack: correct-new destination
Status: active component, current checkout legacy-wired
This checkout is not proof of correct-new adoption.
