# Packaging

> **Status: Accepted for MVP**

## 1. Platform

MVP release target:

- Windows 10/11;
- x86_64;
- portable ZIP distribution.

An installer, auto-update, Microsoft Store distribution, macOS, and Linux are post-MVP.

This keeps packaging simple while product value is still being proven.

## 2. Portable package

ZIP contains:

~~~text
RhythmEffects.exe
ffmpeg/...
licenses/...
required runtime assets/fonts/shaders
README/release notes as appropriate
~~~

The app must not require:

- Rust toolchain;
- Visual Studio developer environment;
- FFmpeg on PATH.

AI-331 defines the exact portable tree in packaging/portable-layout.json and
builds it with packaging/build-portable.ps1. The builder takes the Cargo
release rhythm_app.exe, publishes it as RhythmEffects.exe, verifies the pinned
FFmpeg archive filename/byte-size/SHA-256 before extraction, copies only the
pinned ffmpeg.exe into ffmpeg/, includes release README plus provenance and
license records, and then creates RhythmEffects-<version>-windows-x86_64.zip.
A missing/mismatched FFmpeg archive or missing license material fails package
creation rather than producing an incomplete ZIP. Inter font bytes and WGSL
shaders are already embedded into RhythmEffects.exe at compile time, so they
are recorded as embedded runtime assets rather than duplicated as loose
runtime files. CI parses the PowerShell script to prevent syntax regressions;
AI-332 remains the clean-machine execution proof.

## 3. Release profile

Use an explicit Cargo release profile.

AI-323 makes the MVP release profile explicit in Cargo.toml while preserving
Cargo's documented release defaults: opt-level=3, no debug info/stripping,
debug assertions and overflow checks off, LTO off, unwind panics,
incremental off, 16 codegen units and rpath off. Platform-specific
split-debuginfo remains intentionally unspecified. Because these values are
identical to the prior implicit Cargo release profile, the configuration
introduces no compiler/linker build-time delta by construction; it also
avoids claiming an unmeasured runtime gain. Absolute clean release build time
on the recorded reference machine remains the AI-315 baseline measurement.

Do not enable maximal LTO/codegen settings automatically if release-build cost becomes unreasonable without measured runtime benefit.

Record final profile in Cargo.toml and PERFORMANCE baseline.

## 4. FFmpeg

Bundle a pinned known FFmpeg Windows build compatible with the project's distribution/licensing choice.

AI-324 pins **Gyan FFmpeg 9.0.2 Essentials x86_64** from the provider's
versioned GitHub release asset. The exact archive URL, byte size, SHA-256,
archive root, source executable location, expected version banner, required
encoders and provider-declared GPL-3.0-or-later license are machine-readable
in `packaging/ffmpeg/manifest.json`. This is deliberately a release tag,
not a moving "latest" download. The Essentials build contains libx264 and is
distributed by its provider as a GPLv3 static Windows build; Rhythm Effects
uses it only across the accepted child-process boundary. Release packaging
must carry the applicable FFmpeg/x264 notices and corresponding-source
provenance; AI-330 owns that packaging documentation. This project note is
an implementation/compliance record, not legal advice.

AI-325 resolves exactly `<package>/ffmpeg/ffmpeg.exe` relative to the
absolute normalized running application executable. It never invokes a bare
`ffmpeg` name and never falls back to PATH. The version probe executes only
that resolved file with structured arguments and requires the pinned
`9.0.2-essentials_build-www.gyan.dev` banner before export may rely on it.
Missing files, unsafe/relative paths, process failures and version mismatch
are separate structured errors. Unit tests keep this runtime contract aligned
with the pinned packaging manifest.

At startup or first export, verify:

- executable exists;
- expected version/capabilities can be queried;
- H.264 encoder required by the release build is available;
- AAC encoding is available.

AI-326 extends the pinned version probe with an exact encoder-list check.
`ffmpeg -hide_banner -encoders` must contain the standalone encoder tokens
`libx264` and `aac`; similarly named encoders such as `libx264rgb` or
`aac_latm` do not satisfy the release contract. Missing H.264/AAC support,
encoder-list process failure, and version failure remain distinct structured
errors, so export can stop before opening a destination or rendering frames.

