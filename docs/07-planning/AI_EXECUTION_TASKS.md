# AI Execution Tasks

> **Status: Accepted execution plan**
>
> Purpose: each checkbox is intentionally scoped so a capable coding model can normally implement it in one focused request, including tests for that one change.
>
> Canonical behavior remains in the subsystem specs. If a task conflicts with an Accepted spec, the spec wins and the task must be corrected.
>
> Working rule for every coding request: read the referenced canonical subsystem docs first, change only the requested slice, add/update tests, run fmt/clippy/tests, and do not opportunistically pull later tasks forward.

## How to use

Work strictly top-to-bottom unless a task is explicitly blocked by an external validation. One task should usually map to one branch/commit or one tightly scoped model request. A task is complete only when its local acceptance behavior/tests pass.


## M1 — Foundation

- [x] **AI-001** — Create the root Cargo workspace manifest with resolver=3 and members rhythm_core, rhythm_engine, rhythm_app.
- [x] **AI-002** — Add rust-toolchain.toml pinned to Rust 1.98.1 with rustfmt and clippy components.
- [x] **AI-003** — Add a repository .gitignore for Cargo/Windows development artifacts.
- [x] **AI-004** — Create crates/rhythm_core as a library crate with workspace lint/package inheritance.
- [x] **AI-005** — Create crates/rhythm_engine as a library crate that depends only on rhythm_core.
- [x] **AI-006** — Create crates/rhythm_app as a binary crate that depends on rhythm_core and rhythm_engine.
- [x] **AI-007** — Add the accepted initial workspace dependency versions for winit, wgpu, egui, egui-winit, egui-wgpu and pollster to the root manifest.
- [x] **AI-008** — Add tracing and tracing-subscriber dependencies to rhythm_app.
- [x] **AI-009** — Implement tracing initialization with RUST_LOG/EnvFilter fallback to info.
- [x] **AI-010** — Implement a minimal winit 0.30 ApplicationHandler that creates one Rhythm Effects window on resume.
- [x] **AI-011** — Handle CloseRequested by exiting the winit event loop.
- [x] **AI-012** — Log window creation, resize and application shutdown events.
- [x] **AI-013** — Add one rhythm_core smoke unit test so cargo test proves the workspace test path.
- [x] **AI-014** — Add a Windows GitHub Actions workflow that runs fmt, clippy, test and workspace build.
- [x] **AI-015** — Run cargo metadata and fix all workspace-manifest errors.
- [x] **AI-016** — Run cargo fmt --check and fix formatting.
- [x] **AI-017** — Run cargo clippy --workspace --all-targets -- -D warnings and fix all warnings.
- [x] **AI-018** — Run cargo test --workspace and fix failures.
- [x] **AI-019** — Run cargo run -p rhythm_app and verify one native window opens/closes cleanly.
- [ ] **AI-020** — Record the first local build baseline required by PERFORMANCE.md. **Blocked on local reference-machine measurement; run measure_build_baseline.bat.**

## M2 — Render/UI shell

- [x] **AI-021** — Create a renderer module in rhythm_engine without exposing wgpu types to rhythm_core.
- [x] **AI-022** — Initialize one wgpu Instance/Adapter/Device/Queue compatible with the application window.
- [x] **AI-023** — Create and configure the wgpu window surface and handle zero-size/minimized windows.
- [x] **AI-024** — Add resize handling that reconfigures the surface only for non-zero dimensions.
- [x] **AI-025** — Create an offscreen Rgba16Float composition texture at a fixed 1920x1080 test size.
- [x] **AI-026** — Add a render pass that clears the offscreen composition texture to opaque black.
- [x] **AI-027** — Integrate egui Context and egui-winit State with the winit event loop.
- [x] **AI-028** — Integrate egui-wgpu rendering using the same wgpu Device/Queue.
- [x] **AI-029** — Render the accepted five-region editor shell using placeholder panels only.
- [x] **AI-030** — Apply the accepted dark theme and initial Inter-like typography sizing using egui defaults until bundled Inter is added.
- [x] **AI-031** — Expose a small diagnostics panel with adapter name, backend, window size and frame time.
- [x] **AI-032** — Display the offscreen composition texture inside the viewport panel.
- [x] **AI-033** — Implement viewport panel resize without changing composition dimensions.
- [x] **AI-034** — Implement Auto/Full/Half/Quarter preview-quality state in EditorSession placeholder state.
- [x] **AI-035** — Add a renderer capability check for Rgba16Float render-attachment support.
- [x] **AI-036** — Add a startup error path when no compatible GPU adapter/device can be created.
- [x] **AI-037** — Add a manual reference check for top-left composition orientation.
- [ ] **AI-038** — Verify the shell at 1280x720 and normal 1920x1080 desktop sizes. **Blocked on local visual Windows validation; follow `docs/06-quality/M2_MANUAL_VALIDATION.md`.**

