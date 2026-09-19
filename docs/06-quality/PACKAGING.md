# Packaging

> **Status: Draft**
>
> This document defines how a Windows MVP build becomes a reproducible distributable application.

## 1. MVP target

- Windows 10/11;
- x86_64;
- one supported release package;
- optional portable archive for testers;
- no requirement for Microsoft Store distribution.

---

## 2. Release artifacts

Potential artifacts:

### Portable
Archive containing:
- executable;
- required bundled runtime files;
- FFmpeg if distributed;
- licenses.

Useful for early testers.

### Installer
Later MVP candidate should provide a normal Windows installation experience if packaging effort is reasonable.

Final installer technology TBD.

---

## 3. Release build

Use an explicit Cargo release profile.

Measure:

- executable size;
- startup;
- runtime performance;
- compile/link time.

Do not enable extreme LTO/size settings blindly if they dramatically slow iteration/release with little benefit.

---

## 4. FFmpeg distribution

Preferred product behavior:

normal users should not install FFmpeg manually or configure PATH.

Options:

1. bundle known FFmpeg executable/build;
2. download/install component through installer;
3. system dependency.

MVP preference: bundled controlled version, subject to licensing review and package size.

Record exact version/build provenance.

---

## 5. FFmpeg verification

At runtime/export initialization:

- resolve bundled path;
- verify executable exists;
- optionally verify version/expected functionality;
- provide clear packaging error if missing.

Do not fail mysteriously with "process not found".

---

## 6. Third-party licenses

Generate/maintain notices for:

- Rust crates as required;
- bundled fonts;
- FFmpeg/build licensing;
- icons/assets;
- other native components.

No untracked web-downloaded assets in release builds.

---

## 7. Fonts

If the app bundles editor UI fonts or composition fallback fonts:

- license permits redistribution;
- versions are pinned;
- files are included intentionally;
- fallback behavior documented.

Do not package arbitrary system fonts.

---

## 8. Application data directories

Define Windows locations for:

- settings;
- logs;
- recovery/autosave;
- caches;
- crash reports if any.

Keep these separate from installed program files.

Use appropriate user-writable OS directories.

---

## 9. Cache cleanup

Caches may include:

- waveform data;
- thumbnails;
- shader/cache metadata;
- temporary export files.

Define:

- location;
- safe deletion behavior;
- cleanup on startup or age threshold where needed.

The app must remain functional if cache is deleted.

---

## 10. Logs

Release builds should retain useful diagnostics without unbounded growth.

Need:

- rotating/bounded logs;
- app version;
- OS/build info;
- GPU adapter;
- audio backend/device info where useful;
- error chain.

Do not log personal project content unnecessarily.

---

## 11. Crash diagnostics

MVP minimum:

- panic information written to log where possible;
- recovery system protects user work;
- clear next-start recovery behavior.

Automatic crash upload is not required.

---

## 12. Versioning

Use semantic-ish application versions:

~~~text
0.1.0
0.2.0
...
~~~

Build should expose version in:

- About/help surface;
- logs;
- project metadata if useful;
- package filename.

File schema version remains separate.

---

## 13. Reproducible release procedure

Document release steps:

1. clean checkout;
2. confirm Cargo.lock;
3. run format/clippy/tests;
4. build release;
5. run packaging;
6. verify bundled FFmpeg/licenses;
7. smoke test on clean Windows environment;
8. generate checksums;
9. tag release;
10. publish artifacts.

Automate gradually.

---

## 14. Code signing

Unsigned Windows binaries may trigger warnings.

Code signing is desirable for public distribution but may be deferred for private/early MVP testing due to certificate cost/process.

Before broader release, evaluate signing strategy.

---

## 15. Installer behavior

If installer is used:

- user-writable settings remain outside install dir;
- uninstall does not silently delete user projects;
- optional cache cleanup is explicit;
- upgrades preserve settings/recovery safely.

---

## 16. Update system

Auto-update is not required for MVP.

Manual download/install is acceptable.

Do not build update infrastructure before core product value is proven.

---

## 17. Package size

Track package size as a metric.

Likely contributors:

- FFmpeg;
- fonts;
- debug symbols if accidentally included;
- duplicated native libraries.

Avoid package bloat but do not compromise essential functionality for arbitrary size goals.

---

## 18. Symbols

Keep debug symbols/artifacts for diagnosing release crashes where practical, even if not shipped inside end-user package.

---

## 19. Clean-machine test

Test release on a Windows machine/environment without:

- Rust toolchain;
- Visual Studio developer environment;
- FFmpeg on PATH;
- project source tree.

The package must contain everything required by supported distribution model.

---

## 20. Packaging failures

Test:

- read-only install location assumptions;
- missing bundled component;
- non-ASCII user path;
- spaces in path;
- low disk space;
- antivirus/quarantine scenarios where observable.

---

## 21. CI/CD

Later release workflow can:

- build Windows artifact;
- run tests;
- package portable bundle;
- attach to GitHub release.

Do not allow CI complexity to block early development.

---

## 22. Definition of Done

Packaging is MVP-ready when:

- release build runs on clean supported Windows;
- export works without manually installed FFmpeg if bundling policy says so;
- required licenses included;
- settings/log/recovery directories correct;
- package version visible;
- clean smoke test passes;
- user projects survive install/uninstall/update behavior;
- release procedure is documented and repeatable.
