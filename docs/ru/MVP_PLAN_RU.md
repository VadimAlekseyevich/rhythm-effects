# Rhythm Effects — MVP Master Plan

> Статус: общий master-plan.
> Цель: описать полный маршрут от пустого репозитория до первого законченного MVP.
> Каждый крупный раздел позже превращается в отдельную спецификацию, implementation plan и набор задач.

---

## 0. Видение продукта

Rhythm Effects — легковесный desktop-редактор motion design, в котором музыка и ритмическая сетка являются основой таймлайна.

Главный принцип:

**Не motion editor, в который добавили BPM, а музыкальный sequencer, события которого управляют графикой.**

Основной пользовательский цикл:

~~~text
услышал момент в музыке
→ нашёл его на timeline
→ поставил keyframe
→ изменил визуальное свойство
→ сразу увидел результат
→ повторил
~~~

Ключевые качества продукта:

- быстрый запуск;
- низкая задержка интерфейса;
- точная синхронизация с музыкой;
- быстрый keyboard-first workflow;
- предсказуемый timeline;
- GPU preview;
- безопасное сохранение проекта;
- стабильные undo/redo;
- быстрый экспорт.

---

# 1. Зафиксировать границы MVP

## 1.1. Главный сценарий MVP

MVP считается функционально состоявшимся, когда новый пользователь способен пройти весь путь:

~~~text
Launch
→ New Project
→ Import audio
→ Set BPM / offset
→ See waveform + beat grid
→ Add rectangle / circle / image / text
→ Animate position / scale / rotation / opacity
→ Snap keyframes to rhythm
→ Change easing
→ Preview animation with audio
→ Save project
→ Reopen project
→ Export MP4
~~~

Все архитектурные решения должны в первую очередь обслуживать этот сценарий.

## 1.2. Целевая платформа

Для MVP:

- Windows 10/11;
- x86_64;
- desktop only;
- без обязательной поддержки macOS/Linux;
- архитектура по возможности остаётся переносимой.

## 1.3. Предварительный стек

Основной кандидат:

- Rust;
- winit — window/event loop;
- wgpu — GPU renderer;
- egui — editor UI;
- CPAL — audio output;
- Symphonia — audio decoding;
- rustfft или аналог — audio analysis;
- serde — serialization;
- FFmpeg — final encoding/muxing;
- WGSL — GPU shaders.

Окончательно стек фиксируется отдельным техническим документом.

## 1.4. Что входит в MVP

### Project
- New Project;
- Open Project;
- Save;
- Save As;
- dirty-state;
- минимальный autosave/recovery.

### Audio
- один основной audio track;
- import;
- playback;
- seek;
- waveform;
- BPM;
- BPM offset;
- beat grid;
- subdivision grid;
- snapping.

### Visual objects
- Rectangle;
- Circle/Ellipse;
- Image;
- Text;
- SVG — только если не создаёт заметного scope growth.

### Transform
- Position X/Y;
- Scale X/Y;
- Rotation;
- Opacity;
- Anchor/Pivot.

### Animation
- keyframes;
- add/delete/move;
- multi-select;
- copy/paste;
- Hold;
- Linear;
- preset easing;
- cubic Bezier;
- минимальный curve editor.

### Editor
- viewport;
- timeline;
- layer/object list;
- inspector;
- transport controls;
- BPM/snap controls.

### Effects
Кандидаты:
- Blur;
- Glow;
- Tint/Color adjustment;
- Noise;
- RGB Split.

Количество эффектов можно сократить ради завершения основного workflow.

### Export
- H.264 MP4;
- resolution;
- FPS;
- audio;
- deterministic offline rendering;
- progress;
- cancel.

---

# 2. Явно определить NON-GOALS MVP

До MVP намеренно не делать, если функция не нужна для проверки основной продуктовой гипотезы:

- 3D;
- cameras;
- lights;
- particles;
- advanced masks;
- rotoscoping;
- motion tracking;
- полноценный video editor;
- multi-track audio mixing;
- VST;
- scripting;
- expressions;
- plugin API;
- cloud;
- collaboration;
- accounts;
- marketplace;
- mobile;
- web editor;
- After Effects import;
- Illustrator-level vector editing;
- advanced typography;
- HDR;
- сложные precomps;
- procedural node graph;
- AI features.

