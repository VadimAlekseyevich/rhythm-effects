# Export

> **Status: Draft**
>
> Export is deterministic offline rendering. It must not depend on realtime preview frame delivery or screen capture.

## 1. MVP goal

Produce a standard playable MP4 containing:

- rendered composition video;
- project audio;
- correct audiovisual synchronization.

Minimum codec/container target:

- H.264;
- MP4.

---

## 2. Core invariant

Preview and export share:

- Project data;
- Time Model;
- Animation evaluator;
- visual scene semantics;
- renderer/shaders as far as practical.

They differ primarily in clock source and output target.

### Preview

~~~text
audio/editor clock
→ evaluation time
→ scene
→ offscreen preview
~~~

### Export

~~~text
integer frame index
→ exact frame timestamp
→ same evaluation semantics
→ offscreen export target
→ encoder
~~~

---

## 3. No realtime dependency

Export must not assume it can render at playback speed.

Rendering may be:

- faster than realtime;
- slower than realtime.

Audio is not “played” during export.

The source audio is encoded/muxed against deterministic timeline timestamps.

---

## 4. Frame time

For rational frame rate:

~~~text
timestamp(frame_index) =
    frame_index * fps_denominator / fps_numerator
~~~

Derive timestamp directly from frame index.

Do not repeatedly add floating-point frame duration.

This prevents accumulated timing drift.

---

## 5. Export range

MVP default:

- entire composition duration.

Optional in/out range can be deferred unless inexpensive.

Export duration should not be implicitly limited to audio length if composition supports visual tail after audio.

---

## 6. Export settings

MVP surface:

- destination path;
- width;
- height;
- FPS;
- quality preset.

Possible quality UI:

- Low;
- Medium;
- High;

or a restrained single quality slider/preset.

Do not expose dozens of encoder flags in MVP.

---

## 7. Resolution

Defaults to composition resolution.

Allow override only if renderer can correctly scale composition semantics.

If override complicates pixel-sensitive effects/text, MVP may initially require composition resolution.

This should be decided during renderer/export spike.

---

## 8. FPS

Default to composition FPS.

Supported initial values may include:

- 24;
- 25;
- 30;
- 50;
- 60.

Represent frame rate rationally internally.

Do not couple animation keyframe positions to FPS.

---

## 9. Rendering pipeline

Conceptual pipeline:

~~~text
load/freeze export project state
→ prepare runtime assets
→ for frame N:
     derive exact timestamp
     evaluate project
     render composition to export texture
     transfer/encode frame
→ encode/mux project audio
→ finalize container
→ atomically publish output where practical
~~~

---

## 10. Project consistency during export

MVP preferred behavior:

Export operates on a stable snapshot of project semantic state captured at export start.

The user may either:

- continue editing while export uses snapshot, if implementation is safe;
- or editor may restrict mutation during export initially.

Do not allow export result to unpredictably combine states from different moments.

A snapshot/clone cost should be measured.

---

## 11. GPU target

Export renders to an offscreen texture at requested output dimensions.

No dependency on:

- viewport zoom;
- window size;
- monitor DPI;
- whether editor window is visible;
- swapchain contents.

---

## 12. Readback / encoder boundary

Encoding H.264 through FFmpeg requires a clear frame transfer path.

MVP preferred integration:

- FFmpeg process boundary;
- feed raw frames through pipe or temporary strategy;
- capture stderr/progress diagnostics;
- mux audio through explicit arguments/input.

Exact implementation must be benchmarked.

Potential formats across pipe:

- RGBA;
- BGRA;
- another format minimizing conversion cost.

Avoid premature native FFmpeg bindings unless process-based approach proves inadequate.

---

## 13. Audio source

Use project/source audio data aligned to project time.

Requirements:

- correct start offset;
- correct duration;
- preserve synchronization;
- handle composition beginning before/after musical grid origin correctly.

Do not derive export audio from live audio output capture.

---

## 14. Audio/video synchronization

A/V sync is a release-critical correctness property.

Test using:

- known click/transient audio;
- visual event placed on exact beat;
- exported file inspected at multiple timestamps;
- long-duration projects.

Define numerical tolerance in TESTING.md after prototype measurement.

---

## 15. Color and alpha

MVP MP4/H.264 output is opaque.

Transparent video export is not required.

Composition background should be resolved before encoding.

Color-management sophistication beyond standard SDR output is not MVP.

Exact color-space handling should still be explicit enough to avoid accidental obvious gamma differences.

---

