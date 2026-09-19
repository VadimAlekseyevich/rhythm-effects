# Editor UI

> **Status: Accepted for MVP**

## 1. Default workspace

The default workspace has five persistent functional regions:

~~~text
┌──────────────────────────────────────────────────────────────┐
│ Top bar / project actions                                   │
├──────────────┬───────────────────────────┬───────────────────┤
│ Object list  │                           │ Inspector         │
│              │         Viewport          │                   │
│              │                           │                   │
├──────────────┴───────────────────────────┴───────────────────┤
│ Transport / rhythm strip                                    │
├──────────────────────────────────────────────────────────────┤
│ Timeline: ruler + waveform + object/property rows            │
└──────────────────────────────────────────────────────────────┘
~~~

No fully dockable IDE-style workspace in MVP.

## 2. Initial sizing

At a normal 1920×1080 desktop:

- left object panel: about 240 px;
- right inspector: about 320 px;
- timeline: about 35–40% of editor height;
- viewport receives remaining central area.

Minimum practical app window target: 1280×720.

Panels are resizable with comfortable splitter targets.

## 3. Collapse behavior

Object list and Inspector may collapse to recover viewport space.

Timeline and Viewport remain primary and are not hidden behind tabs during normal editing.

Do not add tabbed stacks of major panels.

## 4. Top bar

Keep compact.

MVP content:

- project menu/actions;
- Add/Create;
- undo/redo indicators/actions where useful;
- export entry;
- command-search entry;
- minimal status.

Do not place every tool/effect in top bar.

## 5. Transport/rhythm strip

Located immediately above timeline.

Contains:

- Play/Pause;
- current musical position;
- BPM;
- grid offset access;
- current BeatDivision;
- preview-quality control;
- optional follow-playhead toggle.

This keeps music/time controls spatially attached to timeline.

## 6. Add/Create surface

One Add action opens a shallow searchable/popover list:

- Rectangle;
- Ellipse;
- Image;
- Text.

Effects are added from Inspector, not the global Add menu.

## 7. Focus

Exactly one editor region has primary contextual keyboard focus:

- Object list;
- Viewport;
- Timeline;
- Inspector/control;
- command surface.

Focus is visible but subtle.

Text/numeric fields override physical-key editor shortcuts while editing.

## 8. Selection synchronization

Selecting an object anywhere synchronizes:

- Object list;
- Viewport;
- Inspector.

Timeline keyframe selection identifies the owning object/property without unexpectedly collapsing other user context.

Selection uses stable IDs.

## 9. Inspector context

No selection:

- neutral explanation and Add hint.

One object:

- full relevant properties.

Multiple objects:

- common transform controls and mixed-value states;
- object-type-specific controls hidden unless all selected objects share compatible type.

## 10. Timeline persistence

Timeline remains visible during ordinary object/property editing.

Curve editing expands contextually within timeline area rather than opening a separate window.

## 11. Temporary surfaces

Use shallow popovers/menus for:

- Add Object;
- Add Effect;
- grid choice;
- font picker;
- easing presets;
- command search.

Avoid multi-level submenu chains.

## 12. Dialog policy

OS/native dialogs are used for file open/save/import.

Modal app dialogs are limited to:

- unsaved-data close decision;
- unrecoverable project-open problem requiring choice;
- recovery Restore/Discard where necessary.

Routine property editing is never modal.

## 13. Status/error area

Background/recoverable status uses non-modal banner/toast area.

Examples:

- missing asset;
- audio device lost;
- autosave failure;
- export complete/fail.

Do not generate a toast for every successful ordinary edit.

## 14. Empty project

A new empty project shows a strong Import Audio action and a secondary Add Object action.

The editor itself is already usable before audio import.

## 15. Workspace persistence

Creative .rhfx does not store layout.

MVP may store panel widths/collapse state in AppSettings if inexpensive.

Failure to load settings falls back to default layout.

## 16. Scaling

UI follows logical-pixel/DPI scaling.

Do not solve small windows by shrinking typography below design-system minimums.

At constrained width, side panels collapse before text/control shrinkage.

## 17. Performance

UI layout/draw should depend on visible content.

Avoid:

- full project property traversal every frame;
- per-frame filesystem/font scans;
- rebuilding command maps;
- one widget per invisible timeline keyframe.

## 18. Growth policy

New feature exposure order:

1. existing inspector/property group;
2. context-sensitive control;
3. collapsed advanced subsection;
4. command search;
5. new persistent panel only when the task requires persistent spatial context.

Nested tab hierarchies are a design smell.

## 19. Definition of Done

Editor shell is MVP-ready when layout, collapse, focus, selection synchronization, transport placement, error surfaces, DPI behavior, and 1280×720 constrained behavior are all tested without violating design tokens.