Фильтр для любой новой функции:

**Без неё невозможно проверить основную идею продукта?**

Если нет — после MVP.

---

# 3. Product requirements и UX foundation

До production UI зафиксировать модель взаимодействия.

## 3.1. Основной layout

~~~text
┌──────────────────────────────────────────────────────┐
│ Menu / Toolbar                                       │
├──────────────┬───────────────────────┬───────────────┤
│ Layer/Object │                       │ Inspector     │
│ list         │      Viewport         │ / Properties  │
│              │                       │               │
├──────────────┴───────────────────────┴───────────────┤
│ Transport / Time / BPM / Snap                        │
├──────────────────────────────────────────────────────┤
│ Timeline + waveform + beat grid + keyframes          │
└──────────────────────────────────────────────────────┘
~~~

## 3.2. Interaction model

Отдельно определить:

- select;
- multi-select;
- box select;
- drag;
- duplicate;
- delete;
- copy/paste;
- undo/redo;
- viewport zoom/pan;
- timeline zoom/pan;
- snapping;
- temporary snap override;
- keyboard navigation;
- property editing;
- add/remove keyframe;
- jump between keyframes;
- jump between beats;
- play/pause;
- frame/beat step.

## 3.3. Hotkey philosophy

Редактор проектировать keyboard-first.

Нужно определить:

- глобальные shortcuts;
- panel-specific shortcuts;
- conflict resolution;
- focus rules;
- возможность remapping позже.

## 3.4. UX prototype

До тяжёлой интеграции создать mock editor:

- panels;
- mock viewport;
- mock waveform;
- playhead;
- layers;
- mock keyframes;
- selection/drag;
- timeline zoom.

Цель — проверить workflow до production engine.

---

# 4. Repository и engineering foundation

## 4.1. Предварительная структура

~~~text
rhythm-effects/
├─ Cargo.toml
├─ crates/
│  ├─ app/
│  ├─ core/
│  ├─ animation/
│  ├─ audio/
│  ├─ renderer/
│  ├─ editor/
│  ├─ project/
│  └─ export/
├─ assets/
├─ shaders/
├─ docs/
├─ tests/
└─ tools/
~~~

На старте не дробить код на crates без необходимости.

## 4.2. Tooling

Настроить:

- rustfmt;
- clippy;
- tests;
- debug/release profiles;
- logging;
- error reporting;
- profiling hooks.

## 4.3. CI

Минимум:

- cargo fmt check;
- cargo clippy;
- cargo test;
- Windows build.

Позже:

- packaged artifact;
- release workflow.

## 4.4. Error model

Разделить:

- recoverable error;
- user-facing error;
- fatal error;
- internal diagnostic.

## 4.5. Performance instrumentation

Предусмотреть измерение:

- total frame time;
- UI time;
- render time;
- animation evaluation time;
- audio callback health;
- visible objects;
- visible keyframes;
- memory;
- dropped preview frames.

---

# 5. Core data model

UI не должен быть источником истины.

Нужна самостоятельная модель проекта, которую можно:

- сериализовать;
- тестировать;
- проигрывать;
- рендерить;
- изменять командами;
- использовать без UI.

## 5.1. Project model

~~~text
Project
├─ metadata
├─ settings
├─ tempo_map
├─ audio
├─ assets
└─ composition
   └─ layers
      └─ objects
~~~

## 5.2. Composition settings

Минимум:

- width;
- height;
- FPS;
- duration;
- background.

## 5.3. Stable IDs

Стабильный ID нужен для:

- layer;
- object;
- asset;
- effect;
- keyframe.

ID не зависит от индекса в массиве.

## 5.4. Layer/object model

Определить:

- order;
- visibility;
- lock;
- name;
- transform;
- effects;
- parenting — отдельно решить, входит ли в MVP.

## 5.5. Assets

Registry для:

- audio;
- images;
- fonts;
- optional SVG.

Отдельно решить external references vs copied/packed assets.

---

# 6. Time model — фундамент продукта

Музыкальное время должно быть частью core, а не визуальной надстройкой timeline.

