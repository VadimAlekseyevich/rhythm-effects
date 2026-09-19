# Editor UI

> **Status: Draft**
>
> This document defines the top-level editor workspace. The design must remain spacious, shallow, and predictable as functionality grows.

## 1. Primary workspace

Default layout:

~~~text
┌────────────────────────────────────────────────────────────┐
│ Top bar / project actions                                  │
├───────────────┬───────────────────────────┬────────────────┤
│ Object list   │                           │ Inspector      │
│               │         Viewport          │                │
│               │                           │                │
├───────────────┴───────────────────────────┴────────────────┤
│ Transport / BPM / Grid                                    │
├────────────────────────────────────────────────────────────┤
│ Timeline                                                   │
│ waveform + grid + object/property rows + keyframes         │
└────────────────────────────────────────────────────────────┘
~~~

The exact proportions are adjustable, but the conceptual regions stay stable.

---

## 2. UI principles

- no tab maze;
- no permanent panel for every subsystem;
- readable text;
- comfortable hit targets;
- strong keyboard path;
- predictable focus;
- timeline and viewport remain primary;
- context appears in inspector rather than spawning more windows.

---

## 3. Default panel roles

### Object list

Answers: "What exists in the composition and in what order?"

### Viewport

Answers: "What does the composition look like and what object am I manipulating?"

### Inspector

Answers: "What properties can I edit for the current context?"

### Timeline

Answers: "When does animation happen relative to music?"

### Transport/rhythm strip

Answers: "Where am I in time, what is the BPM/grid, and is playback active?"

---

## 4. Panel resizing

Panels should resize with forgiving splitters.

Rules:

- splitter hit target larger than visual line;
- sensible minimum widths/heights;
- timeline minimum height large enough for useful editing;
- inspector cannot shrink text into unreadability;
- viewport keeps a nonzero useful area.

MVP does not need a fully dockable IDE framework.

---

## 5. Panel collapsing

Optional collapse is allowed where it helps small windows.

Likely candidates:

- object list;
- inspector.

Timeline and viewport should not both disappear during normal editing.

Collapsed state can be app/workspace preference, not project data.

---

## 6. Top bar

Keep minimal.

Potential contents:

- project name/dirty indicator;
- New/Open/Save access through menu;
- Export entry;
- optional command search.

Do not place every object/effect tool permanently here.

---

## 7. Creation actions

Object creation can be exposed through:

- one Add button/menu;
- command search;
- shortcuts later.

Avoid four giant permanent buttons if they consume workspace.

The Add surface should remain shallow:

- Rectangle
- Ellipse
- Image
- Text

Effects are added contextually from inspector, not the object-creation menu.

---

## 8. Transport/rhythm strip

Keep near timeline.

Core controls:

- Play/Pause;
- current musical position;
- current time optional;
- BPM;
- offset adjustment entry;
- grid subdivision;
- snap/grid state if needed.

Avoid moving BPM controls to distant project settings because BPM alignment is frequent creative work.

---

## 9. Focus model

Focused region should be visually understandable.

Regions:

- viewport;
- timeline;
- object list;
- inspector;
- text/numeric field.

Contextual shortcuts depend on focus, but global transport/save remain available where safe.

Do not require clicking a panel before every rhythm-navigation action if a more global mapping is unambiguous.

---

## 10. Selection synchronization

Object selection is shared semantically across:

- object list;
- viewport;
- timeline.

Selecting an object in one place updates the others without unexpected scrolling unless needed.

Keyframe selection is timeline-specific but inspector/object context should identify the property/object involved.

---

## 11. Inspector context

Inspector contents follow current semantic selection.

Priority examples:

1. text editing control has local field state;
2. explicit effect selection may show effect controls;
3. object selection shows object properties/effects;
4. no selection shows useful empty state.

Avoid inspector tabs for Transform / Appearance / Effects unless later usability testing proves they are necessary.

Prefer vertical sections.

---

## 12. Timeline visibility

Timeline remains accessible during:

- object editing;
- effect editing;
- text editing;
- playback.

Do not open a separate "animation mode."

Animation is always part of the main workspace.

---

## 13. Temporary surfaces

Allowed:

- context menu;
- popover;
- command search;
- compact export surface;
- file picker.

Temporary surfaces close with Escape where safe and do not create long-lived navigation state.

---

## 14. Status feedback

Use unobtrusive feedback for:

- saving;
- waveform processing;
- missing asset;
- export progress;
- audio device failure.

Do not reserve a giant permanent status bar unless actual information density requires it.

---

## 15. Empty project

Initial editor should guide without a wizard.

Suggested viewport/timeline empty state:

- Import Audio as primary action;
- Add Object remains available;
- BPM setup becomes prominent once audio exists.

---

## 16. No selection

Inspector displays:

- project/composition summary or neutral prompt;
- no stale selected-object fields.

---

## 17. Error placement

Contextual errors appear near affected feature where practical.

Examples:

- missing asset in inspector/object row;
- invalid BPM near BPM field;
- audio device error near transport.

Global blocking failures can use a dialog.

---

## 18. UI state persistence

Potential app-level persistence:

- panel sizes;
- collapsed panels;
- last workspace layout;
- optional theme;
- recent projects.

Do not store these inside creative Project unless intentionally required.

---

## 19. Scaling

Editor respects Windows DPI scaling.

Minimum text/hit sizes are design-system constraints.

Timeline can remain visually dense, but interaction targets must stay forgiving.

---

## 20. Performance

Avoid rebuilding expensive lists or layouts for invisible content.

Examples:

- timeline virtualization;
- asset thumbnail caching;
- inspector only lays out current context;
- object list only handles visible rows where large.

No editor panel may perform blocking decode/file IO during render.

---

## 21. Growth policy

Before adding a new permanent panel, ask:

1. Can it be contextual in inspector?
2. Can it be a temporary popover?
3. Can it be an expandable timeline row?
4. Can command search expose it?
5. Does it really require persistent spatial context?

A new panel is the last option.

---

## 22. MVP usability tests

Test:

- first audio import;
- BPM alignment;
- first keyframe;
- repeated beat-step editing;
- object selection across viewport/list/timeline;
- effect editing;
- save/export;
- recovery from missing asset.

Record focus errors and unnecessary pointer travel.

---

## 23. Definition of Done

- default workspace supports entire MVP workflow;
- no routine task requires nested tabs;
- focus behavior is predictable;
- timeline/viewport remain primary;
- UI works at common DPI values;
- panel sizes and targets meet design-system review.
