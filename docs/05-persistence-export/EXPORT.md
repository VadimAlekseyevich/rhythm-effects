# Export

> **Status: Accepted for MVP**
>
> Export is deterministic offline rendering, never screen capture.

## 1. Output target

MVP:

- container: MP4;
- video codec: H.264 through the bundled/validated FFmpeg build;
- final pixel format: yuv420p for broad compatibility;
- audio: AAC when primary audio is present;
- SDR sRGB-compatible output.

The exact FFmpeg H.264 encoder implementation is a packaging capability, not project semantics. Release packaging must provide and validate one supported encoder.

## 2. Snapshot

At export start, create an immutable semantic Project snapshot and resolve required asset references.

Subsequent editor edits do not alter the running export.

The editor may remain responsive; export uses its snapshot.

AI-273 adds `ExportJobSnapshot::capture(&ProjectEditor)` at a committed
editing boundary. It rejects active drag/keyframe transactions, clones and
validates the complete semantic Project, and retains the captured revision
for job diagnostics. Its Project accessor is read-only, and export scene
evaluation takes the snapshot rather than the mutable editor. Later edits,
undo/redo and additional export captures cannot change an existing job's
creative state. Runtime media/font resolution and separate GPU resources are
subsequent tasks and are deliberately not stored in this initial snapshot.

## 3. Time

For frame index N:

~~~text
ProjectTimeNs =
    N * fps_denominator * 1e9 / fps_numerator
~~~

with wide integer intermediates and explicit rounding.

Never accumulate frame delta.

AI-275 reuses the central `rhythm_core::time::project_time_for_frame`
rational i128 computation for arbitrary direct frame indexes, with a
nearest-nanosecond/ties-away-from-zero result; the export timestamp is
independent of prior frames, playback tempo, or preview seek order.

## 4. Range

Default export range is whole composition:

~~~text
0 .. ProjectSettings.duration
~~~

Custom in/out range is post-MVP.

AI-276 captures the default half-open [0, composition duration) range in
`ExportFrameTimeline`. Frame count is computed by exact integer ceiling
of `duration_ns * fps_numerator / (fps_denominator * 1e9)`, so a
non-frame-aligned final fraction still receives its last frame start
without adding a spurious frame at an exact end boundary. Frame-index
queries reject indexes outside this precomputed count. FPS overrides
never alter the snapshot's semantic composition frame rate.

## 5. Export settings

MVP UI exposes:

- destination;
- output resolution preset/custom dimensions constrained to same composition aspect ratio;
- output FPS preset;
- quality: Fast / Balanced / High.

Defaults:

- composition resolution;
- composition FPS;
- Balanced.

Supported common FPS presets include 24, 25, 30, 50, 60.

Rational internals remain supported.

## 6. Resolution scaling

When output resolution differs but keeps aspect ratio:

~~~text
output_scale = output_width / composition_width
~~~

All composition-space geometry/effect units scale consistently into output pixels.

Do not reinterpret project coordinates.

AI-277 adds `ExportOutputResolution`: it verifies source and output
dimensions against V1 bounds and uses integer cross multiplication to
require exactly the same rational aspect ratio, rejecting even small
rounding-induced stretches. Its immutable composition/output integer
ratio scales every geometry/effect/font spatial pixel parameter consistently
without modifying the snapshot. Alpha, color and intensity are not spatial
quantities. The full-resolution GPU render-target allocation follows in
the export renderer integration tasks.

## 7. Rendering worker

Export job owns separate export-renderer state while reusing the same shader/semantic code.

It may share thread-safe wgpu Device/Queue handles where implementation supports clean ownership, but it does not mutate preview renderer caches/state.

AI-278 separates two phases: `ExportRendererPlan::prepare` validates
aspect-safe output dimensions, exact frame timeline and ALL required media/font
dependencies from an immutable job snapshot, before allocating GPU targets.
`initialize_gpu` then creates its own size-specific `Renderer`, retaining
independent composition/preview-display textures, effect pipelines, temporary
pool, text/font state, and image caches. The construction reuses the actual
preview Renderer implementation and its semantic evaluator, never borrows or
resizes the app preview Renderer. Hardware-free tests check that the plan
respects resolution/FPS overrides, captures the original Project and rejects
stretching ahead of GPU work. Upload/render/FFmpeg remain subsequent steps.

If concurrent GPU use hurts editor responsiveness, preview can be throttled while export runs; Project editing remains semantically independent.

## 8. Render pipeline

~~~text
snapshot
-> validate/prepare assets
-> for each frame index:
     exact ProjectTimeNs
     animation evaluation
     full-resolution render
     GPU sRGB RGBA/BGRA output conversion
     bounded readback
     FFmpeg raw-video pipe
