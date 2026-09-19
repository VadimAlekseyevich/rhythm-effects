# Rhythm Effects Documentation

This directory is the canonical implementation and product documentation for the MVP.

## Navigation

| Section | Purpose |
|---|---|
| [00-overview](00-overview/) | Philosophy, accepted scope, vocabulary, documentation rules |
| [01-product](01-product/) | Product behavior, interaction, design system, hotkeys, flows |
| [02-architecture](02-architecture/) | State, stack, schema, time, coordinates, undo, threading, errors |
| [03-engine](03-engine/) | Animation, renderer, audio, waveform, effects, text |
| [04-editor](04-editor/) | Workspace, timeline, viewport, inspector, assets, curves |
| [05-persistence-export](05-persistence-export/) | .rhfx save/load, recovery, deterministic export |
| [06-quality](06-quality/) | Performance, testing, usability, packaging, security/privacy |
| [07-planning](07-planning/) | Readiness, milestones, backlog, risks, decision log |
| [adr](adr/) | Architecture Decision Records |
| [ru](ru/) | Russian overview/master-plan documents |

## Read this order before coding a subsystem

1. [Project Philosophy](00-overview/PROJECT_PHILOSOPHY.md)
2. [MVP Scope](00-overview/MVP_SCOPE.md)
3. [Product Specification](01-product/PRODUCT_SPEC.md)
4. [Architecture](02-architecture/ARCHITECTURE.md)
5. the canonical subsystem spec(s);
6. relevant ADR(s);
7. [Implementation Readiness](07-planning/IMPLEMENTATION_READINESS.md);
8. relevant milestone/backlog gate.

## Architecture core

The most important cross-system contracts are:

- [Domain Types](02-architecture/DOMAIN_TYPES.md)
- [Time Model](02-architecture/TIME_MODEL.md)
- [Project Model](02-architecture/PROJECT_MODEL.md)
- [Coordinate Systems](02-architecture/COORDINATE_SYSTEMS.md)
- [Commands & Undo](02-architecture/COMMANDS_UNDO.md)
- [State Ownership](02-architecture/STATE_OWNERSHIP.md)
- [Threading Model](02-architecture/THREADING_MODEL.md)
- [Error Model](02-architecture/ERROR_MODEL.md)

## Canonical language

English technical documents are authoritative.

Russian documents are overview/discussion aids. If a translation conflicts with an accepted English spec, resolve the discrepancy explicitly; do not maintain two divergent technical contracts.

## Status meanings

- **Draft** — design is not yet stable enough to be a coding contract.
- **Accepted** — current contract to implement.
- **Implemented** — code/tests conform to the accepted contract.
- **Deprecated/Superseded** — historical/non-authoritative.

At the end of the pre-code documentation phase, canonical MVP technical/product specs should be Accepted. Living indexes/registers may say "Accepted — living".

## Change rule

Do not let implementation silently become the new specification.

A semantic behavior/architecture change updates the canonical document and, when tradeoffs are meaningful, its ADR.
