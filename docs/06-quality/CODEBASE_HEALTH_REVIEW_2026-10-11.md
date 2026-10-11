# Codebase health review — 2026-10-11

> Scope: source-level audit of `main`, stacked editor PRs #144–147, and the CI configuration. **Not** an independent Windows runtime or performance certification. Findings distinguish implemented code, exercised tests, and user-visible features.

## Executive status

**Verdict: structurally viable early implementation; NOT feature-complete, integrationally validated, or release-ready.**

There are strong foundations: typed domain IDs and time values, `rhythm_core` isolated from GUI/GPU, audio code in `rhythm_engine`, command/history abstraction in `ProjectEditor`, range-based timeline keyframe lookup, and strict `fmt → clippy -D warnings → test → build` on Windows. These foundations should be retained.

As of this review, `main` records **149 checked / 193 unchecked out of 342** AI tasks. This is a task-ledger count, **not** a percent-complete claim. AI-158–161 are implemented or partially implemented in **unmerged** PRs #144–147, and AI-160's runtime clock integration remains incomplete. No shipped user workflow should be inferred from the checkboxes.

## Prioritized, code-verified risks

| Priority | Finding and evidence | Release impact | Required gate |
| --- | --- | --- | --- |
| P0 | `crates/rhythm_app/src/main.rs` constructs an empty Project and a GPU shell but does not wire an audio playback runtime or music-file import. `editor_ui.rs` exposes only a Start seek and playhead label. Follow Playhead receives no real audible clock events. | Rhythm-synchronized interactive playback has no end-to-end app path. | Import a real audio file, play/pause/seek, show clock-derived playhead and audio-driven follow; manual listening/drift checks. |
| P0 | `main.rs` clears the composition at setup; current `rhythm_engine::renderer` exposes a composition preview target but the app does not evaluate Project objects to draw them. `editor_ui.rs` still has placeholder Objects/Inspector panels. | Geometry/keyframe code can pass CI with nothing visible/editable in the actual viewport. | Create a Rectangle through the app; render, select, manipulate, undo/redo it; verify pixel parity and a second object. |
| P0 | No project open/save/recovery workflow is wired to the app shell. `rhythm_core/src/lib.rs` does not yet expose a persistence module. | Creative work cannot be relied upon to survive closing the app. | Save a representative project, reopen in a fresh process and compare objects, IDs, animations, tempo, asset links. |
| P1 | Four stacked editor PRs (#144→#145→#146→#147) had diverged bases and #145 became non-mergeable after parent updates. Merge commits have since been used to reconcile branch ancestry, but the number of stacked reviews is still a risk. | CI can be green in isolated heads while merge order is broken or code is not in `main`. | Require current-head green CI and mergeability against the latest parent; merge/review parent first; stop stacks at two unmerged PRs when possible. |
| P1 | `.github/workflows/ci.yml` runs Windows metadata, formatting, Clippy, unit tests and build, but no native-window launch, audio-device smoke, manual UI QA, visual references, project roundtrip, or reference-machine FPS gate. | Green CI means compiles and tests pass, **not** that workflow, GPU behavior or audio sync works. | Add targeted integration tests as features arrive; record manual smoke evidence before calling milestones done; benchmark on reference Windows hardware. |
| P1 | The 10k-key test in #147 counts headless egui draw shapes for a narrow window with one animated property. This guards against drawing every key glyph but does not prove 500-object scrolling cost or p95/p99 latency. `build_timeline_rows` still iterates objects/properties on a UI frame. | Performance can degrade before release without invalidating the existing shape-count regression. | Add medium/stress fixtures, instrument frame time and allocations, measure visible-key and row layout costs and avoid unnecessary full-project work. |
| P1 | Documentation and task checkboxes can get ahead of runtime integration (e.g. audio-clock Follow Playhead policy with no caller). `README.md` still says implementation is only about to start. | Misleading status drives premature expansion and hidden dependency debt. | Every feature PR must state **Implemented / Tested / Wired to app / Manually verified** separately; never mark the release workflow complete merely on a helper-unit test. |

## Practical merge/release policy (anti-regression gates)

1. **One feature and one acceptance surface per PR.** Pure core-only building blocks may land before their UI caller, but title and description must explicitly say they are groundwork. Avoid opportunistic refactors and new dependencies.
2. **At exact HEAD:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `cargo build --workspace` must all pass; no new commit after the last successful CI check without rechecking.
3. **Integration is separate from compilation.** For visible/audio/file behaviors, require at least one runtime-path integration test or documented Windows manual reproduction procedure. A headless pure-function test alone is insufficient.
4. **Track ancestry.** Every PR must be mergeable against its **current** base. Parent edits require syncing/revalidating child branches before merge; prefer new independent branches from `main` for pure, unrelated core tasks.
5. **Test failure modes** in addition to happy paths: extreme ticks, invalid/non-finite values, negative/zero scales, clipped/project-bound limits, undo/redo collisions, stale IDs, asset/device failures and serialization round trips.
6. **No fake completion.** Maintain distinct statuses for code authored, CI passed, app wired, reference hardware/manual verified, and end-user complete. Do not remove failing tests or increase thresholds without deriving the new bounds.
7. **Do not release against a task counter.** Progress is a user-visible vertical slice plus recovery, media sync, export and measured performance; checklist completion is advisory.

## Integration milestones to prioritize

- **Gate A — minimum editing loop:** construct Rectangle in app → evaluate and visibly render at a given playhead → hit-test/select → inspector edit → undo/redo → save/reopen; one real integration test and Windows screen recording/manual checklist.
- **Gate B — rhythm loop:** choose audio → decode asynchronously → waveform → Play/Pause/Seek → authoritative CPAL audible clock → keyframe evaluation → Follow Playhead; deterministic fake-clock tests plus hardware tests.
- **Gate C — content/export loop:** representative Rectangle/Ellipse/Text/Image with basic effects → project roundtrip → full-resolution export → inspect generated output and audio sync.
- **Gate D — release quality:** empty/basic/medium/heavy fixtures, memory boundedness, p95/p99 times from `PERFORMANCE.md`, Windows DPI/GPU matrix, device-loss/recovery and packaging.

## Status discipline

This audit is a **snapshot**, not a guarantee that unexamined modules are free of bugs. There was no reference-machine GUI run or 30-minute hardware/audio stress session. Re-run this audit after Gates A and B, and update the table with measured outcomes rather than claims.
