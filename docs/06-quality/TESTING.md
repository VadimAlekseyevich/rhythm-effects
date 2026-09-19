# Testing

> **Status: Accepted for MVP**

## 1. Testing layers

1. fast unit tests;
2. invariant/property tests;
3. subsystem integration tests;
4. deterministic scene/reference tests;
5. end-to-end project fixtures;
6. manual hardware/usability QA.

Core deterministic logic receives the strongest automation.

## 2. Required CI on every pull request

- cargo fmt --check;
- clippy with project-selected warning policy;
- core/unit tests;
- non-hardware integration tests;
- debug/check build for workspace;
- project-schema fixture tests;
- WGSL/shader validation through the renderer test path where practical.

CI does not require a physical audio device.

## 3. Time tests

Required:

- tick zero = GridOffsetNs;
- exact PPQ divisions;
- 120 BPM one beat = 0.5 s;
- decimal/fixed BPM;
- negative tick conversions;
- snap half-tie positive/negative;
- exact keyboard forward/back;
- frame N direct timestamp;
- long frame sequence has no accumulated drift.

## 4. Animation tests

For scalar, Vec2, LinearRgba:

- zero keys;
- one key;
- before first;
- exact key;
- between;
- after last;
- Hold;
- Linear;
- Bezier presets/custom;
- outgoing-key ownership;
- 0→360 rotation;
- negative scale;
- linear-light color midpoint;
- arbitrary seek order.

## 5. Project/command/history tests

- ID allocation;
- validation;
- add/delete/duplicate object;
- nested fresh IDs;
- key collision replace;
- one drag = one history entry;
- cancel restores before;
- key-repeat coalescing;
- redo truncation;
- 500-entry cap;
- save revision/dirty semantics;
- BPM/offset undo without rewriting ticks;
- compound Add Image from File.

## 6. Serialization tests

Keep V1 and every future shipped historical schema fixture.

Test:

- round trip;
- malformed JSON;
- truncated file;
- unknown future schema;
- duplicate IDs;
- bad references;
- NaN/invalid numeric representation;
- safety limits;
- relative/absolute assets;
- missing assets;
- safe-save write/publish failures;
- failed Open leaves current session unchanged.

## 7. Recovery tests

- unsaved project recovery;
- dirty saved project recovery;
- 30-second cadence logic;
- transaction defer;
- write coalescing;
- current corrupt -> previous fallback;
- explicit Save failure retains recovery;
- Don't Save removal;
- Restore opens dirty;
- stale cleanup.

At least one automated/integration test should simulate abnormal termination state through filesystem fixtures.

## 8. Waveform tests

Generated PCM:

- min/max;
- stereo envelope;
- 64-frame base buckets;
- partial bucket;
- mip reduction;
- visible query;
- level selection;
- short source;
- sample-rate positioning.

## 9. Audio pure tests

Without device:

- supported decode fixtures;
- mono/stereo preparation;
- resampling length/timing;
- project-time ↔ output-frame mapping;
- seek clamping;
- generation stale-anchor rejection;
- end behavior.

## 10. Audio manual/integration matrix

Before MVP:

- Windows 10 and 11;
- default built-in/wired output;
- USB audio device where available;
- Bluetooth as best-effort characterization;
- 44.1 kHz and 48 kHz sources;
- sample-rate mismatch;
- device loss/default change;
- rapid play/pause/seek;
- 30-minute playback under editor load.

Record hardware/driver/backend for sync results.

## 11. Renderer tests

Semantic/unit:

- coordinate transforms;
- anchor;
- clockwise rotation;
- preview scale parameter conversion.

GPU/reference:

- offscreen target;
- rectangle;
- ellipse;
- sRGB image;
- transparent premultiplied edge;
- text;
- each effect;
- effect order;
- Rgba16Float path;
- export readback.

Pixel/reference comparison uses tolerances where GPU differences make exact equality inappropriate.

## 12. Reference scenes

Keep deterministic scenes:

- Transform;
- Alpha Overlap;
- Image sRGB;
- Text Cyrillic;
- Easing;
- Blur/Glow;
- Five Effects;
- Missing Asset;
- Preview Scale.

At chosen ProjectTimeNs, semantic evaluated values are exact expected data.

Pixel references supplement semantic tests.

## 13. Export tests

Automated where environment has FFmpeg:

- 1080p60 short;
- 720p scale;
- 30 FPS override;
- video-only;
- AAC audio;
- cancellation;
- FFmpeg failure;
- output publication failure;
- bounded readback count.

Sync fixture:

- generated clicks;
- exact visual flashes;
- analyze multiple timestamps including long duration.

## 14. UI logic tests

Extract/test where reasonable:

- focus routing;
- physical-key shortcut dispatch;
- keyframe drag tick resolution;
- box-selection math;
- timeline range query;
- viewport hit testing;
- command enable/disable;
- property keyframing behavior.

Avoid brittle screenshot tests for every ordinary control.

## 15. Manual usability QA

Check:

- 100/125/150% DPI;
- 1280×720 constrained layout;
- hit targets;
- focus;
- Russian/English keyboard layout;
- Esc cancel;
- numeric invalid intermediate input;
- timeline zoom/pan;
- direct animated transform;
- missing font/asset;
- long-session fatigue.

## 16. GPU matrix

Before MVP, exercise normal workflow on available representatives of:

- NVIDIA;
- AMD;
- Intel integrated.

A vendor unavailable to the project cannot block development indefinitely, but broader public release requires collecting real reports before claiming support confidence.

## 17. Fuzz/property testing

High-value targets:

- time conversion/snap;
- JSON/migration parser;
- randomized valid command sequences preserving Project invariants.

Fuzzing may run scheduled/manual rather than every PR.

## 18. Regression rule

Every deterministic core bug receives a regression test when practical.

A bug that cannot be automated should receive a reproducible fixture/manual case.

## 19. Release smoke test

On a clean packaged Windows environment:

1. launch;
2. new project;
3. import audio;
4. set BPM/offset;
5. create rectangle;
6. animate Position;
7. add image/text;
8. add/ease/effect;
9. save;
10. close/reopen;
11. undo/redo;
12. force/check recovery flow in a separate run;
13. export;
14. play output externally.

## 20. Definition of Done

Testing is MVP-ready when deterministic core systems are automated, V1/recovery fixtures exist, hardware QA is recorded, export sync is verified, and the clean-package smoke test is repeatable.
