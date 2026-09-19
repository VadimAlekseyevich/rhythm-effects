# Decision Log

> **Status: Draft**
>
> Compact index of project decisions. Detailed rationale belongs in canonical specs or ADRs.

## Status values

- Proposed
- Accepted
- Superseded
- Deferred

## Decisions

| ID | Date | Decision | Status | Canonical source | Notes |
|---|---|---|---|---|---|
| D-001 | 2026-09-19 | Rhythm Effects is rhythm-first motion design, not a reduced general-purpose AE clone | Accepted | ../00-overview/PROJECT_PHILOSOPHY.md | Product-level constraint |
| D-002 | 2026-09-19 | Authored keyframes live in integer musical time/grid positions | Accepted | ../00-overview/PROJECT_PHILOSOPHY.md, ../02-architecture/TIME_MODEL.md | Continuous evaluation remains allowed |
| D-003 | 2026-09-19 | UI comfort and shallow navigation are first-class requirements | Accepted | ../00-overview/PROJECT_PHILOSOPHY.md | New permanent panels are last resort |
| D-004 | 2026-09-19 | Runtime performance and build speed are first-class constraints | Accepted | ../00-overview/PROJECT_PHILOSOPHY.md, ../06-quality/PERFORMANCE.md | Measure both loops |
| D-005 | 2026-09-19 | Initial implementation language is Rust | Draft/Accepted pending ADR cleanup | ../02-architecture/TECH_STACK.md | Treat as working stack decision |
| D-006 | 2026-09-19 | Renderer uses wgpu/WGSL | Draft/Accepted pending ADR cleanup | ../02-architecture/TECH_STACK.md | |
| D-007 | 2026-09-19 | Editor UI uses egui with direct winit/wgpu integration | Draft/Accepted pending ADR cleanup | ../02-architecture/TECH_STACK.md | |
| D-008 | 2026-09-19 | Initial workspace uses three main crates: core, engine, app | Draft | ../02-architecture/ARCHITECTURE.md | Split only with evidence |
| D-009 | 2026-09-19 | Audio playback clock is authoritative during playback | Draft | ../02-architecture/ARCHITECTURE.md, ../02-architecture/TIME_MODEL.md | |
| D-010 | 2026-09-19 | Proposed PPQ is 960 | Draft | ../02-architecture/TIME_MODEL.md | Accept before schema freeze |
| D-011 | 2026-09-19 | Preview and export share animation evaluation semantics | Draft | ../02-architecture/ARCHITECTURE.md, ../05-persistence-export/EXPORT.md | |
| D-012 | 2026-09-19 | Project saves use schema versioning and safe temp/replace writes | Draft | ../05-persistence-export/SERIALIZATION.md | |
| D-013 | 2026-09-19 | Autosave is recovery state, not silent canonical save replacement | Draft | ../05-persistence-export/PROJECT_RECOVERY.md | |
| D-014 | 2026-09-19 | H.264 MP4 via FFmpeg is MVP export target | Draft | ../05-persistence-export/EXPORT.md | |
| D-015 | 2026-09-19 | No general async runtime by default | Draft | ../02-architecture/ARCHITECTURE.md, ../02-architecture/TECH_STACK.md | Explicit workers first |
| D-016 | 2026-09-19 | Undo stores semantic affected state, not whole-project snapshots per edit | Draft | ../02-architecture/COMMANDS_UNDO.md | |

## Maintenance rule

When a decision changes:

1. add/update ADR if rationale is material;
2. mark previous decision Superseded;
3. link replacement;
4. update affected canonical specs;
5. do not erase historical reasoning.
