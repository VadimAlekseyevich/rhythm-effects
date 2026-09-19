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

## 3. Time

For frame index N:

~~~text
ProjectTimeNs =
    N * fps_denominator * 1e9 / fps_numerator
~~~

with wide integer intermediates and explicit rounding.

Never accumulate frame delta.

## 4. Range

Default export range is whole composition:

~~~text
0 .. ProjectSettings.duration
~~~

Custom in/out range is post-MVP.

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

## 7. Rendering worker

Export job owns separate export-renderer state while reusing the same shader/semantic code.

It may share thread-safe wgpu Device/Queue handles where implementation supports clean ownership, but it does not mutate preview renderer caches/state.

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