## 6.1. Разделить четыре пространства времени

- sample position;
- absolute seconds;
- video frame;
- musical time.

~~~text
audio sample position
→ seconds
→ TempoMap
→ beat/tick
~~~

## 6.2. MusicalTime

Использовать integer-based musical coordinate:

- bar/beat/tick;
- либо единый integer tick.

Не хранить музыкальную координату только во float.

## 6.3. PPQ

Выбрать ticks per quarter note:

- 480;
- 960;
- 1920.

Решение обосновать отдельно.

## 6.4. TempoMap

Даже если MVP UI поддерживает один BPM, core желательно не закрывать для:

- tempo changes;
- time signature changes.

Но tempo editor не нужен до MVP.

## 6.5. Conversion API

Единый протестированный API:

- samples ↔ seconds;
- seconds ↔ ticks;
- frames ↔ seconds;
- tick ↔ nearest subdivision.

## 6.6. Snapping

Минимум:

- 1/1;
- 1/2;
- 1/4;
- 1/8;
- 1/16;
- 1/32.

Желательно:

- triplets;
- 1/64.

Определить snap для:

- playhead;
- keyframes;
- keyboard step;
- drag;
- temporary bypass modifier.

---

# 7. Animation engine

Animation engine полностью отделён от timeline UI.

## 7.1. Animated property

~~~text
Animated<T>
├─ default value
└─ keyframes[]
~~~

## 7.2. Keyframe

Минимум:

- stable ID;
- musical/absolute time representation;
- value;
- interpolation;
- curve data.

## 7.3. Supported value types

- float;
- Vec2;
- Color;
- discrete value при необходимости.

## 7.4. Evaluation

Главная операция:

~~~text
evaluate(property, time) -> value
~~~

Требования:

- deterministic;
- independent from UI;
- одинаковая логика preview/export;
- unit tested.

## 7.5. Interpolation

Порядок:

1. Hold;
2. Linear;
3. easing presets;
4. Cubic Bezier.

## 7.6. Keyframe operations

- insert;
- delete;
- move;
- multi-move;
- duplicate;
- copy/paste;
- change value;
- change interpolation.

---

# 8. Command system + Undo/Redo

Undo/redo проектировать до полноценного editor.

Значимые изменения проекта желательно проводить командами.

Примеры:

- AddObject;
- DeleteObject;
- SetProperty;
- AddKeyframe;
- MoveKeyframes;
- DeleteKeyframes;
- ImportAsset;
- RenameLayer;
- AddEffect.

Требования:

- undo;
- redo;
- command grouping;
- drag = один undo step;
- sensible typing grouping;
- dirty-state integration;
- testability without UI.

---

# 9. Renderer foundation

Один rendering/evaluation path должен обслуживать preview и export.

## 9.1. GPU bootstrap

- adapter;
- device;
- queue;
- surface;
- resize;
- DPI;
- device lost strategy.

## 9.2. Coordinate systems

Зафиксировать:

- composition coordinates;
- viewport coordinates;
- local/world transform;
- pixel/DPI behavior.

## 9.3. Render primitives

Последовательно:

1. background;
2. rectangle;
3. ellipse;
4. image texture;
5. text;
6. alpha compositing.

## 9.4. Transform

Renderer получает вычисленные:

- position;
- scale;
- rotation;
- anchor;
- opacity.

## 9.5. Pass architecture

Минимум:

- main scene;
- offscreen target;
- effect passes;
- final composite.

Не строить слишком общий render graph раньше времени.

## 9.6. Performance target

Предварительно:

- 60 FPS — обязательная база на простых/средних сценах;
- 120+ FPS — желаемо на простых сценах;
- минимизировать CPU↔GPU sync.

---

# 10. Viewport editor

## 10.1. Navigation

- zoom;
- pan;
- fit composition;
- reset view.

## 10.2. Selection

- click select;
- deselect;
- multi-select;
- box select — желательно.

## 10.3. Gizmos

Минимум:

- position;
- scale;
- rotation.

Допустим один simplified transform tool на ранней стадии.

## 10.4. Helpers

- composition boundary;
- center;
- selection outline;
- anchor point;
- guides позже.

