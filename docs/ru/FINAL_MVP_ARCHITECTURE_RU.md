# Rhythm Effects — финальная архитектура MVP

> **Статус: актуальный русскоязычный обзор.**
>
> Canonical implementation contracts находятся в английских документах docs/00–07. Этот файл нужен для быстрой ориентации.

## Философия

Rhythm Effects — не урезанный After Effects с BPM.

Это rhythm-first motion-design editor:

~~~text
музыка
→ BPM grid
→ keyframes в музыкальном времени
→ motion design
~~~

Три ограничения проекта:

1. комфортный, крупный, неглубокий UI;
2. высокая runtime-производительность и быстрый developer iteration;
3. музыкальная сетка является основой authoring-модели.

## Время

Canonical keyframe time:

~~~text
MusicalTick(i64)
PPQ = 960
~~~

BPM хранится как BpmMicros(u64), project time — ProjectTimeNs(i64).

Keyframe не хранит секунды как источник истины. Playhead и evaluation непрерывные, но создание/перемещение keyframes происходит по BPM grid.

Сетка MVP:

~~~text
1/1, 1/2, 1/3, 1/4, 1/6,
1/8, 1/12, 1/16, 1/24, 1/32
~~~

## Animation

Основная модель:

~~~text
Animated<T>
  base_value
  keyframes[]

Keyframe<T>
  id
  MusicalTick
  value
  interpolation
~~~

Interpolation исходящего keyframe управляет сегментом до следующего.

Поддерживаются Hold, Linear и Cubic Bezier.

Rotation не использует shortest-path:

~~~text
0 → 360 = один полный оборот
0 → 720 = два оборота
~~~

## Project

MVP содержит:

- одну Composition;
- один основной AudioTrack;
- Rectangle;
- Ellipse;
- Image;
- Text;
- Blur;
- Glow;
- Tint;
- Noise;
- RGB Split.

Project format:

~~~text
.rhfx
versioned JSON
~~~

IDs — typed project-local u64.

Creative Project не содержит selection, playhead, GPU resources, PCM, waveform cache или undo stack.

## State ownership

Активный Project имеет одного writer-а в editor/application context.

Не используется общий Arc/Mutex Project между UI, audio и workers.

Background workers получают owned/immutable input и возвращают result.

Audio callback никогда не мутирует Project.

## Threading

Основные contexts:

- main/editor;
- realtime audio callback;
- bounded background workers;
- export job.

Общий Tokio/async runtime для MVP не используется.

## Audio

Stack:

- Symphonia;
- Rubato;
- CPAL.

Трек декодируется в background, строится waveform, затем готовится PCM под sample rate output device.

Realtime callback только читает подготовленный PCM.

Playback clock основан на CPAL playback timestamps/stream clock, а не UI frame delta.

## Renderer

Stack:

- wgpu;
- WGSL.

Working target:

~~~text
Rgba16Float
linear-light RGB
premultiplied alpha
~~~

Preview:

~~~text
Auto / Full / Half / Quarter
~~~

Spatial effect units остаются composition pixels независимо от preview scale.

Export использует те же creative rendering semantics.

## Coordinates

~~~text
origin = top-left
+X = right
+Y = down
1 unit = 1 composition pixel
~~~

Anchor normalized, default center = (0.5, 0.5).

Transform:

~~~text
subtract anchor
→ scale
→ rotate
→ translate
~~~

Positive rotation выглядит clockwise.

## UI

Default project:

~~~text
1920×1080
60 FPS
10 s до импорта audio
black background
4/4
~~~

Persistent workspace:

- Object list;
- Viewport;
- Inspector;
- Transport/Rhythm;
- Timeline.

Нет fully dockable IDE UI.

Space всегда Play/Pause.

Основные shortcuts:

~~~text
P Position
S Scale
R Rotation
O Opacity
K keyframe action
Ctrl+K command search
~~~

Global Auto-Key отсутствует. Animation включается на уровне property.

## Timeline

Waveform, BPM grid, playhead и keyframes используют один time-to-x transform.

Keyframe drag всегда проходит через musical grid.

Grid change не двигает существующие keys.

Rendering/hit-testing виртуализированы по visible rows/time range.

## Text

Stack:

- cosmic-text;
- glyphon.

MVP использует installed/system fonts.

Bundled Inter — deterministic fallback.

Поддерживаются Latin/Cyrillic, multiline, alignment.

Imported/embedded fonts — post-MVP.

## Assets

MVP assets:

- Audio;
- PNG/JPEG/WebP Image.

Файлы остаются внешними.

Если asset находится внутри project directory — путь хранится relative, иначе absolute.

Missing files не ломают Project: используется Relink.

## Undo/Redo

Все creative mutations проходят через ProjectEditor.

Drag:

~~~text
begin
→ live update
→ commit/cancel
~~~

Один drag = один history entry.

History capacity MVP: 500 logical entries.

Полный Project не snapshot-ится на каждое изменение.

## Save / Recovery

Save:

~~~text
serialize
→ temporary file
→ flush/close
→ safe replace
~~~

Recovery:

- только dirty Project;
- максимум раз в 30 секунд;
- после окончания active transaction;
- current + previous generation;
- LocalAppData;
- Restore не перезаписывает canonical .rhfx автоматически.

## Export

MVP:

- deterministic offline render;
- MP4;
- H.264;
- yuv420p;
- AAC audio;
- FFmpeg child process;
- raw-video pipe;
- bounded readback;
- progress;
- cancel;
- temp output → publish on success.

Frame timestamp вычисляется напрямую из frame index.

## Performance gates

Primary Medium fixture:

~~~text
100 objects
1,000 keyframes
1080p
mixed text/images/effects
10-minute audio
~~~

Target:

~~~text
60 FPS
p95 <= 16.67 ms
p99 <= 25 ms
~~~

Audio:

- no cumulative drift;
- target <=10 ms sync error where timestamps reliable;
- release ceiling <=25 ms systematic error;
- 30-minute stress playback without known underruns.

Timeline Stress:

~~~text
500 objects
10,000+ keyframes
~~~

должен оставаться навигируемым без O(total keys) draw path каждый frame.

## Packaging

MVP:

- Windows 10/11 x86_64;
- portable ZIP;
- bundled validated FFmpeg;
- no installer requirement;
- no auto-update;
- unsigned MVP допустим;
- no telemetry/account/cloud.

## Non-goals

До MVP не входят:

- macOS/Linux;
- SVG/video editing;
- 3D;
- particles;
- masks/roto;
- motion tracking;
- parenting;
- scripts/expressions;
- plugins;
- nodes;
- multi-track audio;
- tap tempo/BPM detection;
- packed projects;
- imported fonts;
- temporal feedback effects;
- cloud/collaboration;
- AI generation.

## Состояние документации

Pre-code milestone M0 — Architecture Contract — завершён.

Главный gate: ../07-planning/IMPLEMENTATION_READINESS.md

Документация считается контрактом. Если реализация доказывает несовместимость с library/hardware assumption, сначала фиксируется evidence и обновляется spec/ADR, затем меняется код.