## M3 — Domain/time/project/animation/history

- [x] **AI-039** — Implement typed project-local ObjectId, AssetId, EffectId and KeyframeId u64 newtypes with zero reserved.
- [x] **AI-040** — Implement a monotonic project ID allocator and unit tests for non-reuse.
- [x] **AI-041** — Implement MusicalTick(i64), ProjectTimeNs(i64), DurationNs(u64), GridOffsetNs(i64), BpmMicros(u64), SampleRate(u32), AudioFramePosition(u64).
- [x] **AI-042** — Implement validated BpmMicros construction for the accepted 1..1000 BPM range.
- [x] **AI-043** — Implement exact BPM string parse/format helpers including decimal micro-BPM precision tests.
- [x] **AI-044** — Define PPQ=960 in one canonical rhythm_core location.
- [x] **AI-045** — Implement BeatDivision with exactly the accepted MVP parts-per-beat set.
- [x] **AI-046** — Implement ticks_per_step for every accepted BeatDivision and tests.
- [x] **AI-047** — Implement rational FrameRate with normalized positive numerator/denominator.
- [x] **AI-048** — Implement TimeSignature and the default 4/4 value.
- [x] **AI-049** — Implement TempoMap V1 with one initial tempo segment and GridOffsetNs.
- [x] **AI-050** — Implement MusicalTick to ProjectTimeNs conversion using i128 intermediates.
- [x] **AI-051** — Implement ProjectTimeNs to continuous musical position conversion without hidden rounding.
- [x] **AI-052** — Implement nearest-grid snapping with exact half ties resolving later/right.
- [x] **AI-053** — Implement floor/ceil grid conversion helpers and negative-tick tests.
- [x] **AI-054** — Implement frame-index to ProjectTimeNs conversion derived independently from N.
- [x] **AI-055** — Implement LinearRgba with finite-value validation and normalized alpha.
- [x] **AI-056** — Implement Vec2/domain validation needed by project transforms.
- [x] **AI-057** — Implement the accepted TransformAnimation semantic fields.
- [x] **AI-058** — Implement ProjectSettings defaulting to 1920x1080, 60 FPS, 10 seconds and black background.
- [x] **AI-059** — Implement the Project root, metadata, Composition and Object structures for schema V1.
- [x] **AI-060** — Implement RectangleObject with animated size/fill/corner radius.
- [x] **AI-061** — Implement EllipseObject with animated size/fill.
- [x] **AI-062** — Implement ImageObject storing only AssetId.
- [x] **AI-063** — Implement TextObject static text/font/font-size/alignment plus animated color.
- [x] **AI-064** — Implement FontReference family/weight/style semantic types.
- [x] **AI-065** — Implement the five typed EffectKind variants and their parameter structs.
- [x] **AI-066** — Implement AssetRecord/AssetKind/AssetSource and AudioTrack semantic structures.
- [x] **AI-067** — Implement Project validation for IDs, dimensions, duration, numeric finiteness and references.
- [x] **AI-068** — Implement Animated<T> and Keyframe<T> with sorted unique MusicalTick invariant.
- [x] **AI-069** — Implement Hold interpolation.
- [x] **AI-070** — Implement Linear interpolation for f32 and Vec2.
- [x] **AI-071** — Implement LinearRgba interpolation in linear-light values.
- [x] **AI-072** — Implement scalar degree rotation interpolation without shortest-path normalization.
- [x] **AI-073** — Implement CubicBezier easing representation with x/y handles constrained to 0..1 for MVP.
- [x] **AI-074** — Implement deterministic cubic-Bezier timing evaluation and endpoint tests.
- [x] **AI-075** — Implement before-first/exact-key/after-last animation behavior.
- [x] **AI-076** — Implement binary-search segment lookup for arbitrary evaluation order.
- [x] **AI-077** — Implement EvaluatedScene containing renderer-ready evaluated object state only.
- [x] **AI-078** — Implement ProjectEditor as the only normal creative mutation boundary.
- [x] **AI-079** — Implement explicit EditCommand intent types for first object/property/keyframe mutations.
- [x] **AI-080** — Implement HistoryEntry before/after payloads without whole-Project snapshots.
- [x] **AI-081** — Implement undo/redo cursor and redo-branch truncation.
- [x] **AI-082** — Implement begin/update/commit/cancel transaction support.
- [x] **AI-083** — Implement dirty/saved revision semantics.
- [x] **AI-084** — Implement the 500 logical history-entry capacity.
- [x] **AI-085** — Add tests proving one drag transaction becomes one undo entry.
- [x] **AI-086** — Add tests proving BPM and grid-offset edits preserve MusicalTick values.