-> mux/encode primary audio
-> finalize temp MP4
-> publish destination
~~~

## 9. Frame transport

MVP uses raw video through FFmpeg stdin/pipe.

A bounded pool of at most 3 readback/frame buffers prevents memory growing with export length.

Zero-copy hardware encoding is post-MVP.

## 10. Audio

Primary audio comes from original audio asset file, not captured playback output.

Audio starts at project time 0 in MVP.

If composition ends before audio:
- trim/mux to composition duration.

If composition extends beyond audio:
- video may continue after audio ends.

No audio time-stretch or mixing.

A project without audio may export video-only MP4.

## 11. A/V sync

Release-critical.

Use generated fixture with audio clicks and visual flashes at known ProjectTimeNs.

No cumulative sync drift is allowed.

Export timing does not depend on realtime audio clock.

## 12. Preview parity

At any tested ProjectTimeNs:

- same animation evaluator;
- same object/effect semantics;
- same text layout;
- same color/alpha contract.

Preview resolution may differ, but full-resolution export is semantic reference.

## 13. Asset readiness

Before first frame:

- all required image assets resolved/decoded;
- required fonts resolved/fallback known;
- source audio path available if audio included.

Missing required asset blocks export with actionable list rather than silently producing broken output.

AI-274 implements `ExportJobSnapshot::prepare_resources(canonical_project_path)`
as an all-or-nothing, pre-frame worker operation. It resolves normalized
absolute/project-relative sources, deduplicates and fully decodes all visible
referenced images using the editor's PNG/JPEG/WebP decoder, probes the primary
audio file with Symphonia and shapes visible Text objects using the same
font-system creation and bundled Inter fallback as preview. A requested font
that is not installed produces an explicit fallback report; missing glyphs
are an actionable preflight failure. It collects every missing/broken media
and glyph issue before declining to start frame 0; invisible images and
unreferenced asset records are not required. The decoded resources are owned
by the export job, and the project remains unchanged. This is CPU/media/font
preparation, not yet GPU upload or FFmpeg encoding; those are separate tasks.

## 14. Progress

Show:

- current frame / total frames;
- percent;
- elapsed time.

ETA may be shown only when enough throughput history exists and is labelled approximate.

## 15. Cancellation

Atomic/cooperative cancellation:

- stop producing new frames;
- close/terminate FFmpeg safely;
- release readback/resources;
- delete incomplete temp output;
- Project remains unchanged.

## 16. Output publication

Never encode directly into the only requested final file path.

~~~text
destination.partial/temp
-> FFmpeg success + close
-> verify output exists/non-empty
-> safe rename/publish
~~~

Failed/cancelled export is never reported as success.

## 17. FFmpeg diagnostics

Capture stderr.

Translate common failures:

- encoder unavailable;
- permission denied;
- disk full;
- source audio invalid;
- process terminated.

Raw stderr goes to logs/details.

## 18. Encoder quality presets

Fast/Balanced/High map to encoder-specific settings inside export backend.

The UI contract is stable even if bundled FFmpeg encoder implementation changes.

Document the concrete mapping in code/release notes once the release encoder is selected.

## 19. Color

Renderer performs creative linear working math.

AI-280 adds an export-only cached Rgba8Unorm target with
`RENDER_ATTACHMENT | COPY_SRC` usage, separate from the linear-premultiplied
Rgba16Float composition. Its WGSL reuses the preview shader's exact linear to
sRGB transfer/unpremultiplication math but writes **straight-alpha**, clipped
SDR sRGB RGBA8 for FFmpeg/readback instead of preview's premultiplied UI
texture. Alpha-zero RGB remains zero; HDR values above SDR are clipped at
output. `ExportRendererState::encode_sdr_frame` encodes the conversion after
creative compositing. There is no unnecessary texture/view recreation for
ordinary subsequent frames. The GPU readback/mux and complete object draw
path are separate follow-up steps.

Final export conversion targets standard SDR sRGB appearance before FFmpeg YUV conversion.

No HDR metadata or wide-gamut support in MVP.

## 20. Determinism

Semantic frame content is deterministic for the same:

- Project snapshot;
- frame index;
- app rendering semantics.

Bit-identical H.264 files are not required.

## 21. Tests

- short 1080p60;
- 720p scale;
- 30 FPS override;
- video-only;
- audio included;
- 1/5/10 minute sync fixture;
- text/effects reference frames;
- missing asset block;
- cancel;
- FFmpeg fail;
- destination fail;
- bounded memory;
- output playable in common Windows/browser/player software.

## 22. Definition of Done

Export is MVP-ready when deterministic frames, H.264 MP4, optional AAC audio, scale/FPS overrides, sync tests, bounded readback memory, cancellation, and safe output publication all pass.
