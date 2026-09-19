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

## 3. Release profile

Use an explicit Cargo release profile.

Do not enable maximal LTO/codegen settings automatically if release-build cost becomes unreasonable without measured runtime benefit.

Record final profile in Cargo.toml and PERFORMANCE baseline.

## 4. FFmpeg

Bundle a pinned known FFmpeg Windows build compatible with the project's distribution/licensing choice.

The app resolves the bundled executable path directly.

At startup or first export, verify:

- executable exists;
- expected version/capabilities can be queried;
- H.264 encoder required by the release build is available;
- AAC encoding is available.

Do not silently fall back to arbitrary PATH FFmpeg in normal release behavior.

## 5. Licenses

Package includes required notices/licenses for:

- bundled FFmpeg;
- Inter;
- redistributed runtime assets;
- dependencies where required by their licenses.

Maintain provenance/version information for bundled binaries/assets.

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