## 10.5. Integration with commands

Viewport не должен напрямую мутировать модель.

~~~text
mouse drag
→ transient preview
→ command commit
→ undo stack
~~~

---

# 11. Audio engine

Audio — источник playback clock.

## 11.1. Import/decode

Минимум:

- WAV;
- MP3;
- AAC/M4A;
- FLAC;
- OGG при низкой дополнительной стоимости.

## 11.2. Internal representation

Определить:

- streaming vs decoded buffer;
- sample format;
- channels;
- sample rate conversion;
- caching.

## 11.3. Playback

- play;
- pause;
- stop;
- seek;
- volume;
- loop — optional.

## 11.4. Clock

Playback position связывается с реальным audio sample position.

UI frame delta не является master clock.

## 11.5. Scrubbing

Для MVP допустимо:

- silent drag;
- seek on release/drag.

Audible scrubbing можно добавить позже.

## 11.6. Drift testing

Проверить:

- 1 minute;
- 5 minutes;
- 10+ minutes;
- repeated seek/play;
- different sample rates;
- editor under low FPS.

---

# 12. Waveform pipeline

Waveform не строить из raw PCM каждый кадр.

## 12.1. Preprocess

~~~text
audio
→ decode
→ peak extraction
→ multi-resolution waveform cache
~~~

## 12.2. Multi-resolution data

Timeline запрашивает только подходящий zoom level и видимый диапазон.

## 12.3. Rendering

- clipping to visible range;
- batched rendering;
- no full-song work every frame.

## 12.4. Cache lifecycle

Определить:

- in-memory cache;
- project-side cache;
- rebuild;
- invalidation.

---

# 13. BPM, offset и rhythm grid

Это одна из главных продуктовых features.

## 13.1. BPM

MVP:

- manual BPM;
- decimal BPM;
- offset;
- default 4/4.

Automatic BPM detection не обязательна.

## 13.2. Beat grid

Разные visual levels:

- bar;
- beat;
- subdivision.

Grid density меняется с zoom.

## 13.3. Rhythm navigation

Actions:

- next/previous beat;
- next/previous subdivision;
- jump N beats;
- snap playhead.

## 13.4. BPM alignment UX

Пользователь быстро:

- вводит BPM;
- двигает offset;
- совмещает сетку с waveform;
- делает fine nudge;
- tap tempo — кандидат.

---

# 14. Timeline — центральный editor

Timeline, вероятно, будет самым сложным UI-компонентом MVP.

## 14.1. Coordinate API

Единые функции:

- time → x;
- x → time;
- tick → x;
- x → nearest tick.

## 14.2. Navigation

- pan;
- horizontal scroll;
- cursor-centered zoom;
- jump to playhead;
- optional follow playback.

## 14.3. Playhead

- click;
- drag;
- snap;
- exact position display.

## 14.4. Rows

Object/layer = row.

Animated properties:

- expand;
- collapse;
- show keyframes.

## 14.5. Keyframe visuals

- selected/unselected;
- large enough hit target;
- overlapping keyframe strategy;
- interpolation marker optional.

## 14.6. Editing

- add;
- delete;
- drag;
- multi-drag;
- duplicate;
- box select;
- copy/paste;
- snap;
- keyboard nudge.

## 14.7. Virtualization

Обрабатывать только:

- visible rows;
- visible time range;
- visible keyframes.

Не допускать стоимости каждый кадр от полного размера проекта.

## 14.8. Transport

- play/pause;
- start;
- current time;
- current beat;
- BPM;
- snap subdivision.

---

# 15. Inspector / properties

## 15.1. Common transform

- Position;
- Scale;
- Rotation;
- Anchor;
- Opacity.

## 15.2. Animation controls

У animatable property:

- value;
- add/remove keyframe;
- animated state;
- current keyframe state.

## 15.3. Object-specific

Rectangle:
- width;
- height;
- fill;
- optional radius.

Ellipse:
- width;
- height;
- fill.

Image:
- source;
- dimensions;
- fit mode optional.

Text:
- text;
- font;
- size;
- alignment;
- color.

---

# 16. Assets/import

## 16.1. Import UX

- file picker;
- drag & drop desirable;
- validation;
- useful errors.