Do not silently fall back to arbitrary PATH FFmpeg in normal release behavior.

## 5. Licenses

Package includes required notices/licenses for:

- bundled FFmpeg;
- Inter;
- redistributed runtime assets;
- dependencies where required by their licenses.

Maintain provenance/version information for bundled binaries/assets.

AI-330 records the human-readable release provenance in
packaging/THIRD_PARTY_PROVENANCE.md and a machine-readable component list in
packaging/third-party-manifest.json. The record identifies the exact pinned
FFmpeg artifact/checksum/license boundary, both redistributed Inter font
files and their OFL/upstream source commits, and explicitly distinguishes
project-authored generated fixtures from user-owned imported media. Cargo.lock
remains the exact Rust package version/checksum record; AI-340 owns final
reachable-dependency/advisory review. Package assembly must copy the Inter
OFL and applicable FFmpeg/GPL/source notices rather than relying on links
that may disappear.

## 6. App data

Use appropriate Windows per-user local data directories.

Conceptual structure:

~~~text
%LOCALAPPDATA%\RhythmEffects\
    settings/
    logs/
    recovery/
    cache/
~~~

User .rhfx projects and source assets are never stored inside application install/package data unless the user explicitly chooses that path.

## 7. Cache

Cache may contain:

- waveform data;
- image thumbnails;
- nonessential derived metadata.

Deleting cache must be safe.

The application recreates it.

## 8. Logs

Release logs are bounded/rotating.

Include:

- app version;
- OS;
- GPU adapter/backend;
- audio device/backend/config where relevant;
- structured errors.

Do not log full project creative text/content unnecessarily.

## 9. Crash diagnostics

MVP minimum:

- panic/fatal diagnostic in log where possible;
- recovery system protects committed recent work;
- next launch can restore recovery.

Automatic crash upload/telemetry is not included in MVP.

## 10. Version

Application version is visible in About/log/package filename.

Project schema version remains separate.

## 11. Code signing

Code signing is not required for the first MVP portable build.

Unsigned-build warning is a known distribution limitation.

Before broad public distribution beyond early MVP/test users, evaluate signing.

## 12. Installer/update

Not in MVP.

Portable package upgrades are manual.

AppSettings/recovery live outside the package directory so replacing the ZIP/executable does not erase user state.

## 13. Clean-machine release test

Test on Windows environment without development dependencies.

Required:

- launch;
- audio playback;
- image/text;
- save/reopen;
- recovery;
- export through bundled FFmpeg.

Also test paths containing:

- spaces;
- Cyrillic/non-ASCII user/project names.

AI-333/334 add an automated Windows filesystem smoke test that creates nested
user/project-like directories containing spaces, Cyrillic and an additional
non-ASCII character, performs first Save As, parses the published .rhfx,
edits the project, then republishes over the same destination. The test runs
in the ordinary Windows CI suite, so both first-file creation and replacement
semantics are exercised on a real Windows filesystem path rather than only
checking string handling. Clean-machine package launch remains a separate
AI-332/337/338 gate.

## 14. Package failures

Test:

- bundled FFmpeg missing/quarantined;
- read-only output folder;
- low disk space;
- non-ASCII paths;
- cache/recovery directory unavailable;
- antivirus warning behavior where observable.

Errors must be actionable.

## 15. Release procedure

1. clean checkout/tag candidate;
2. pin Rust toolchain/Cargo.lock;
3. fmt/clippy/tests;
4. release build;
5. package bundled FFmpeg/assets/licenses;
6. verify package contents;
7. run clean-machine smoke test;
8. run export sync smoke;
9. record known limitations;
10. generate SHA-256 checksum;
11. tag/publish GitHub Release ZIP.

Automate only after manual procedure is stable.

## 16. Definition of Done

Packaging is MVP-ready when one portable ZIP works on clean Windows 10/11 without external FFmpeg/toolchain, required licenses are included, non-ASCII paths work, and release procedure is repeatable.
