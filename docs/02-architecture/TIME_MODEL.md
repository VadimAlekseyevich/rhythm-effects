# Time Model

> **Status: Accepted for MVP**
>
> Audio, timeline, animation, serialization, snapping, and export must use these definitions consistently.

## 1. Core invariant

Rhythm Effects has four distinct time domains:

1. audio frame position;
2. project time;
3. musical time;
4. video frame index.

The canonical authored keyframe coordinate is integer musical time.

> **A keyframe is identified by MusicalTick, never by floating-point seconds.**

Continuous evaluation between keyframes is still required.

## 2. Canonical types

See DOMAIN_TYPES.md.

Core time types:

~~~rust
MusicalTick(i64)
ProjectTimeNs(i64)
DurationNs(u64)
GridOffsetNs(i64)
BpmMicros(u64)
AudioFramePosition(u64)

FrameRate {
    numerator: u32,
    denominator: u32,
}
~~~

PPQ:

~~~text
960 ticks per quarter-note beat
~~~

## 3. Project time origin

Visible/export composition begins at:

~~~text
ProjectTimeNs(0)
~~~

Calculations may temporarily use signed values around zero.

## 4. Musical origin

GridOffsetNs means:

~~~text
project time of MusicalTick(0)
~~~

This supports intro/pickup before the first downbeat.

Example:

~~~text
audio begins at project time 0
first downbeat occurs at 0.350 seconds
GridOffsetNs = 350,000,000
MusicalTick(0) = 0.350 seconds
~~~

Negative MusicalTicks may represent musical positions before tick zero.

MVP should not normally author keyframes whose mapped project time is below 0.

## 5. BPM

Persist BPM as BpmMicros.

Examples:

~~~text
120 BPM      -> 120,000,000
128.5 BPM    -> 128,500,000
174.1234 BPM -> 174,123,400
~~~

Core valid range:

~~~text
1 <= BPM <= 1000
~~~

No floating-point BPM is persisted.

## 6. Constant-tempo conversion

Given:

~~~text
B = BpmMicros
P = 960
T = MusicalTick
O = GridOffsetNs
~~~

Conceptually:

~~~text
delta_ns =
    T * 60,000,000,000 * 1,000,000
    / (B * P)

project_time_ns = O + rounded(delta_ns)
~~~

Implementation uses wide integer intermediates and rounds to the nearest integer nanosecond. Exact half-nanosecond ties round away from zero.

Never accumulate time tick-by-tick.

## 7. Inverse conversion

Project time maps to a continuous musical position.

Conceptually:

~~~text
tick_position =
    (project_time_ns - offset_ns)
    * B * PPQ
    / (60e9 * 1e6)
~~~

Callers must explicitly choose whether they need:

- continuous position;
- floor tick;
- ceil tick;
- nearest tick;
- grid snap.

Do not hide rounding in generic conversion names.

## 8. Beat divisions

MVP divisions are:

~~~text
1/1
1/2
1/3
1/4
1/6
1/8
1/12
1/16
1/24
1/32
~~~

The denominator is parts per beat.

Ticks per step:

~~~text
PPQ / parts_per_beat
~~~

Every MVP division is exact under PPQ 960.

## 9. Permanent tick lattice vs current grid

The permanent lattice is integer MusicalTick.

The current authoring grid is only a subset used for create/move operations.

Example:

~~~text
a key exists at tick 60 from 1/16 editing
user switches to 1/4 where step = 240
the key remains at tick 60
~~~

Changing grid never silently moves existing keyframes.

## 10. Keyframe validity

A persisted keyframe requires:

- integer MusicalTick;
- unique tick on its Animated property;
- valid project range semantics.

A keyframe does not need to lie on the currently selected authoring division.

## 11. Snap algorithm

For continuous tick position and step S:

1. compute lower grid multiple using Euclidean division;
2. upper = lower + S;
3. choose nearest;
4. exact half-step tie chooses upper/later point.

Example S = 240:

~~~text
+120 -> +240
-120 -> 0
~~~

Tie always goes right/later on timeline.

## 12. Keyboard movement

Keyboard rhythm movement is exact integer arithmetic.

~~~text
new_tick = old_tick +/- ticks_per_step
~~~

Beat movement:

~~~text
+/- 960 ticks
~~~

Bar movement uses time signature.

No float round trip.

## 13. Playhead

Playhead is continuous ProjectTimeNs.

It may exist between grid positions.

This supports:

- smooth scrub;
- exact video frame preview;
- audio playback observation;
- arbitrary scene inspection.

Creating/moving a keyframe resolves onto the authoring grid.

## 14. Playback authority

Paused/stopped:

- EditorSession playhead is authoritative.

On Play:

1. editor resolves requested source position;
2. audio engine starts;
3. audio playback clock becomes authoritative.

