# Keyboard Shortcuts

> **Status: Accepted for MVP**
>
> Shortcuts are part of the product architecture. They must remain fast, discoverable, and predictable around focus.

## 1. Input model

Default editor letter shortcuts are mapped from physical key codes when the active control is not accepting text.

This allows the same editing layout while the OS keyboard layout is English or Russian.

When a text/numeric field is actively editing, text-entry semantics take priority.

## 2. Global shortcuts

| Action | Binding |
|---|---|
| New Project | Ctrl+N |
| Open | Ctrl+O |
| Save | Ctrl+S |
| Save As | Ctrl+Shift+S |
| Undo | Ctrl+Z |
| Redo | Ctrl+Shift+Z and Ctrl+Y |
| Copy | Ctrl+C |
| Paste | Ctrl+V |
| Duplicate | Ctrl+D |
| Select All in active region | Ctrl+A |
| Delete active semantic selection | Delete |
| Play/Pause | Space |
| Command Search | Ctrl+K |
| Cancel current transient interaction | Esc |
| Rename selected object | F2 |

## 3. Timeline rhythm navigation

When timeline navigation is available and no text field owns the keys:

| Action | Binding |
|---|---|
| Playhead previous grid step | Left |
| Playhead next grid step | Right |
| Playhead previous beat | Ctrl+Left |
| Playhead next beat | Ctrl+Right |
| Playhead previous bar | Ctrl+Shift+Left |
| Playhead next bar | Ctrl+Shift+Right |
| Move selected keyframes one grid step left/right | Alt+Left / Alt+Right |
| Move selected keyframes one beat left/right | Ctrl+Alt+Left / Ctrl+Alt+Right |
| Project start | Home |
| Project end | End |
| Coarser authoring grid | [ |
| Finer authoring grid | ] |

Keyframe movement is exact integer tick arithmetic and never becomes off-grid.

## 4. Property quick access

| Property | Binding |
|---|---|
| Position | P |
| Scale | S |
| Rotation | R |
| Opacity | O |

These shortcuts reveal/focus the relevant property for the current object. They do not modify values by themselves.

## 5. Keyframe action

K is the primary property-keyframe command.

For the focused/recently revealed animatable property:

- if the property is static, K creates the first keyframe at the nearest current grid point and moves playhead there;
- if a keyframe exists at the resolved grid point, K removes it;
- if the property is animated but no key exists there, K creates one using the current evaluated value.

The UI must always reveal what property K will affect.

If there is no unambiguous property target, K does nothing and the UI indicates that a property must be focused.

## 6. Viewport navigation

| Action | Binding |
|---|---|
| Pan | Middle mouse drag |
| Zoom | Mouse wheel |
| Frame selected object(s) | F |
| Fit composition | Shift+F |

Space is never overloaded as viewport pan in MVP.

## 7. Timeline pointer navigation

- wheel: normal vertical scroll where applicable;
- Shift+wheel: horizontal timeline pan;
- Ctrl+wheel: timeline zoom around pointer;
- middle mouse drag: horizontal pan;
- ruler click/drag: continuous playhead seek.

## 8. Selection

- Ctrl+click toggles an item in selection;
- box drag from empty edit area replaces selection;
- Ctrl+box drag adds/toggles according to panel semantics;
- Shift may extend ranges only where a meaningful ordered range exists.

## 9. Easing

MVP does not dedicate global hotkeys to individual easing presets.

Ease commands are available through:

- context menu;
- command search;
- curve/easing UI.

This keeps the default shortcut vocabulary small.

## 10. Command search

Ctrl+K opens a searchable command surface.

Each entry may show:

- command name;
- current shortcut;
- context;
- disabled reason.

The command surface is a deliberate scalability mechanism so infrequent functionality does not become permanent chrome.

## 11. Shortcut remapping

User remapping is post-MVP.

Implementation must still define shortcuts through a central data/command map, not scattered raw key checks.

## 12. Repeat behavior

Held playhead-navigation keys may repeat using normal OS repeat.

Held selected-keyframe movement may repeat, but the history system should coalesce one continuous key-repeat burst into one logical history action where practical.

Delete and other destructive actions must not repeat unexpectedly.

## 13. Focus priority

Shortcut resolution priority:

1. active text/numeric editor;
2. active transient drag/edit cancel/commit semantics;
3. global safe commands;
4. focused-region commands;
5. command search fallback/discoverability.

No key event may intentionally trigger two semantic commands.

## 14. Usability acceptance

The keyboard model is accepted only if this sequence is comfortable without focus gymnastics:

1. P;
2. K;
3. Right several times;
4. change Position;
5. K/update key;
6. Ctrl+Right;
7. edit another transform property;
8. Space preview;
9. Alt+Left/Right move selected key pattern;
10. Ctrl+S.

Shortcuts may be tuned after prototype testing, but changing the command families or semantics requires updating this document.
