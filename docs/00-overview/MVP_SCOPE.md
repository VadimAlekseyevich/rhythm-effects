# MVP Scope

> **Status: Draft**

This file is the concise English scope contract for MVP.

The detailed Russian master plan is available at [../ru/MVP_PLAN_RU.md](../ru/MVP_PLAN_RU.md).

## MVP outcome

A user can:

1. create a project;
2. import one music track;
3. configure BPM and grid offset;
4. see waveform + musical grid;
5. add basic visual objects/assets;
6. animate supported properties with grid-aligned keyframes;
7. edit easing;
8. preview in sync with audio;
9. save/reopen;
10. export a playable MP4 with audio.

## Required object types

- rectangle;
- ellipse;
- image;
- text.

## Required animated properties

- position;
- scale;
- rotation;
- opacity;
- selected basic visual/effect parameters.

## Required editor areas

- viewport;
- timeline;
- layer/object context;
- inspector;
- transport and rhythm controls.

## Required timeline behavior

- waveform;
- BPM grid;
- configurable subdivision;
- keyframes constrained to valid grid positions;
- multi-selection;
- drag;
- copy/paste;
- undo/redo;
- basic easing.

## Required output

- deterministic offline rendering;
- H.264 MP4;
- original project audio muxed into output.

## Explicit non-goals

See the Russian master plan for the expanded list. Core exclusions include 3D, particles, advanced masks, motion tracking, scripting, plugins, collaboration, cloud, full video editing, and AE compatibility.

## Definition of Done

This document will be expanded into explicit acceptance criteria before implementation begins.
