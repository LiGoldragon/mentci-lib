# Agent Bootstrap — mentci-lib

Heavy application logic for the mentci interaction surface.
Holds workbench state, view snapshots, user-event handling,
dual-daemon connection management, schema-aware constructor
flows, per-kind canvas renderers, theme/layout interpretation.

Skeleton-as-design today; bodies are `todo!()`.

Read [ARCHITECTURE.md](ARCHITECTURE.md) for boundaries and the
mentci-lib / shell-shell pattern.

For project intent: [mentci/INTENTION.md](https://github.com/LiGoldragon/mentci/blob/main/INTENTION.md).
For project-wide rules: [mentci/AGENTS.md](https://github.com/LiGoldragon/mentci/blob/main/AGENTS.md).
For project-wide architecture: [criome/ARCHITECTURE.md](https://github.com/LiGoldragon/criome/blob/main/ARCHITECTURE.md).

## Process

- Jujutsu only (`jj`).
- Push immediately after every change.
- Skeleton-as-design over prose-as-design.
- One artifact per repo.