## M4 — Audio

- [x] **AI-087** — Add Symphonia, CPAL and Rubato dependencies with only required features.
- [x] **AI-088** — Implement audio-file probing and decode result/error types in rhythm_engine.
- [x] **AI-089** — Implement WAV decode fixture support.
- [ ] **AI-090** — Implement MP3 decode fixture support.
- [ ] **AI-091** — Implement FLAC decode fixture support.
- [ ] **AI-092** — Implement OGG/Vorbis decode fixture support.
- [ ] **AI-093** — Implement AAC/M4A support for the pinned Symphonia feature set or return a clear unsupported error.
- [x] **AI-094** — Implement mono/stereo decoded f32 interleaved PCM representation.
- [ ] **AI-095** — Reject unsupported multichannel input instead of undocumented downmix.
- [ ] **AI-096** — Implement background output-rate resampling with Rubato.
- [ ] **AI-097** — Implement immutable stereo PlaybackBuffer and release temporary source PCM after waveform handoff.
- [ ] **AI-098** — Initialize the system-default CPAL output device and stream config.
- [ ] **AI-099** — Implement realtime callback copying prepared PCM without allocation, decode, resample or file I/O.
- [ ] **AI-100** — Implement play/pause/end state transitions.
- [ ] **AI-101** — Implement paused seek by updating EditorSession playhead only.
- [ ] **AI-102** — Implement playing seek using a new playback generation and callback cursor switch.
- [ ] **AI-103** — Implement playback-generation atomics and stale-anchor rejection.
- [ ] **AI-104** — Publish CPAL playback timestamp plus ProjectTimeNs clock anchors from the callback.
- [ ] **AI-105** — Implement editor-side audible ProjectTimeNs estimation using the stream clock.
- [ ] **AI-106** — Implement documented output-frame/latency fallback when backend timestamps are unusable.
- [ ] **AI-107** — Implement gain application with finite scalar validation.
- [ ] **AI-108** — Handle audio-device/stream error by stopping playback without mutating Project.
- [ ] **AI-109** — Add diagnostics for device/config/generation/clock/error count/memory.
- [ ] **AI-110** — Run 44.1k source -> 48k output integration test.
- [ ] **AI-111** — Run rapid play/pause/seek manual stress test.
- [ ] **AI-112** — Run the initial 10-minute no-cumulative-drift sync test.

## M5 — Waveform and beat timeline