## 16.2. Asset browser

Минимум:

- filename;
- type;
- thumbnail/preview.

## 16.3. Images

Минимум:

- PNG;
- JPEG;
- WebP desirable.

## 16.4. GPU texture lifecycle

- upload;
- cache;
- unload;
- resizing;
- memory tracking.

---

# 17. Text rendering

Text — отдельный технический риск.

MVP intentionally limits typography.

Минимум:

- multi-line;
- font selection;
- font size;
- color;
- alignment.

Не обязательно:

- per-character animation;
- text-on-path;
- variable-font controls;
- advanced typography.

Проверить:

- Unicode;
- Cyrillic;
- shaping;
- font fallback;
- emoji policy.

---

# 18. Effects pipeline

## 18.1. Concept

~~~text
object
→ base render
→ effect 1
→ effect 2
→ composite
~~~

## 18.2. MVP candidates

- Blur;
- Glow;
- Tint/Color;
- Noise;
- RGB Split.

## 18.3. Animated effect parameters

Основные параметры effect по возможности используют тот же Animated<T>.

Так любой effect можно анимировать по ритму без отдельной системы.

## 18.4. GPU resource strategy

Определить:

- shader loading;
- pipeline cache;
- intermediate textures;
- texture pool;
- bind group strategy.

---

# 19. Easing / curve editor

## 19.1. First stage

Presets:

- ease in;
- ease out;
- ease in-out.

## 19.2. Second stage

Cubic Bezier handles.

## 19.3. Minimal graph UI

- selected property/keyframes;
- handles;
- reset;
- presets.

Полный AE-style graph editor не является целью MVP.

---

# 20. Project serialization

Формат проекта versioned с первого дня.

## 20.1. Requirements

- version field;
- migrations;
- stable IDs;
- asset paths;
- deterministic enough;
- debuggable format желательно.

## 20.2. Candidate format

- JSON;
- RON;
- другой serde-based format.

Решение документируется отдельно.

## 20.3. Safe save

- temp file;
- atomic replace where possible;
- backup;
- clear error.

## 20.4. Autosave/recovery

- periodic autosave;
- detect recovery state;
- restore/discard.

---

# 21. Export pipeline

Export — offline deterministic rendering, не screen capture.

## 21.1. Frame evaluation

~~~text
frame index
→ exact timestamp
→ animation evaluation
→ GPU render
→ encoded frame
~~~

## 21.2. Audio

Original audio muxed into output.

## 21.3. MVP export settings

- path;
- width/height;
- FPS;
- H.264;
- quality/bitrate preset.

## 21.4. Progress

- current frame;
- total;
- percentage;
- cancel.

## 21.5. Preview/export parity

Контрольные timestamps должны давать одинаковое визуальное состояние.

---

# 22. Performance pass

## 22.1. Benchmark projects

Small:
- 10 layers;
- 100 keyframes.

Medium:
- 100 layers;
- 1,000 keyframes.

Stress:
- 500+ layers;
- 10,000+ keyframes.

Stress нужен для поиска плохой asymptotic complexity, а не как обязательная гарантия MVP.

## 22.2. Profile

Проверить:

- timeline layout;
- animation evaluation;
- keyframe lookup;
- draw calls;
- texture allocation;
- waveform;
- text;
- effects;
- export.

## 22.3. Hot-path allocations

Минимизировать allocations в:

- audio callback;
- frame evaluation;
- timeline draw;
- renderer submission.

---

# 23. Stability pass

## 23.1. Undo/Redo torture tests

Циклы:

~~~text
create
→ animate
→ delete
→ undo
→ redo
→ save
→ reopen
~~~

## 23.2. Save/load round trip

~~~text
project
→ save
→ load
→ compare semantic state
~~~

## 23.3. Audio abuse tests

- seek spam;
- play/pause spam;
- project switch;
- invalid files;
- unusual sample rates.

## 23.4. Window/GPU

- resize;
- minimize;
- maximize;
- DPI changes;
- adapter differences;
- device lost strategy.

## 23.5. Broken inputs

Gracefully handle:

- missing image;
- missing audio;
- invalid project;
- future/incompatible version;
- corrupted cache.

