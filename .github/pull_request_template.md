## User-visible outcome / task

Describe the behavior and link the accepted subsystem spec. Separate a pure groundwork API from a feature actually reachable in the app.

## Validation

- [ ] Exact HEAD passes Windows CI: cargo fmt, Clippy -D warnings, workspace tests, workspace build.
- [ ] Existing tests remain enabled; new edge/failure cases are covered.
- [ ] PR is mergeable against its **current** base, including any parent updates.
- [ ] No unreviewed dependency changes, broad unrelated refactors, or unintended project mutation paths.
- [ ] Documentation and task ledger accurately distinguish implementation from integration.

## Integration status

- [ ] Implemented in code
- [ ] Unit/integration tests passed
- [ ] Reachable through the app UI/runtime (or explicitly marked **groundwork only**)
- [ ] Manually validated on Windows for GUI, audio, file I/O or GPU changes (or **not verified**)

**Known gaps and blockers:** State what is deliberately not wired yet and which next task owns that integration. Do not claim end-to-end completion based solely on compilation.