- [ ] **AI-113** — Implement 64-source-frame WavePeak min/max base aggregation.
- [ ] **AI-114** — Implement stereo envelope combination across both channels.
- [ ] **AI-115** — Implement waveform mip pyramid by pairwise peak reduction.
- [ ] **AI-116** — Implement final partial waveform bucket behavior.
- [ ] **AI-117** — Implement immutable waveform result keyed by AssetId generation.
- [ ] **AI-118** — Implement background waveform worker request/result messages.
- [ ] **AI-119** — Implement visible-range waveform slice query.
- [ ] **AI-120** — Implement mip-level choice from visible time range and pixel width.
- [ ] **AI-121** — Render the waveform as a batched egui mesh/shape rather than one widget per bucket.
- [ ] **AI-122** — Implement one shared TimelineTransform for project-time to x and inverse mapping.
- [ ] **AI-123** — Implement timeline ruler row.
- [ ] **AI-124** — Render bar lines using TimeSignature and TempoMap.
- [ ] **AI-125** — Render beat lines with medium visual priority.
- [ ] **AI-126** — Render subdivision lines for the current BeatDivision.
- [ ] **AI-127** — Hide labels before grid lines become visually overcrowded.
- [ ] **AI-128** — Implement continuous playhead ruler click/drag seek.
- [ ] **AI-129** — Implement Left/Right current-grid playhead stepping.
- [ ] **AI-130** — Implement Ctrl+Left/Right one-beat stepping.
- [ ] **AI-131** — Implement Ctrl+Shift+Left/Right one-bar stepping.
- [ ] **AI-132** — Implement [ and ] current authoring-grid changes.
- [ ] **AI-133** — Implement Ctrl+wheel timeline zoom around mouse position.
- [ ] **AI-134** — Implement Shift+wheel horizontal timeline pan.
- [ ] **AI-135** — Implement middle-mouse horizontal timeline pan.
- [ ] **AI-136** — Implement object-row and property-row data models for the timeline UI.
- [ ] **AI-137** — Implement vertical visible-row virtualization.
- [ ] **AI-138** — Implement horizontal visible-MusicalTick keyframe query API.
- [ ] **AI-139** — Implement timeline empty states for no audio, no BPM and no objects.
- [ ] **AI-140** — Add a generated 10-minute waveform/zoom benchmark fixture.

## M6 — Keyframe timeline editing

- [ ] **AI-141** — Render keyframe diamonds with at least 18x18 logical hit boxes.
- [ ] **AI-142** — Implement single keyframe selection by stable KeyframeId.
- [ ] **AI-143** — Implement Ctrl-click keyframe toggle selection.
- [ ] **AI-144** — Implement empty-canvas box selection.
- [ ] **AI-145** — Implement Ctrl box-add/toggle selection semantics.
- [ ] **AI-146** — Implement K on a static focused property to create the first key at nearest grid and move playhead there.
- [ ] **AI-147** — Implement K on an existing resolved key to remove that key.
- [ ] **AI-148** — Implement K on an animated property without a current key to create one from evaluated value.
- [ ] **AI-149** — Implement pointer keyframe drag through continuous time -> musical position -> current-grid snap.
- [ ] **AI-150** — Implement Esc cancellation for a keyframe drag transaction.
- [ ] **AI-151** — Implement multi-key drag using one anchor key and identical integer tick delta.
- [ ] **AI-152** — Implement occupied-target collision so incoming/moved key wins and replaced key is stored for undo.
- [ ] **AI-153** — Implement Delete for selected keyframes as one compound history entry.
- [ ] **AI-154** — Implement Alt+Left/Right selected-key movement by exactly one current grid step.
- [ ] **AI-155** — Implement Ctrl+Alt+Left/Right selected-key movement by exactly one beat.
- [ ] **AI-156** — Implement copy packet with property compatibility, values, easing and relative tick offsets.
- [ ] **AI-157** — Implement paste anchored at nearest-grid playhead with fresh KeyframeIds.
- [ ] **AI-158** — Implement Ctrl+D duplicate of selected keys with duplicates remaining selected.
- [ ] **AI-159** — Implement Hold/Linear/easing preset context actions for selected outgoing segments.
- [ ] **AI-160** — Implement Follow Playhead toggle defaulting OFF and edge-follow behavior.
- [ ] **AI-161** — Add the 10,000-key timeline stress fixture and verify no full-project draw scan.

