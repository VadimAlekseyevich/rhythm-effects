# Rhythm Effects

A rhythm-first motion design editor.

**Core thesis:** motion-design keyframes are authored in musical time on a BPM grid, while playback/evaluation stays continuous and audio-synchronized.

## Documentation

Start here:

- [Documentation Index](docs/README.md)
- [Project Philosophy](docs/00-overview/PROJECT_PHILOSOPHY.md)
- [Accepted MVP Scope](docs/00-overview/MVP_SCOPE.md)
- [Implementation Readiness](docs/07-planning/IMPLEMENTATION_READINESS.md)
- [Русская финальная сводка](docs/ru/FINAL_MVP_ARCHITECTURE_RU.md)

The original [MVP_PLAN.md](MVP_PLAN.md) is retained as historical planning context, not the current implementation contract.

## Current status

**Architecture contract accepted; Rust implementation in progress (not MVP-ready).**

The Windows CI validates the workspace format, Clippy, unit tests, and build. The app currently exposes a GPU/egui shell, editor session, and timeline foundations, but the end-to-end audio transport, scene rendering, object editing, project save/open and measured reference-machine release gates are not complete. Several feature tasks remain in unmerged pull requests. Do not infer product readiness from a green CI run or a task checkbox.

See the [2026-10-11 Codebase Health Review](docs/06-quality/CODEBASE_HEALTH_REVIEW_2026-10-11.md) for prioritized integration risks and quality gates. Canonical specs are Accepted; implementation changes must update specs/ADRs rather than silently diverging.

## Local Windows setup

See [Windows Development Setup](docs/07-planning/WINDOWS_SETUP.md) for commands from a clean machine through `cargo run -p rhythm_app`.