---

# 24. UX/polish pass

Только после полного end-to-end workflow.

## 24.1. Visual consistency

- spacing;
- typography;
- iconography;
- hierarchy;
- selected/focused/hover states.

## 24.2. Interaction consistency

Одинаковая логика:

- Esc;
- Enter;
- Delete;
- Ctrl+C/V;
- modifiers;
- drag cancellation.

## 24.3. Empty states

Понятное поведение, когда:

- project empty;
- no audio;
- no selection;
- no assets.

## 24.4. Onboarding

Минимум:

- sensible defaults;
- shortcut hints;
- tooltips.

Сложный tutorial system не нужен.

---

# 25. Packaging и distribution

## 25.1. Windows build

Подготовить:

- release binary;
- dependencies;
- FFmpeg distribution strategy;
- third-party licenses.

## 25.2. Distribution

На ранних этапах:

- portable build.

К MVP:

- installer, если не создаёт лишний риск.

## 25.3. Versioning

Например:

~~~text
0.1.0
0.2.0
...
~~~

MVP не обязан называться 1.0.

---

# 26. MVP QA checklist

На чистой Windows-машине пройти:

1. start application;
2. create project;
3. import song;
4. configure BPM;
5. align BPM offset;
6. create shape;
7. create ellipse;
8. import image;
9. add text;
10. animate transform;
11. edit keyframes;
12. use snapping;
13. use easing;
14. use one effect;
15. save;
16. close;
17. reopen;
18. continue editing;
19. undo/redo;
20. export MP4;
21. play exported MP4 externally;
22. verify audiovisual sync.

---

# 27. MVP Definition of Done

MVP закончен только когда одновременно выполнены все группы условий.

## Functional

- основной сценарий работает end-to-end;
- audio playback стабилен;
- BPM grid работает;
- snapping работает;
- keyframe animation работает;
- realtime preview работает;
- save/load работает;
- export создаёт воспроизводимое видео.

## Performance

- простой/средний project держит стабильный realtime preview;
- timeline не лагает на реалистичном числе keyframes;
- waveform zoom/pan не вызывает заметных зависаний;
- audio не трещит в обычном workflow.

## Reliability

- основные операции имеют undo/redo;
- нет известных частых corruption bugs;
- есть минимальный crash recovery;
- bad asset не валит editor.

## UX

- новый пользователь понимает базовый workflow без технической документации;
- частые операции выполняются быстро;
- rhythm-first keyframe workflow ощущается главным преимуществом продукта.

---

# 28. Базовый порядок реализации

~~~text
PHASE 0
Product scope + technical decisions

↓

PHASE 1
Repository + app shell + CI

↓

PHASE 2
Core project model
Time model
Tempo/BPM model
Animation primitives

↓

PHASE 3
Minimal wgpu renderer
First rectangle

↓

PHASE 4
Minimal editor UI
Viewport + panels

↓

PHASE 5
Audio import + playback
Sample-based clock

↓

PHASE 6
Waveform + beat grid

↓

PHASE 7
Timeline + playhead
Basic keyframes + snap

↓

PHASE 8
Animation engine integrated with renderer

↓

PHASE 9
Objects
Rectangle / ellipse / image / text
Inspector

↓

PHASE 10
Selection + viewport transforms
Undo/redo integration

↓

PHASE 11
Advanced keyframe editing
Multi-select / copy-paste / easing

↓

PHASE 12
Save/load
Assets
Autosave/recovery

↓

PHASE 13
Effects

↓

PHASE 14
Curve editor

↓

PHASE 15
Offline export + FFmpeg

↓

PHASE 16
Performance + stability + UX polish

↓

PHASE 17
Packaging + MVP QA

↓

MVP
~~~

---

# 29. Milestone gates

Не строить много независимых идеальных подсистем. Каждый этап должен давать работающий vertical slice.

## Milestone A — Window

Приложение запускается и показывает editor shell.

## Milestone B — First Pixel

Renderer выводит объект.

## Milestone C — First Motion

Объект анимируется через core animation engine.

## Milestone D — First Sound

Импортируется и проигрывается музыка.

## Milestone E — First Beat

Waveform, BPM grid и playhead синхронизированы.