## 16. Effect parity

Export must use the same effect parameter evaluation and shader semantics as preview.

If an effect cannot render equivalently offline, it should not be considered finished for MVP.

---

## 17. Text parity

Text layout/font resolution must match preview.

Missing font at export time is a blocking/recoverable error unless fallback semantics are explicitly the same as preview.

---

## 18. Asset readiness

Before frame loop, validate required assets.

Possible behavior:

- wait for required decode/loading;
- fail with clear missing asset list.

Do not produce silently broken final video when a required source asset cannot be resolved.

---

## 19. Progress

Expose:

- current frame;
- total frames;
- percentage;
- optional elapsed time;
- optional estimated remaining time only if stable enough.

Do not promise a precise ETA if throughput is highly variable.

---

## 20. Cancellation

Cancellation must:

- stop scheduling new frames;
- terminate/close encoder safely;
- clean temporary output;
- leave editor project unchanged;
- not publish a corrupt file as successful output.

Partial file policy should be explicit.

Preferred: write temporary destination, then rename on success.

---

## 21. Output publication

Safer flow:

~~~text
destination.tmp
→ complete encode
→ verify FFmpeg success
→ rename/move to requested destination
~~~

If final rename fails, retain enough information to recover or clearly tell the user where temporary output exists.

---

## 22. FFmpeg errors

Translate common errors into understandable product messages:

- FFmpeg unavailable;
- unsupported encoder/build;
- destination permission denied;
- disk full;
- pipe/process terminated;
- invalid source audio;
- encoder failure.

Keep raw stderr in diagnostic logs.

Do not show the raw command line as the primary UX.

---

## 23. Packaging boundary

If FFmpeg is distributed with the app:

- version is controlled;
- executable path is known;
- license obligations documented;
- startup/export verifies expected executable.

Do not rely on user PATH for normal MVP operation unless distribution policy explicitly chooses that tradeoff.

See PACKAGING.md.

---

## 24. Performance

Export throughput is secondary to correctness but should still be efficient.

Measure:

- animation evaluation/frame;
- render/frame;
- GPU readback;
- CPU pixel conversion;
- encoder throughput;
- total memory.

Potential pipeline optimization later:

- multiple readback buffers;
- pipelined GPU/CPU/encoder stages.

MVP should first implement a correct bounded-memory pipeline.

---

## 25. Memory

Never accumulate all rendered frames in RAM.

Use streaming/bounded buffering.

Long exports must have approximately bounded memory relative to resolution and pipeline depth.

---

## 26. Determinism

Given:

- same project;
- same application/render implementation;
- same export settings;

semantic visual state at frame N should be the same across runs.

Bit-identical encoded H.264 output is not required.

Frame evaluation semantics are.

---

## 27. Testing

### Basic

- short 1080p project exports;
- output plays;
- audio present;
- correct dimensions/FPS.

### Timing

- visual flash on known audio beat;
- 1 minute;
- 5+ minutes;
- 29.97/30/60 if supported;
- decimal BPM.

### Parity

Compare exported reference frames with offline renderer expected output at selected frame indices.

### Failure

- missing asset;
- bad output path;
- disk/write failure where simulatable;
- FFmpeg termination;
- cancel mid-export.

### Memory

Long export does not grow linearly with total frame count.

---

## 28. Implementation sequence

1. Implement exact frame-index timestamp API.
2. Render one arbitrary timestamp to offscreen texture.
3. Read one frame back to CPU.
4. Encode a sequence of generated frames with FFmpeg.
5. Feed real rendered frames.
6. Add audio mux.
7. Add progress.
8. Add cancellation.
9. Add temp-output publication.
10. Add UI settings.
11. Add sync/parity tests.
12. Benchmark and pipeline only if needed.

---

## 29. Open decisions

- process pipe vs temporary frame transport;
- pixel format;
- exact quality presets;
- whether resolution override is MVP;
- supported FPS list;
- audio encode/copy strategy;
- FFmpeg distribution model;
- editor mutability during export;
- precise A/V sync tolerance;
- standard SDR color-space convention.

---

## 30. Definition of Done

Export is MVP-ready when:

- a full composition renders to H.264 MP4;
- audio is present;
- A/V sync passes defined tests;
- frame time is derived deterministically from integer frame index;
- preview/export visual semantics match at tested timestamps;
- memory does not grow with total frame count;
- missing assets fail clearly;
- cancellation is safe;
- failed export is not presented as a successful final file;
- output plays correctly in common external players.