## M7 — Editor core

- [ ] **AI-162** — Implement CPU inverse-transform hit testing for Rectangle.
- [ ] **AI-163** — Implement CPU inverse-transform hit testing for Ellipse.
- [ ] **AI-164** — Implement CPU bounds hit testing for Image.
- [ ] **AI-165** — Implement CPU layout-bounds hit testing hook for Text.
- [ ] **AI-166** — Select the topmost visible unlocked viewport object on click.
- [ ] **AI-167** — Implement Ctrl-click object toggle selection in the viewport.
- [ ] **AI-168** — Implement viewport box selection from empty space.
- [ ] **AI-169** — Implement middle-mouse viewport pan.
- [ ] **AI-170** — Implement mouse-wheel viewport zoom around pointer.
- [ ] **AI-171** — Implement Fit Composition and Frame Selection actions.
- [ ] **AI-172** — Render selected-object bounds and center anchor marker as editor-only overlays.
- [ ] **AI-173** — Implement Position direct drag as one transaction.
- [ ] **AI-174** — Implement Shift axis constraint for Position drag.
- [ ] **AI-175** — Implement Scale corner handles for a single object.
- [ ] **AI-176** — Implement Shift uniform Scale constraint.
- [ ] **AI-177** — Implement Rotation handle around Anchor.
- [ ] **AI-178** — Implement Shift 15-degree Rotation snapping.
- [ ] **AI-179** — Implement multi-object move by shared composition-space delta.
- [ ] **AI-180** — Implement animated direct-transform edits using the accepted property-keyframing behavior.
- [ ] **AI-181** — Implement the Object list with visible/locked/name state.
- [ ] **AI-182** — Synchronize Object list, Viewport and Inspector selection.
- [ ] **AI-183** — Implement standard Inspector animatable property row.
- [ ] **AI-184** — Implement numeric edit commit on Enter/focus-loss and Escape restore.
- [ ] **AI-185** — Implement static property edit to base_value.
- [ ] **AI-186** — Implement animated off-key property edit to create/update nearest-grid key and move playhead.
- [ ] **AI-187** — Implement final-key removal returning the property to static using removed-key value.
- [ ] **AI-188** — Implement multi-selection mixed-value transform rows.
- [ ] **AI-189** — Implement Rectangle Inspector controls.
- [ ] **AI-190** — Implement Ellipse Inspector controls.
- [ ] **AI-191** — Implement Image source/intrinsic-dimensions/Relink Inspector controls.
- [ ] **AI-192** — Implement Text content/font/weight/style/size/alignment/color Inspector controls.
- [ ] **AI-193** — Implement Effect stack Inspector with enable/reorder/remove.
- [ ] **AI-194** — Implement Add Effect shallow searchable popover.
- [ ] **AI-195** — Implement physical-key shortcut dispatcher for P/S/R/O/K.
- [ ] **AI-196** — Implement Space Play/Pause without Space viewport-pan behavior.
- [ ] **AI-197** — Implement Ctrl+K command search surface.
- [ ] **AI-198** — Implement focus routing so active text/numeric controls suppress editor letter shortcuts.
- [ ] **AI-199** — Implement Ctrl+N/O/S/Shift+S/Z/Shift+Z/Y/C/V/D/A and Delete command bindings.
- [ ] **AI-200** — Add rfd native file-dialog dependency and Open/Save/Import dialog wrapper.
- [ ] **AI-201** — Implement PNG image decode worker.
- [ ] **AI-202** — Implement JPEG image decode worker.
- [ ] **AI-203** — Implement WebP image decode worker.
- [ ] **AI-204** — Implement decoded-image generation validation before GPU upload.
- [ ] **AI-205** — Implement image GPU texture caching by AssetId generation.
- [ ] **AI-206** — Implement relative asset paths for files under the saved project directory.
- [ ] **AI-207** — Implement absolute paths for external files.
- [ ] **AI-208** — Implement duplicate-path AssetRecord reuse within the project/session.
- [ ] **AI-209** — Implement missing-asset runtime state and Relink while keeping AssetId stable.
- [ ] **AI-210** — Block deletion of referenced AssetRecords.
- [ ] **AI-211** — Implement OS file drag/drop for Add Image from File as one compound edit.
- [ ] **AI-212** — Integrate cosmic-text FontSystem as one long-lived text resource.
- [ ] **AI-213** — Integrate glyphon using the existing wgpu Device/Queue.
- [ ] **AI-214** — Bundle Inter as the deterministic composition fallback family.
- [ ] **AI-215** — Implement system font family enumeration/cache.
- [ ] **AI-216** — Implement Latin/Cyrillic text shaping.
- [ ] **AI-217** — Implement explicit multiline text layout.
- [ ] **AI-218** — Implement left/center/right text alignment.
- [ ] **AI-219** — Implement deterministic text local bounds for anchor/hit testing.
- [ ] **AI-220** — Cache text layout by content/font/size/alignment and avoid reshape for pure transform/color changes.
- [ ] **AI-221** — Show a visible missing-requested-font state while rendering Inter fallback.
- [ ] **AI-222** — Implement Ease In, Ease Out and Ease In-Out preset canonical Bezier values.
- [ ] **AI-223** — Implement one-segment custom timing Curve Editor in the timeline area.
- [ ] **AI-224** — Constrain custom curve x/y handles to 0..1.
- [ ] **AI-225** — Make one handle drag one undoable transaction with Esc cancel.