## Milestone F — First Rhythm Animation

Keyframe ставится на beat, и объект меняется синхронно с музыкой.

**Это первый настоящий proof of concept Rhythm Effects.**

## Milestone G — First Project

Композиция сохраняется и открывается без потери состояния.

## Milestone H — First Export

Ритмическая композиция экспортируется в MP4.

## Milestone I — MVP Candidate

Весь основной сценарий работает без release-blocking bugs.

## Milestone J — MVP

QA checklist пройден, build можно передать внешнему пользователю.

---

# 30. Документы для последующей детализации

Каждый документ ниже должен детализировать отдельную subsystem.

1. PRODUCT_SPEC.md — точные продуктовые границы и user flows.
2. ARCHITECTURE.md — modules, dependencies, data flow.
3. TECH_STACK.md — выбор библиотек и ADR.
4. TIME_MODEL.md — samples/seconds/frames/beats/ticks.
5. PROJECT_MODEL.md — структуры данных.
6. ANIMATION_ENGINE.md — keyframes/interpolation/evaluation.
7. COMMANDS_UNDO.md — command architecture.
8. RENDERER.md — wgpu pipeline.
9. AUDIO_ENGINE.md — decode/playback/clock/sync.
10. WAVEFORM.md — preprocessing/cache/rendering.
11. TIMELINE.md — timeline architecture + UX.
12. VIEWPORT.md — canvas/selection/gizmos.
13. EDITOR_UI.md — panels/focus/shortcuts.
14. ASSETS.md — import/cache/path strategy.
15. TEXT_RENDERING.md — fonts/shaping/rendering.
16. EFFECTS.md — shader/effect architecture.
17. EXPORT.md — offline renderer + FFmpeg.
18. SERIALIZATION.md — format/versioning/migrations.
19. PERFORMANCE.md — targets/benchmarks/profiling.
20. TESTING.md — unit/integration/manual QA.
21. PACKAGING.md — Windows releases.
22. MVP_BACKLOG.md — эпики и implementation tasks.

---

# 31. Шаблон детализации каждого раздела

Для каждой subsystem использовать один формат.

## Problem
Что решаем.

## User-facing behavior
Что видит и делает пользователь.

## Requirements
Что система обязана уметь.

## Non-goals
Что намеренно не делаем.

## Data model
Необходимые структуры.

## Public API
Как subsystem взаимодействует с остальным приложением.

## Architecture
Внутреннее устройство.

## Edge cases
Что может пойти не так.

## Performance
Ограничения по latency/CPU/GPU/memory.

## Testing
Как доказать корректность.

## Implementation steps
Пошаговый порядок.

## Definition of Done
Проверяемые критерии завершения.

---

# 32. Порядок детализации документов

Начать рекомендуется так:

~~~text
1. PRODUCT_SPEC.md
↓
2. TECH_STACK.md + ARCHITECTURE.md
↓
3. TIME_MODEL.md
↓
4. PROJECT_MODEL.md
↓
5. ANIMATION_ENGINE.md
↓
6. RENDERER.md
↓
7. AUDIO_ENGINE.md
↓
8. TIMELINE.md
↓
остальные subsystem specs
~~~

Сначала нужно максимально подробно определить PRODUCT_SPEC.md, потому что технический scope зависит от точных границ MVP.

---

# 33. Принцип vertical slice

На протяжении разработки сохранять работающую end-to-end цепочку:

~~~text
audio
→ beat
→ keyframe
→ animation
→ renderer
→ preview
→ save
→ export
~~~

Предпочитать упрощённую, но целиком работающую цепочку вместо нескольких идеальных подсистем, которые ещё не образуют продукт.

Каждая система:

1. получает минимальную реализацию;
2. подключается к общему workflow;
3. проверяется;
4. только затем углубляется.

---

# 34. North Star MVP

Если приходится выбирать между двумя задачами, приоритет получает та, которая сокращает цикл:

~~~text
услышал момент
→ нашёл beat
→ поставил keyframe
→ изменил свойство
→ увидел результат
~~~

Скорость, точность и приятность именно этого цикла должны стать главным отличием Rhythm Effects от обычного motion-design software.
