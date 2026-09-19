# Rhythm Effects Documentation

This directory is the canonical documentation map for the project.

## How to navigate

Documentation is grouped by responsibility rather than by implementation chronology.

| Section | Purpose |
|---|---|
| [00-overview](00-overview/) | Philosophy, scope, vocabulary, documentation rules |
| [01-product](01-product/) | Product behavior, UX, design system, hotkeys |
| [02-architecture](02-architecture/) | System boundaries, stack, data model, time model, undo/redo |
| [03-engine](03-engine/) | Animation, rendering, audio, waveform, effects, text |
| [04-editor](04-editor/) | Timeline, viewport, inspector, assets, curve editor |
| [05-persistence-export](05-persistence-export/) | Save/load, recovery, export |
| [06-quality](06-quality/) | Performance, testing, packaging, usability |
| [07-planning](07-planning/) | Backlog, milestones, risks, decisions |
| [adr](adr/) | Architecture Decision Records |
| [ru](ru/) | Russian-language overview documents |

## Read first

1. [Project Philosophy](00-overview/PROJECT_PHILOSOPHY.md)
2. [MVP Scope](00-overview/MVP_SCOPE.md)
3. [Product Specification](01-product/PRODUCT_SPEC.md)
4. [Architecture](02-architecture/ARCHITECTURE.md)
5. [Time Model](02-architecture/TIME_MODEL.md)
6. [Timeline](04-editor/TIMELINE.md)

## Canonical-document rule

English technical documents are the canonical implementation reference.

Russian documents are allowed as product-level summaries, explanations, and planning notes. If an English technical spec and a Russian summary disagree, the discrepancy must be resolved explicitly rather than silently choosing one.

## Document status

Every substantial document should begin with one of:

- **Placeholder** — structure exists, decisions are not made.
- **Draft** — actively being designed; may change.
- **Accepted** — current project contract.
- **Implemented** — accepted and reflected in code.
- **Deprecated** — kept for history; not authoritative.

See [Documentation Rules](00-overview/DOCUMENTATION_RULES.md).