## M8 — Save/load/recovery

- [ ] **AI-226** — Implement ProjectFileV1 root wrapper with schema_version=1 and created_with_version.
- [ ] **AI-227** — Implement serde JSON serialization for all schema V1 semantic types.
- [ ] **AI-228** — Implement .rhfx UTF-8 JSON parsing into a candidate project.
- [ ] **AI-229** — Implement load-time semantic validation before active-project replacement.
- [ ] **AI-230** — Implement rejection of unknown newer schema versions.
- [ ] **AI-231** — Implement sequential migration framework even though only V1 initially exists.
- [ ] **AI-232** — Commit a minimal V1 .rhfx fixture.
- [ ] **AI-233** — Add semantic JSON round-trip equality tests.
- [ ] **AI-234** — Implement the documented project-file safety limits.
- [ ] **AI-235** — Implement transactional Open so failure leaves current Project untouched.
- [ ] **AI-236** — Implement explicit Save serialization to a sibling temporary file.
- [ ] **AI-237** — Flush/close the temporary file before publication.
- [ ] **AI-238** — Implement Windows-safe replacement/publication preserving the previous known-good file on failure.
- [ ] **AI-239** — Only update saved_revision after successful publication.
- [ ] **AI-240** — Implement Save As so canonical path changes only after successful publication.
- [ ] **AI-241** — Recalculate eligible relative AssetSource paths after successful Save As.
- [ ] **AI-242** — Implement LocalAppData RhythmEffects/recovery directory resolution.
- [ ] **AI-243** — Implement stable editing-session recovery identifier/metadata.
- [ ] **AI-244** — Implement dirty-project recovery scheduling at max 30-second interval.
- [ ] **AI-245** — Defer recovery while an edit transaction is active and wait at least 1 second after it ends.
- [ ] **AI-246** — Coalesce recovery requests so at most one write is in flight.
- [ ] **AI-247** — Keep current and previous successful recovery generations.
- [ ] **AI-248** — Implement startup recovery discovery.
- [ ] **AI-249** — Implement Restore opening recovered state as dirty without silently overwriting canonical .rhfx.
- [ ] **AI-250** — Implement Discard deleting only selected recovery state.
- [ ] **AI-251** — Implement corrupt-current -> previous recovery fallback.
- [ ] **AI-252** — Delete obsolete recovery after successful explicit Save/clean close.
- [ ] **AI-253** — Delete recovery after explicit user Don't Save confirmation.
- [ ] **AI-254** — Implement 14-day stale recovery cleanup with canonical-newer safeguards.
- [ ] **AI-255** — Add forced-crash filesystem/integration recovery tests.