On Pause:

1. editor samples resolved final playback time;
2. stores paused playhead;
3. editor becomes authority again.

## 15. Audio-driven playback

During playback:

~~~text
AudioFramePosition
-> ProjectTimeNs
-> continuous musical position
-> animation evaluation
~~~

Never advance playback with editor frame delta.

Dropped UI frames may reduce visual smoothness but cannot shift musical timing.

## 16. Output latency

Audio engine must distinguish producer position from audible position.

The exposed playback clock should approximate the audible timeline position within a tested tolerance.

Exact CPAL/device latency strategy belongs in AUDIO_ENGINE.md.

## 17. Video frame time

FrameRate is rational.

For zero-based frame N:

~~~text
time_ns =
    N * denominator * 1,000,000,000
    / numerator
~~~

Use wide integer intermediates and explicit rounding.

Every frame timestamp derives from N independently.

Never accumulate frame duration.

## 18. Composition duration

Composition duration is project-time duration.

Default after audio import may be source audio duration.

The project may later extend duration for a visual tail.

MVP editing should prevent accidental keyframe creation outside allowed composition time unless an explicit extension exists.

## 19. Time signature

MVP UX assumes 4/4 initially.

Core representation remains explicit:

~~~rust
TimeSignature {
    numerator: u8,
    denominator: u8,
}
~~~

Do not hardcode four beats per bar into generic conversion APIs.

## 20. TempoMap abstraction

MVP normally has one tempo segment.

All callers still go through TempoMap rather than embedding their own BPM formula.

Concept:

~~~rust
TempoMap {
    ppq: 960,
    grid_offset,
    segments,
}
~~~

Future tempo changes must preserve MusicalTick keyframe identity.

## 21. Animation evaluation

Given continuous project time:

1. map to continuous musical position;
2. locate surrounding integer-tick keyframes;
3. derive interpolation progress;
4. apply outgoing interpolation/easing;
5. interpolate value.

Exactly on a keyframe returns its exact stored value.

## 22. Before/after policy

Accepted MVP behavior:

- zero keys: base_value;
- before first key: first key value;
- exactly on key: exact key value;
- after last key: last key value.

Once a property has keyframes, the curve clamps to first/last key outside its authored span.

## 23. Collision policy

One keyframe per property per MusicalTick.

Accepted MVP behavior:

- moved/pasted incoming key wins over occupied unselected key;
- overwritten key is stored in undo history;
- operation is fully reversible;
- timeline should indicate replacement during preview where practical.

No duplicate ambiguous keys are stored.

## 24. BPM changes

Changing BPM does not modify keyframe ticks.

It changes their derived absolute project time.

This is intentional:

~~~text
musical pattern remains attached to rhythm
BPM changes
animation retimes in seconds
~~~

## 25. Grid-offset changes

Changing GridOffsetNs does not modify keyframe ticks.

Their absolute project positions shift with musical grid.

Waveform/audio remain in project time.

This enables BPM-grid alignment over a stationary waveform.

## 26. Serialization

Persist:

- BpmMicros;
- GridOffsetNs;
- PPQ/schema semantics;
- TimeSignature;
- tempo segments;
- MusicalTick keyframes;
- rational FrameRate;
- DurationNs.

Do not serialize derived seconds per keyframe.

## 27. Display formatting

UI may show:

- bar:beat:division;
- bar:beat:tick;
- seconds;
- frame index.

Formatting never changes stored time.

The primary editor display should emphasize musical position.

## 28. Required tests

Fixed-point BPM:

- 120;
- 128.5;
- 174.123456;
- invalid range.

Tick/time:

- tick zero equals offset;
- plus/minus one beat at known BPM;
- negative ticks;
- large tick values;
- round-trip under explicit rounding tolerance.

Grid:

- every MVP division exact;
- positive half-step tie goes later;
- negative half-step tie goes later;
- keyboard forward/back exact;
- changing grid leaves key ticks untouched.

Video:

- frame 0 equals time 0;
- frame N derived independently;
- long export does not accumulate drift.

Retiming:

- BPM change preserves ticks;
- offset change preserves ticks.

## 29. Prohibited patterns

Do not:

- store keyframe seconds as authority;
- store persisted BPM as arbitrary f64;
- advance playback from render delta;
- accumulate export timestamps;
- let timeline implement separate time formulas;
- silently quantize on grid change.

## 30. Definition of Done

Time model is implementation-ready when:

- core time newtypes exist;
- PPQ/division tests pass;
- fixed-point BPM parse/format is tested;
- TempoMap owns conversion;
- snap tie behavior is tested;
- audio/export use the same ProjectTimeNs semantics;
- no subsystem requires raw persisted floating-point timestamp identity.
