# Time Model

> **Status: Draft**
>
> This is a critical project contract. Audio, timeline, animation, serialization, snapping, and export must use these definitions consistently.

## 1. Core principle

Rhythm Effects has multiple time domains.

They must not be conflated.

The most important authoring invariant is:

> **Authored keyframes are stored in integer musical time, never as arbitrary floating-point seconds.**

Animation evaluation may happen at any continuous time between keyframes.

---

## 2. Time domains

### 2.1. Audio sample/frame position

Represents progress through audio playback.

Conceptually:

~~~rust
AudioFramePosition(u64)
~~~

One audio frame contains one sample per channel.

Its conversion to seconds requires the relevant sample rate.

### 2.2. Project time

Continuous timeline time used to connect audio, musical time, and video time.

Conceptually represented transiently as high-precision seconds:

~~~rust
ProjectTimeSeconds(f64)
~~~

This is an interchange/evaluation representation, not the persisted identity of a keyframe.

Do not compare raw f64 values for keyframe identity.

### 2.3. Musical time

Canonical authoring coordinate.

~~~rust
MusicalTick(i64)
~~~

Tick 0 corresponds to the configured musical grid origin.

Negative ticks may be allowed so a track can contain pickup/pre-roll before the first downbeat.

### 2.4. Video frame time

Export coordinate.

~~~rust
FrameIndex(u64)
FrameRate { numerator, denominator }
~~~

Frame timestamp is derived from integer frame index and rational frame rate.

Do not accumulate export time with repeated floating-point addition.

---

## 3. PPQ

MVP proposal:

**PPQ = 960 ticks per quarter note.**

Reasons:

- exact integer division for common powers-of-two subdivisions;
- exact support for triplet subdivision;
- enough resolution for fine rhythmic editing;
- small integer values;
- common concept in sequencer/MIDI-style timing.

Examples at 4/4:

| Musical duration | Ticks |
|---|---:|
| Quarter note | 960 |
| Eighth | 480 |
| Sixteenth | 240 |
| Thirty-second | 120 |
| Sixty-fourth | 60 |
| Quarter-note triplet subdivision unit (1/3 beat) | 320 |
| Eighth-note triplet unit (1/6 beat) | 160 |

Terminology in UI must be made unambiguous because "1/4" can mean note value or beat subdivision depending on context.

Internally, APIs should use explicit division ratios rather than ambiguous strings.

---

## 4. Master musical lattice

All keyframes live on the PPQ integer lattice.

This is the permanent musical grid.

The user's selected authoring subdivision is a subset of this lattice.

Example:

~~~text
PPQ lattice:
|.|.|.|.|.|.|.|.|.|.|.|.|.|.|.|.|

authoring grid = 1/8:
|.......|.......|.......|.......|

existing fine-grid keyframes:
|...◆...|.......|.◆.....|.......|
~~~

Changing the current authoring subdivision does not move, quantize, or invalidate existing keyframes.

They remain valid because they still occupy integer musical ticks.

---

## 5. Authoring grid

A GridResolution defines valid positions for creation/movement during a given edit.

Conceptually:

~~~rust
struct GridResolution {
    ticks_per_step: i64,
}
~~~

Only resolutions that divide the PPQ lattice exactly should be exposed in MVP.

Potential choices:

- beat;
- 1/2 beat;
- 1/4 beat;
- 1/8 beat;
- 1/16 beat;
- 1/32 beat;
- triplets;
- other exact supported divisions.

The UI naming scheme is finalized in product/timeline docs.

---

## 6. Keyframe invariant

Persisted keyframe:

~~~rust
struct Keyframe<T> {
    id: KeyframeId,
    tick: MusicalTick,
    value: T,
    interpolation: Interpolation,
}
~~~

No persisted keyframe timestamp in seconds is required for identity.

Derived absolute time is recomputed from:

- tick;
- tempo map;
- grid origin/offset.

---

## 7. BPM representation

MVP user-facing BPM may contain decimals.

Core requirements:

- finite;
- positive;
- bounded to a sensible product range;
- parsed/validated once;
- never allow NaN/Infinity into project state.

Initial implementation may store validated BPM as f64 behind a dedicated newtype.

A later fixed-point representation remains possible if tests reveal reproducibility problems.

Do not expose raw f64 throughout the codebase.

Concept:

~~~rust
struct Bpm(f64);
~~~

Construction goes through validation.

---

## 8. Grid offset

Grid offset defines where MusicalTick(0) lands in project/audio time.

Concept:

~~~text
project time of tick 0 = grid_offset
~~~

The exact persisted representation should be device-independent.

Preferred MVP representation:

- signed integer microseconds or nanoseconds; or
- another explicit fixed-resolution project-time integer.

Do not store offset in output-device samples because the user's audio device/sample rate can change.

Final fixed unit is decided during implementation spike; the API must hide it behind GridOffset.

---

## 9. Single-tempo MVP

MVP UI supports one BPM value.

Core representation should still use a TempoMap abstraction:

~~~rust
TempoMap
└─ TempoSegment {
     start_tick,
     bpm,
     meter
   }
~~~

For MVP there is normally one segment beginning at tick 0.

Why abstract now:

- conversion API belongs behind one object;
- future tempo changes should not require rewriting every caller;
- minimal additional complexity.

Do not build tempo-change editing UI for MVP.

---

## 10. Time signature

MVP assumes 4/4 unless product scope later promotes meter editing.

Still represent meter explicitly where bar/beat conversion needs it.

~~~rust
TimeSignature {
    numerator: u8,
    denominator: u8,
}
~~~

Do not bake "four beats per bar" into generic tick math.

---

## 11. Conversion formulas: constant tempo

For a 4/4-style quarter-note beat and PPQ ticks:

~~~text
seconds_per_quarter = 60 / BPM
seconds_per_tick = seconds_per_quarter / PPQ

time_seconds(tick) =
    grid_offset_seconds +
    tick * seconds_per_tick
~~~

Inverse:

~~~text
tick_float =
    (time_seconds - grid_offset_seconds)
    / seconds_per_tick
~~~

Converting continuous time to an authored tick requires an explicit rounding/snap policy.

Never hide rounding inside generic conversion names.

Prefer APIs like:

- time_to_tick_floor;
- time_to_tick_nearest;
- time_to_tick_for_grid_snap.

---

## 12. Grid snapping

Given continuous pointer/playhead-derived time:

1. convert to continuous tick coordinate;
2. divide by current grid step;
3. apply explicit nearest/floor/ceil policy;
4. multiply back by grid step;
5. return MusicalTick.

Default keyframe drag/create uses nearest valid grid step.

Keyboard stepping uses exact integer addition and does not round.

---

## 13. Keyframe movement

Moving selected keyframes by one grid step:

~~~text
new_tick = old_tick + grid_step_ticks
~~~

For multi-selection, apply the same integer delta to every keyframe.

This preserves rhythmic spacing exactly.

---

## 14. Grid changes

Changing current subdivision affects:

- displayed minor grid;
- keyframe creation;
- keyframe drag snapping;
- keyboard subdivision step.

It does not affect:

- existing keyframe ticks;
- animation values;
- BPM;
- playback;
- export.

No silent quantization.

Explicit quantize may be a future command.

---

## 15. Playhead

Playhead is not a keyframe.

It may exist at arbitrary continuous project time.

This allows:

- smooth scrubbing;
- frame-by-frame video evaluation;
- accurate audio position display;
- preview between musical grid points.

The UI may offer "snap playhead to grid", but it is not a fundamental restriction.

---

## 16. Playback clock

During active audio playback:

~~~text
AudioFramePosition
+ output/project sample-rate mapping
→ ProjectTime
→ musical position
→ animation evaluation
~~~

Do not compute current playback time by:

~~~text
previous_time + frame_delta
~~~

The editor rendering loop is not authoritative.

---

## 17. Paused editor time

When not playing, editor owns a stable playhead ProjectTime.

Timeline actions may set it continuously or by musical stepping.

Starting playback begins audio from the resolved corresponding audio position and then audio becomes the authority.

Stopping/pausing captures the resolved position back into editor playhead state.

---

## 18. Export time

For frame index n and rational FPS:

~~~text
time = n * fps_denominator / fps_numerator
~~~

Compute from n directly.

Do not accumulate:

~~~text
time += 1.0 / fps
~~~

for thousands of frames.

This prevents drift accumulation.

---

## 19. Animation lookup

Keyframes are sorted by MusicalTick.

At evaluation time:

1. convert ProjectTime to continuous musical tick coordinate;
2. binary-search surrounding integer keyframes;
3. calculate normalized interpolation fraction between their absolute musical positions under the tempo map;
4. evaluate interpolation.

For constant BPM this is straightforward.

For future tempo changes, use TempoMap conversion rather than assuming tick distance maps linearly to seconds across a tempo boundary.

---

## 20. Exact keyframe collision

A single animated property cannot contain two independent keyframes with the same MusicalTick in MVP.

When an edit would collide, policy must be explicit.

Recommended MVP behavior:

- moving/pasting onto an occupied tick replaces/merges according to one deterministic rule;
- never leave duplicate ambiguous keyframes.

Exact UX policy belongs in TIMELINE.md.

---

## 21. Project boundaries

Keyframes may not be moved past allowed project musical/time boundaries unless negative/pre-roll support explicitly permits it.

Need explicit policy for:

- negative ticks;
- audio before grid origin;
- composition duration;
- keyframes after audio end.

Recommended:

- allow grid origin offset that places tick 0 after audio start;
- permit negative musical ticks for pre-roll if necessary;
- composition duration remains independent enough to include tail after audio.

Finalize in PROJECT_MODEL/PRODUCT_SPEC.

---

## 22. Display formatting

Internal and display time are different.

Possible displays:

- bar:beat:subdivision;
- bar:beat:tick;
- seconds;
- SMPTE-like frame time later.

The primary editor display should emphasize musical position.

Numeric formatting must not alter stored time.

---

## 23. Serialization

Persist:

- PPQ/schema assumption;
- tempo segments/BPM;
- grid offset;
- keyframe ticks;
- time signature;
- composition FPS/duration.

Do not serialize derived seconds for every keyframe.

Project migration must handle any future PPQ change explicitly. Prefer never changing project PPQ after format stabilization.

---

## 24. Required tests

### Conversion

- tick 0 equals offset;
- one quarter note at 120 BPM = 0.5 seconds;
- negative ticks around offset;
- decimal BPM;
- large tick values.

### Grid

- every exposed subdivision maps to integer ticks;
- repeated keyboard stepping returns exact original tick;
- changing grid resolution leaves keyframes untouched;
- nearest snap boundaries deterministic.

### Playback/export

- long-duration conversion does not accumulate frame drift;
- frame n timestamp is stable;
- audio sample position conversion stable across common rates.

### Animation

- evaluating exactly on keyframe returns exact keyframe value;
- interpolation before/after keyframes follows defined clamping policy.

---

## 25. Open decisions

- exact persisted unit/type for GridOffset;
- exact BPM bounded range;
- UI terminology for subdivision fractions;
- whether negative ticks are fully exposed in MVP UI;
- exact collision policy;
- exact composition-duration relationship to audio duration;
- future tempo-change interpolation semantics.

These must be resolved before serialization format becomes Accepted.
