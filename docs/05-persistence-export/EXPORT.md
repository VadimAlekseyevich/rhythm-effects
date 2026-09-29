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

AI-282–288 implement an export-worker FFmpeg child-process boundary using
`std::process::Command` with one structured argument per path/option and
an explicitly supplied absolute resolved/bundled FFmpeg executable. No
`cmd.exe`, shell interpolation, or user-controlled command-line string.
The backend currently targets a release FFmpeg with `libx264`: raw
`rgba` at exact rational FPS and output size is supplied over `pipe:0`;
H.264 `yuv420p` video is written as MP4 to a caller-owned unique
`.partial.mp4` path with `-n` to prevent overwrite. The process owns a
piped stdin and `write_frame(index, rgba)` checks sequential frame order,
exact packed byte count, and configured frame count before writing all bytes.
`finish` closes the video pipe and checks final process status; drop on an
unfinished job kills/reaps the owned child.

For optional primary audio, the source path comes from export resource
preflight of the **original** primary audio asset, passed as a single `-i`
argument and explicitly mapped to `1:a:0`. AAC uses a composition-duration
`atrim` plus reset timestamps. Omitting `-shortest` preserves a video
tail after shorter audio; no audio means `-an` video-only MP4. Three explicit
`libx264` quality mappings are Fast=veryfast/CRF23,
Balanced=medium/CRF20 and High=slow/CRF17. Tests check argument order,
literal Unicode/spaces/shell metacharacter paths, format, FPS, AAC duration,
video-only behavior, bounds, and presets. Encoder capability probing,
bounded diagnostic collection, asynchronous readback/pipe coordination and
safe final file publication remain separate tasks; these process APIs do
not mean a fully creative end-to-end MP4 export is already available.

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

AI-281 creates three GPU `COPY_DST | MAP_READ` buffers once per
resolution-specific export renderer. An exclusive generation-tagged ticket
reserves one slot per encoded SDR RGBA8 texture-to-buffer copy; when all
three remain in flight the producer receives explicit backpressure rather
than allocating more. wgpu's required 256-byte row pitch is computed by
checked integer arithmetic, and the packing helper strips per-row padding
into tightly packed RGBA8 for the FFmpeg rawvideo pipe. A ticket is returned
only after the consumer finishes async mapping, copying and unmapping;
stale/double release is rejected. Pure tests cover 3×2 padded rows,
aligned rows, stale tokens, slot reuse and the three-buffer cap.
Actual async mapping/pipe integration is part of subsequent export tasks.

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

AI-289 introduces `ExportProgress`: only frames actually delivered to the
encoder count toward completed/total and percentage, monotonically; elapsed
time comes from the job's start Instant and never feeds into creative
timestamps. Invalid backward/out-of-bounds updates are rejected without
changing the last valid count. ETA is deliberately not fabricated from a
single sample.

## 15. Cancellation

Atomic/cooperative cancellation:

- stop producing new frames;
- close/terminate FFmpeg safely;
- release readback/resources;
- delete incomplete temp output;
- Project remains unchanged.

AI-290 provides a cloneable atomic cancellation signal, checked at the
export worker's frame-delivery and process-finalization boundaries. Dropping
or explicitly cancelling its owned FFmpeg process closes stdin, kills an
unfinished child and reaps it on the export worker. A second token check
after encoder success prevents publication of a newly cancelled job.
No editor/audio realtime callback blocks on encoder process I/O.

## 16. Output publication

Never encode directly into the only requested final file path.

~~~text
destination.partial/temp
-> FFmpeg success + close
-> verify output exists/non-empty
-> safe rename/publish
~~~

Failed/cancelled export is never reported as success.

AI-291 introduces `PartialExportOutput`: a unique same-directory partial
MP4 pathname reserved by an exclusive create_new marker, with no
overwrite of the final path during encoding. Finalization closes FFmpeg
stdin and checks its exit status, opens the partial, verifies it is a
nonempty regular file, flushes/synchronizes it and closes the handle
before same-directory rename/replace. If FFmpeg fails, cancellation arrives,
the partial is empty, or OS publication fails, Drop removes only the
owned partial/marker and leaves the previous final file intact. Tests cover
replacing an older known-good output, two concurrent unique stages, an
abandoned/cancelled stage, empty file, blocked publication and rejected
relative destination. This prepares safe publication independently from
the unfinished full creative render/async readback job runner.

## 17. FFmpeg diagnostics

Capture stderr.

Translate common failures:

- encoder unavailable;
- permission denied;
- disk full;
- source audio invalid;
- process terminated.

Raw stderr goes to logs/details.

AI-292 replaces inherited process stderr with a dedicated immediately started
reader that continuously drains the pipe until FFmpeg closes it; the encoder
therefore cannot deadlock because its stderr pipe becomes full. Only the final
64 KiB of diagnostic bytes are retained, independent of export duration or
log volume. On nonzero exit, an `FfmpegDiagnostic` reports the bounded
stderr tail and maps common messages into encoder unavailable, permission
denied, disk full, invalid source audio, or general process failure. Reader
spawn/read/join errors are separate structured process-stage errors. The
reader is also joined when an unfinished process is killed/reaped by Drop.
Tests verify exact last-64-KiB retention and each diagnostic category.

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
