# MVP Scope

> **Status: Accepted for MVP**

This is the canonical concise scope boundary. Detailed subsystem behavior lives in the linked specs.

## 1. MVP outcome

A user can:

1. launch a Windows desktop editor;
2. create a 1920×1080/60 FPS project;
3. import one music track;
4. enter BPM and align grid offset to waveform;
5. choose a beat subdivision;
6. add Rectangle, Ellipse, Image, and Text objects;
7. animate supported properties with musical-grid keyframes;
8. edit easing;
9. preview in audio sync;
10. save/reopen a .rhfx project;
11. recover recent work after abnormal exit;
12. export H.264 MP4 with optional project audio.

## 2. Rhythm contract

- authored keyframes use MusicalTick;
- PPQ = 960;
- keyframe create/move resolves to the current BPM grid;
- evaluation/playhead remain continuous;
- changing visible/current grid never quantizes existing keyframes;
- BPM/offset changes retime absolute animation while preserving musical tick identity.

## 3. Objects

Required:

- Rectangle;
- Ellipse;
- Image;
- Text.

Not MVP:

- SVG;
- video object;
- nested composition;
- 3D;
- particles;
- masks/roto.

## 4. Transform animation

Every object supports:

- Position;
- Scale;
- Rotation;
- Anchor;
- Opacity.

Property animation uses per-property keyframing.

There is no global Auto-Key mode.

## 5. Object-specific animation

Rectangle:
- size;
- fill;
- corner radius if implemented from schema.

Ellipse:
- size;
- fill.

Image:
- transform animation;
- intrinsic source bounds;
- no crop/fit system.

Text:
- transform/opacity;
- animated color;
- static text/font/font-size/alignment in MVP.

## 6. Effects

Final MVP effect set:

- Blur;
- Glow;
- Tint;
- Noise;
- RGB Split.

Supported parameters may be animated as defined in EFFECTS.md.

## 7. Audio

- one primary audio track;
- WAV, MP3, FLAC, OGG/Vorbis, AAC/M4A where supported by pinned decoder;
- mono/stereo creative source semantics;
- play/pause/seek;
- silent scrubbing;
- system-default output device;
- no audio editing/mixing/effects.

## 8. Timeline

Required:

- waveform;
- bar/beat/subdivision grid;
- continuous playhead;
- object/property rows;
- keyframe selection;
- box selection;
- grid-constrained drag;
- multi-drag;
- keyboard rhythm navigation;
- copy/paste/duplicate;
- collision replacement with undo;
- easing presets/custom cubic Bezier timing;
- visible-range virtualization.

## 9. Viewport/Inspector

Required:

- pan/zoom;
- object hit selection;
- move/scale/rotation manipulation;
- inspector property editing;
- mixed multi-selection transform values;
- keyframe affordance;
- shallow contextual effects.

No fully dockable IDE workspace.

## 10. Persistence

- .rhfx;
- versioned JSON schema V1;
- typed stable IDs;
- safe temp/replace Save;
- transactional Open;
- external asset references;
- relative paths where practical;
- Relink for missing media;
- 30-second rolling recovery with current/previous generations.

## 11. Text/fonts

- cosmic-text + glyphon;
- installed/system fonts;
- bundled Inter fallback;
- Latin/Cyrillic;
- explicit multiline;
- left/center/right;
- no embedded/imported fonts.

## 12. Rendering

- wgpu/WGSL;
- Rgba16Float linear-light premultiplied working target;
- Auto/Full/Half/Quarter preview quality;
- 60 FPS Medium-fixture target at 1080p Auto on representative hardware;
- preview/export share creative rendering semantics.

## 13. Export

- deterministic offline frame timestamps;
- H.264 MP4;
- yuv420p;
- AAC audio when present;
- full composition range;
- resolution and FPS override;
- Fast/Balanced/High quality;
- progress/cancel;
- bounded readback memory.

## 14. Platform/package

- Windows 10/11 x86_64;
- portable ZIP MVP;
- bundled validated FFmpeg;
- no installer requirement;
- no auto-update;
- no code-signing requirement for first MVP.

## 15. Privacy/security

MVP is local-first:

- no account;
- no cloud;
- no collaboration;
- no analytics/telemetry upload;
- no automatic crash upload;
- no scripting/plugins.

## 16. Explicit post-MVP

- macOS/Linux;
- installer/updater;
- 3D;
- particles;
- masks/roto;
- motion tracking;
- expressions/scripting;
- plugins;
- nodes;
- multiple audio tracks/mixing;
- automatic BPM detection unless separately promoted;
- packed project/collect assets;
- imported fonts;
- SVG;
- video editing;
- parenting;
- temporal/feedback effects;
- collaboration/cloud/AI generation;
- After Effects compatibility.

## 17. MVP Definition of Done

The release is MVP only when:

- PRODUCT_SPEC acceptance scenario passes;
- all M12 milestone gates pass;
- PERFORMANCE Medium/audio gates pass;
- TESTING release matrix is executed;
- recovery has been force-crash tested;
- export sync tests pass;
- usability observation gates pass;
- a portable package works on a clean Windows machine;
- no known critical data-loss, project-corruption, persistent A/V drift, or export-corruption bug remains.

Scope changes require explicit documentation updates rather than implementation drift.
