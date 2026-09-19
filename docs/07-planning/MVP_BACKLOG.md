# MVP Backlog

> **Status: Accepted for MVP**
>
> This is the implementation index. Canonical behavior remains in subsystem specs.

## Rules

- work in milestone/vertical-slice order;
- tests/performance instrumentation are part of the epic;
- no speculative framework work;
- a checkbox does not override a failed milestone gate;
- post-MVP items require explicit promotion.

## Epic 0 — Repository and development foundation

- [ ] Cargo workspace with rhythm_core/rhythm_engine/rhythm_app.
- [ ] Rust toolchain policy.
- [ ] Cargo.lock.
- [ ] Pin accepted dependencies/features.
- [ ] rustfmt/clippy.
- [ ] Windows CI build/test.
- [ ] tracing/log bootstrap.
- [ ] dev/release profiles.
- [ ] diagnostics timing primitives.
- [ ] build baseline record.

Specs: ARCHITECTURE, TECH_STACK, PERFORMANCE, IMPLEMENTATION_READINESS.

## Epic 1 — Window/UI/render integration

- [ ] winit lifecycle.
- [ ] one wgpu Device/Queue.
- [ ] egui-winit/egui-wgpu.
- [ ] Rgba16Float composition target.
- [ ] texture displayed in viewport.
- [ ] resize/minimize/DPI.
- [ ] five-region editor shell.
- [ ] focus skeleton.
- [ ] diagnostics overlay shell.

Gate: M1/M2 foundation.

## Epic 2 — Core domain/time

- [ ] all domain newtypes.
- [ ] typed IDs/allocator.
- [ ] BpmMicros.
- [ ] ProjectTimeNs/Duration/GridOffset.
- [ ] PPQ 960.
- [ ] BeatDivision set.
- [ ] TimeSignature/TempoMap.
- [ ] rational FrameRate.
- [ ] tick/time/frame conversions.
- [ ] snap tie policy.
- [ ] conversion tests.

## Epic 3 — Project schema/model

- [ ] Project/File V1 Rust types.
- [ ] Rectangle/Ellipse/Image/Text semantic data.
- [ ] TransformAnimation.
- [ ] AssetRecord.
- [ ] AudioTrack.
- [ ] five EffectKind variants.
- [ ] FontReference.
- [ ] validation/safety limits.
- [ ] default project 1920×1080/60/10s/black.

## Epic 4 — Animation

- [ ] Animated<T>/Keyframe<T>.
- [ ] sorted uniqueness.
- [ ] Hold/Linear/CubicBezier.
- [ ] preset curves.
- [ ] rotation/color semantics.
- [ ] EvaluatedScene.
- [ ] arbitrary-seek/determinism tests.

Gate: M3.

## Epic 5 — Commands/history

- [ ] ProjectEditor mutation boundary.
- [ ] EditCommand intents.
- [ ] specialized HistoryEntry.
- [ ] transaction begin/update/commit/cancel.
- [ ] dirty/saved revisions.
- [ ] 500-entry capacity.
- [ ] key-repeat coalescing.
- [ ] compound import/create.
- [ ] undo/redo tests.

## Epic 6 — Base renderer

- [ ] transform matrices matching spec.
- [ ] rectangle analytic edge.
- [ ] ellipse.
- [ ] image texture/sRGB path.
- [ ] painter order.
- [ ] preview quality Auto/Full/Half/Quarter.
- [ ] texture pool skeleton.
- [ ] renderer counters/reference tests.

## Epic 7 — Audio preparation/playback

- [ ] Symphonia required formats.
- [ ] mono/stereo source PCM.
- [ ] waveform handoff.
- [ ] Rubato output-rate preparation.
- [ ] source PCM release.
- [ ] CPAL default device.
- [ ] callback format/channel adapter.
- [ ] play/pause/seek/end/gain.
- [ ] generation clock anchors.
- [ ] CPAL playback timestamp verification.
- [ ] documented fallback.
- [ ] audio diagnostics/stress tests.

Gate: M4.

## Epic 8 — Waveform

- [ ] 64-frame base peaks.
- [ ] mip pyramid.
- [ ] background worker/generation.
- [ ] visible-range query.
- [ ] batched timeline drawing.
- [ ] optional safe disk cache.
- [ ] tests/benchmarks.

## Epic 9 — Timeline foundation

- [ ] shared time/x transform.
- [ ] ruler/waveform fixed rows.
- [ ] bar/beat/subdivision lines.
- [ ] zoom around pointer.
- [ ] horizontal pan.
- [ ] continuous ruler playhead.
- [ ] object/property rows.
- [ ] vertical/horizontal virtualization.
- [ ] focus/navigation shortcuts.

Gate: M5.

## Epic 10 — Timeline keyframe editing

- [ ] key glyph/hit box.
- [ ] property create semantics.
- [ ] single/multi/box selection.
- [ ] grid drag.
- [ ] integer multi-drag.
- [ ] collision replacement.
- [ ] copy/paste packet.
- [ ] duplicate.
- [ ] Alt/Ctrl+Alt movement.
- [ ] easing indicators/actions.
- [ ] undo integration.

Gate: M6.