## M9 — Effects and visual parity

- [ ] **AI-256** — Implement renderer temporary texture pool keyed by size/format/usage.
- [ ] **AI-257** — Implement isolated object rendering for multipass effects.
- [ ] **AI-258** — Implement ordered effect-chain execution.
- [ ] **AI-259** — Implement separable Blur with radius 0..128 composition pixels.
- [ ] **AI-260** — Implement preview-scale compensation for Blur radius.
- [ ] **AI-261** — Implement Glow threshold mask, blur, color and additive intensity.
- [ ] **AI-262** — Implement preview-scale compensation for Glow radius.
- [ ] **AI-263** — Implement Tint color/amount shader and identity-at-zero test.
- [ ] **AI-264** — Implement deterministic Noise from seed/evolution/pixel coordinates without mutable RNG state.
- [ ] **AI-265** — Implement Noise amount and size semantics.
- [ ] **AI-266** — Implement RGB Split amount/angle channel offsets.
- [ ] **AI-267** — Test RGB Split alpha behavior for transparent edges.
- [ ] **AI-268** — Animate every documented effect parameter through Animated<T>.
- [ ] **AI-269** — Skip disabled effects without unnecessary pass/resource cost.
- [ ] **AI-270** — Add visual reference scenes for all five effects.
- [ ] **AI-271** — Add preview Full/Half/Quarter effect-semantic parity tests.
- [ ] **AI-272** — Run Text Heavy and Effects Heavy performance fixtures.

## M10 — Export

- [ ] **AI-273** — Add export-job immutable Project snapshot creation.
- [ ] **AI-274** — Resolve and validate all required image/audio/font resources before frame 0.
- [ ] **AI-275** — Implement exact export ProjectTimeNs from frame index and rational output FPS.
- [ ] **AI-276** — Implement full-composition default export range.
- [ ] **AI-277** — Implement output resolution scaling while preserving composition aspect ratio.
- [ ] **AI-278** — Create export renderer state that reuses creative shader/evaluation semantics without mutating preview state.
- [ ] **AI-279** — Render one full-resolution export frame into an offscreen target.
- [ ] **AI-280** — Convert final frame to standard SDR RGBA/BGRA readback representation.
- [ ] **AI-281** — Implement a bounded pool of at most three GPU readback/frame buffers.
- [ ] **AI-282** — Spawn bundled/resolved FFmpeg through structured process arguments, never cmd.exe shell interpolation.
- [ ] **AI-283** — Pipe raw video frames to FFmpeg stdin.
- [ ] **AI-284** — Configure MP4/H.264/yuv420p output.
- [ ] **AI-285** — Pass original primary audio asset to FFmpeg for AAC mux when present.
- [ ] **AI-286** — Trim audio to composition duration and permit video tail after audio ends.
- [ ] **AI-287** — Support video-only export when Project has no audio.
- [ ] **AI-288** — Implement Fast/Balanced/High encoder-quality backend mapping.
- [ ] **AI-289** — Implement current-frame/total/percent/elapsed export progress.
- [ ] **AI-290** — Implement cooperative export cancellation and owned FFmpeg process termination.
- [ ] **AI-291** — Write export to a temporary destination and publish final path only after successful FFmpeg completion.
- [ ] **AI-292** — Capture bounded FFmpeg stderr and map common failures to structured ExportError stages.
- [ ] **AI-293** — Create generated click/flash A/V sync fixture.
- [ ] **AI-294** — Validate short, 1-minute, 5-minute and 10-minute export sync.
- [ ] **AI-295** — Test 1080p60, 720p and 30 FPS output variants.
- [ ] **AI-296** — Test cancelled/failed export leaves no successful-looking partial output.

## M11 — Quality

