# Security and Privacy

> **Status: Accepted for MVP**
>
> Rhythm Effects is a local-first desktop editor. MVP has no account, cloud sync, collaboration service, plugins, scripting, or telemetry upload.

## 1. Privacy baseline

MVP does not intentionally transmit:

- project files;
- asset contents;
- filenames/paths;
- creative text;
- audio;
- usage analytics;
- crash reports

to any Rhythm Effects service.

There is no required network connection for normal editing/export.

## 2. External processes

Bundled FFmpeg is invoked locally.

The app passes only the file/process data required for local export.

No shell string interpolation is used for untrusted paths.

Spawn through structured process arguments.

## 3. Project/media input

Treat .rhfx and media files as untrusted local input.

Validate:

- schema versions;
- lengths/counts;
- dimensions;
- numeric ranges;
- references;
- decode failures.

Do not panic or allocate unbounded memory because a file claims absurd metadata.

## 4. Paths

Never execute content based on project path.

Normalize/resolve paths carefully without assuming UTF-8-only Windows usernames.

Relink/open dialogs are user-mediated.

## 5. No scripting/plugins

MVP deliberately has no:

- expressions;
- scripting engine;
- third-party plugins.

This removes a major arbitrary-code-execution surface.

Future plugin/scripting support requires a separate security architecture before implementation.

## 6. Cache/recovery

Cache/recovery may contain project metadata or serialized creative state locally.

Store under user-local application data.

Do not place recovery in world-readable/public temp locations when avoidable.

Cleanup never follows arbitrary project-controlled paths for recursive deletion.

## 7. Logs

Logs avoid recording full creative text/audio data.

File paths may appear where needed to diagnose I/O errors; do not upload logs automatically.

## 8. Network dependencies

If a future feature introduces network access, it requires:

- explicit documentation;
- user-visible purpose;
- privacy review;
- timeout/error handling;
- no hidden dependency for basic local editing.

## 9. Dependency hygiene

Before release:

- cargo audit/advisory review where practical;
- pin bundled FFmpeg source/version;
- track licenses/provenance;
- avoid abandoned unnecessary native dependencies.

Security advisories are evaluated for actual reachable risk, not ignored solely because code compiles.

## 10. Export process safety

- no command via shell/cmd.exe string;
- explicit executable path;
- explicit argument list;
- bounded stderr capture/logging;
- cancellation terminates only the owned export process;
- temp output path is app-created and safely published.

## 11. Definition of Done

MVP security/privacy is ready when the app remains local-first/no-telemetry, malformed project/media inputs fail safely, FFmpeg spawning avoids shell injection, recovery/cache paths are controlled, and release dependencies are reviewed.
