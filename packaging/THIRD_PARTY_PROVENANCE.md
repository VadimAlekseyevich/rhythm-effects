# Third-Party and Redistributed Asset Provenance

This file is the release-packaging source of truth for non-user content redistributed with Rhythm Effects. It is an implementation/compliance record, not legal advice.

## FFmpeg

- Component: FFmpeg Windows x86_64 executable.
- Version: 9.0.2 Essentials build.
- Provider: Gyan Doshi / gyan.dev (GyanD/codexffmpeg).
- Versioned artifact: ffmpeg-9.0.2-essentials_build.zip.
- Provider release tag: 9.0.2.
- Upstream FFmpeg source commit recorded by the provider: 946fcce07b.
- Archive SHA-256: 60f467265b1e312373dbcd92200c2618a74850f98d3d078e94296bb3fa2047ba.
- Provider-declared binary license: GPL-3.0-or-later.
- Required release capabilities: libx264 H.264 encoder and native aac encoder.
- Runtime relationship: separate child process; Rhythm Effects does not link FFmpeg libraries into the Rust executable.
- Machine-readable pin: packaging/ffmpeg/manifest.json.

The portable release must ship the notices/license material and corresponding source/provenance required by the selected FFmpeg build and its included GPL components. The package must not replace the pinned executable with an arbitrary PATH copy.

## Inter

Rhythm Effects redistributes two Inter variable-font files as deterministic composition fallback assets:

- crates/rhythm_engine/assets/fonts/inter/InterVariable.ttf
- crates/rhythm_engine/assets/fonts/inter/InterVariable-Italic.ttf

Provenance:

- Inter release: 4.1 (Google Fonts build).
- Upstream project: rsms/inter.
- Upstream source commit recorded by Google Fonts: 66647c0bbbe41a850d79d9c76fb13add3378940f.
- Normal Google Fonts source blob: 047c92f6e2212473dc436020afed689527076d44.
- Italic Google Fonts source blob: c177578428248f08eedcc60e292acc6f414c2167.
- License: SIL Open Font License 1.1.
- Canonical local license copy: crates/rhythm_engine/assets/fonts/inter/OFL.txt.

The font files are renamed locally only to provide stable source paths; they remain Inter font software and retain the OFL notice.

## Project-authored redistributed assets

The current repository does not redistribute third-party creative sample media. WGSL shader source embedded in rhythm_engine, generated benchmark fixtures, generated WAV test fixtures, JSON schemas/plans and UI assets created in this repository are project-authored and follow the workspace MIT OR Apache-2.0 licensing declaration unless a file states otherwise.

Generated fixture media is created deterministically at test/runtime and is not copied from external copyrighted audio, image or video content.

## User content is not redistribution

User .rhfx projects, imported images, audio files and selected system fonts are user-controlled inputs and are not part of the Rhythm Effects portable release.

## Rust dependencies

Cargo.lock is the exact version/checksum provenance record for Rust dependencies. Release review must inspect the licenses/advisories of reachable dependencies (AI-340) before publishing. This document does not replace each crate's own license terms.

## Packaging rules

The release ZIP's licenses/ directory must contain or reference:

1. this provenance document;
2. Inter's complete OFL.txt;
3. the applicable FFmpeg/GPL notices and source-provenance information for the pinned binary;
4. any additional notices identified by the final dependency review.

No release script may silently download a newer FFmpeg or font version without updating provenance, checksums and capability tests in the same change.