- [ ] **AI-297** — Create Empty benchmark fixture.
- [ ] **AI-298** — Create Basic Rhythm benchmark fixture.
- [ ] **AI-299** — Create Medium Motion benchmark fixture with 100 objects and 1,000 keyframes.
- [ ] **AI-300** — Create Text Heavy benchmark fixture.
- [ ] **AI-301** — Create Effects Heavy benchmark fixture.
- [ ] **AI-302** — Create Timeline Stress fixture with 500 objects and 10,000+ keyframes.
- [ ] **AI-303** — Create Long Audio generated fixture.
- [ ] **AI-304** — Implement diagnostics overlay p50/p95 frame timing.
- [ ] **AI-305** — Add UI/timeline CPU timing instrumentation.
- [ ] **AI-306** — Add animation evaluation timing instrumentation.
- [ ] **AI-307** — Add renderer CPU timing/pass/draw/temporary-target counters.
- [ ] **AI-308** — Add background worker queue/job counters.
- [ ] **AI-309** — Add major memory-bucket estimates.
- [ ] **AI-310** — Measure Medium fixture p95/p99 against PERFORMANCE.md gates.
- [ ] **AI-311** — Run 30-minute audio playback stress under Medium editor interaction.
- [ ] **AI-312** — Measure warm/cold startup on the recorded reference machine.
- [ ] **AI-313** — Measure Medium .rhfx parse/validate/open time.
- [ ] **AI-314** — Measure Medium explicit Save/recovery snapshot hitching.
- [ ] **AI-315** — Record clean debug/release and app/core incremental build baselines.
- [ ] **AI-316** — Add regression documentation for any >20% incremental or >15% clean-build regression.
- [ ] **AI-317** — Run Windows 100%, 125%, 150% DPI manual QA.
- [ ] **AI-318** — Run 1280x720 constrained-layout QA.
- [ ] **AI-319** — Run NVIDIA/AMD/Intel normal-workflow GPU checks as hardware becomes available.
- [ ] **AI-320** — Run first-use observation with three creative-software users.
- [ ] **AI-321** — Run one 60-minute real editing session and record UX friction.
- [ ] **AI-322** — Fix repeated core-flow usability defects before MVP candidate.

## M12 — Package/release

- [ ] **AI-323** — Create the Windows release Cargo profile and record its build-time impact.
- [ ] **AI-324** — Choose and pin the distributable FFmpeg Windows build that satisfies project licensing requirements.
- [ ] **AI-325** — Implement bundled FFmpeg path resolution and capability/version check.
- [ ] **AI-326** — Verify bundled FFmpeg exposes the required H.264 encoder and AAC support.
- [ ] **AI-327** — Create Windows LocalAppData paths for settings/logs/recovery/cache.
- [ ] **AI-328** — Implement bounded rotating release logs.
- [ ] **AI-329** — Include app version, OS, GPU and relevant audio configuration in diagnostics logs.
- [ ] **AI-330** — Add dependency/license provenance documentation for FFmpeg, Inter and redistributed assets.
- [ ] **AI-331** — Create the portable Windows x86_64 ZIP layout.
- [ ] **AI-332** — Verify the package works without Rust, Visual Studio developer shell or FFmpeg on PATH.
- [ ] **AI-333** — Run project paths containing spaces.
- [ ] **AI-334** — Run Cyrillic/non-ASCII user/project path smoke tests.
- [ ] **AI-335** — Run missing/quarantined FFmpeg error UX test.
- [ ] **AI-336** — Run low-disk/read-only export/save error tests.
- [ ] **AI-337** — Run Windows 10 clean-machine smoke test.
- [ ] **AI-338** — Run Windows 11 clean-machine smoke test.
- [ ] **AI-339** — Run full release scenario from audio import through external playback of exported MP4.
- [ ] **AI-340** — Run dependency/advisory review and document reachable release risks.
- [ ] **AI-341** — Generate release SHA-256 checksum.
- [ ] **AI-342** — Publish the MVP GitHub Release ZIP with known limitations.

## Completion rule

Do not collapse several unchecked items into a large rewrite merely because a model can generate a lot of code. The point of this file is bounded reviewable changes with a known contract. If one item repeatedly needs multiple independent architectural choices, split that item before implementing it.