## Epic 11 — Viewport editing

- [ ] camera.
- [ ] CPU inverse-transform hit testing.
- [ ] object selection/Ctrl toggle.
- [ ] lock/visibility behavior.
- [ ] move + Shift axis.
- [ ] scale + Shift uniform.
- [ ] rotation + Shift 15°.
- [ ] anchor marker.
- [ ] multi-object move.
- [ ] animated direct-edit semantics.
- [ ] center guides.

## Epic 12 — Inspector/focus/hotkeys

- [ ] standard property row.
- [ ] static/animated states.
- [ ] P/S/R/O + K.
- [ ] numeric edit transactions.
- [ ] last-key removal policy.
- [ ] multi-selection mixed states.
- [ ] object-specific controls.
- [ ] command search Ctrl+K.
- [ ] physical-key dispatch with text-input protection.

## Epic 13 — Assets/images

- [ ] rfd dialogs.
- [ ] PNG/JPEG/WebP decode.
- [ ] external paths.
- [ ] relative-on-project-dir policy.
- [ ] duplicate path reuse.
- [ ] runtime generation safety.
- [ ] GPU upload.
- [ ] thumbnails.
- [ ] missing/relink.
- [ ] referenced-delete block.
- [ ] OS drag/drop.

## Epic 14 — Text

- [ ] cosmic-text/glyphon integration.
- [ ] system font list/cache.
- [ ] Inter bundled fallback.
- [ ] Cyrillic/Latin/multiline.
- [ ] weight/style/alignment.
- [ ] deterministic bounds.
- [ ] animated color.
- [ ] missing-font UI.
- [ ] preview/export parity fixture.

## Epic 15 — Curves/easing

- [ ] preset actions.
- [ ] one-segment curve surface.
- [ ] bounded x/y handles.
- [ ] live preview.
- [ ] transaction/cancel.
- [ ] multi-selection preset application.

## Epic 16 — Effects

- [ ] effect stack Inspector.
- [ ] isolated targets/pool.
- [ ] Blur.
- [ ] Glow.
- [ ] Tint.
- [ ] deterministic Noise.
- [ ] RGB Split.
- [ ] spatial preview compensation.
- [ ] animated parameters.
- [ ] reference/performance tests.

Gate: M9.

## Epic 17 — Serialization/open/save

- [ ] .rhfx V1 JSON wrapper.
- [ ] serializer/parser.
- [ ] validation/limits.
- [ ] transactional Open.
- [ ] safe temp/replace Save.
- [ ] Save As asset-relative rewrite.
- [ ] migration framework.
- [ ] V1 fixture.
- [ ] corruption/failure tests.

## Epic 18 — Recovery

- [ ] LocalAppData recovery root.
- [ ] session metadata.
- [ ] dirty 30-second trigger.
- [ ] 1-second transaction quiet period.
- [ ] one write/coalescing.
- [ ] current/previous generations.
- [ ] startup restore/discard.
- [ ] 14-day stale cleanup.
- [ ] forced-crash tests.

Gate: M8.

## Epic 19 — Export

- [ ] snapshot.
- [ ] exact frame timestamps.
- [ ] export renderer state.
- [ ] full-resolution scaling.
- [ ] <=3 bounded readbacks.
- [ ] raw-video FFmpeg pipe.
- [ ] H.264/yuv420p.
- [ ] AAC/source audio mux.
- [ ] quality presets backend.
- [ ] progress/cancel.
- [ ] temp output publish.
- [ ] sync/reference tests.

Gate: M10.

## Epic 20 — Performance/stability

- [ ] all benchmark fixtures.
- [ ] Medium frame p95/p99.
- [ ] timeline stress.
- [ ] 30-minute audio.
- [ ] memory buckets.
- [ ] startup/open/save.
- [ ] build regression baselines.
- [ ] asset/renderer/audio failure stress.
- [ ] project/recovery corruption stress.

## Epic 21 — UX/design acceptance

- [ ] Inter UI/design tokens.
- [ ] dark theme.
- [ ] 1280×720.
- [ ] 100/125/150 DPI.
- [ ] tooltip/shortcut discoverability.
- [ ] 3 first-use observations.
- [ ] experienced shortcut workflow.
- [ ] 60-minute edit session.
- [ ] resolve repeated core friction.

## Epic 22 — Package/release

- [ ] release profile.
- [ ] pinned bundled FFmpeg/capability validation.
- [ ] Inter/runtime assets.
- [ ] licenses/provenance.
- [ ] logs/cache/recovery paths.
- [ ] portable ZIP.
- [ ] non-ASCII path tests.
- [ ] Windows 10/11 clean-machine smoke.
- [ ] dependency/security review.
- [ ] release checksum.
- [ ] known limitations.
- [ ] GitHub Release.

Gate: M11/M12.

## Post-MVP

Do not pull in without scope update:

- macOS/Linux;
- installer/updater/signing work beyond release need;
- SVG/video;
- parenting;
- 3D/particles/masks/motion tracking;
- scripts/expressions/plugins/nodes;
- multi-track audio;
- BPM detection;
- packed project;
- imported fonts;
- temporal effects;
- cloud/collaboration/AI generation.
