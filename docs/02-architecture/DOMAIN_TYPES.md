# Domain Types

> **Status: Draft**
>
> Canonical primitive and domain representations used across project data, animation, time conversion, rendering input, serialization, and editor logic.

## Principle

Do not pass raw primitive values when the unit or semantic meaning matters. Prefer explicit newtypes and validated constructors.

## Musical time

~~~rust
struct MusicalTick(i64);
~~~

- signed integer;
- canonical keyframe time identity;
- PPQ = 960;
- negative values are valid where they map to valid project time;
- never persisted as floating-point seconds.

## Project time

~~~rust
struct ProjectTimeNs(i64);
struct DurationNs(u64);
struct GridOffsetNs(i64);
~~~

ProjectTimeNs is signed nanoseconds from composition time origin.

GridOffsetNs means:

~~~text
project time at MusicalTick(0)
~~~

The normal visible/export composition begins at project time 0.

## BPM

Persist BPM as fixed-point micro-BPM:

~~~rust
struct BpmMicros(u64);
~~~

Examples:

~~~text
120 BPM      = 120_000_000
128.5 BPM    = 128_500_000
174.1234 BPM = 174_123_400
~~~

Resolution is 0.000001 BPM.

Core range:

~~~text
1.0 <= BPM <= 1000.0
~~~

UI may present a narrower convenient range.

## PPQ

~~~rust
const PPQ: i64 = 960;
~~~

One quarter-note beat equals 960 ticks.

PPQ is schema semantics, not an MVP user preference.

## Beat division

~~~rust
struct BeatDivision {
    parts_per_beat: u16,
}
~~~

MVP values:

~~~text
1, 2, 3, 4, 6, 8, 12, 16, 24, 32
~~~

The UI notation 1/4 means one beat split into four equal authoring steps.

| UI | Parts/beat | Ticks/step |
|---|---:|---:|
| 1/1 | 1 | 960 |
| 1/2 | 2 | 480 |
| 1/3 | 3 | 320 |
| 1/4 | 4 | 240 |
| 1/6 | 6 | 160 |
| 1/8 | 8 | 120 |
| 1/12 | 12 | 80 |
| 1/16 | 16 | 60 |
| 1/24 | 24 | 40 |
| 1/32 | 32 | 30 |

Every MVP division maps exactly to integer ticks.

## Frame rate

~~~rust
struct FrameRate {
    numerator: u32,
    denominator: u32,
}
~~~

Invariant:

- numerator > 0;
- denominator > 0;
- normalized form preferred.

Initial UI presets may expose 24, 25, 30, 50, and 60 FPS.

Architecture remains compatible with rational rates later.

FPS is never identity-bearing f32.

## Audio position

~~~rust
struct AudioFramePosition(u64);
struct SampleRate(u32);
~~~

One audio frame contains one sample per channel.

Use the term audio frame rather than ambiguous sample position in APIs.

## Entity identity

Use typed project-local u64 IDs:

~~~rust
struct ObjectId(u64);
struct AssetId(u64);
struct EffectId(u64);
struct KeyframeId(u64);
~~~

Runtime-only identity may include:

~~~rust
struct RequestId(u64);
struct Generation(u64);
struct ProjectRevision(u64);
~~~

## Position

Composition position uses finite f32 values:

~~~rust
struct Vec2 {
    x: f32,
    y: f32,
}
~~~

Unit:

- one composition unit equals one composition pixel at native composition resolution.

Objects may exist outside visible composition bounds.

## Size

Object size and spatial effect radii use composition/local pixel units where their subsystem spec says so.

Geometric width/height are normally non-negative.

## Scale

~~~text
1.0 = 100%
0.5 = 50%
2.0 = 200%
~~~

Scale is finite.

Negative scale is valid for mirroring.

UI may display percentage.

## Rotation

Persist and animate rotation in degrees.

Rules:

- finite;
- not normalized on write;
- may exceed plus/minus 360;
- ordinary scalar interpolation.

Therefore 0 degrees to 360 degrees means one full turn.

Do not use shortest-angle interpolation for the normal Rotation property.

Renderer converts to radians internally where required.

## Opacity

Canonical semantic range:

~~~text
0.0 = fully transparent
1.0 = fully opaque
~~~

Persisted opacity must be finite and within [0,1].

UI displays percentage.

## Color

Project/animation semantic type:

~~~rust
struct LinearRgba {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}
~~~

RGB channels are linear-light values.

UI hex/RGB controls normally operate in sRGB and convert at the boundary.

Alpha is normalized [0,1].

This follows ADR 0012.

## Anchor

Anchor is normalized object-bounds coordinate:

~~~text
(0.0, 0.0) = top-left
(0.5, 0.5) = center
(1.0, 1.0) = bottom-right
~~~

Default is center.

Values outside 0..1 may be allowed to support pivots outside the object.

Normalized anchor keeps the chosen relative pivot stable when bounds change.

## Font size

Font size is a finite positive f32 in composition-space pixels.

## Validation

Persisted domain values reject:

- NaN;
- Infinity;
- invalid rational denominator;
- impossible semantic ranges.

Use validated constructors where external/project input can create invalid values.

## Conversion precision

Use i128/u128 intermediates for fixed-point and rational time conversion before rounding into i64 time/ticks.

Do not use repeated floating-point accumulation for identity-sensitive time.

## Serialization

Serialize semantic numbers, not presentation strings.

Good:

~~~json
{
  "bpm_micros": 128500000,
  "tick": 3840
}
~~~

Formatting such as "128.5 BPM" or "bar 12 beat 3" is UI responsibility.

## Definition of Done

The domain-type contract is implementation-ready when:

- all MVP project values have unambiguous units;
- time/BPM identity is deterministic;
- renderer/UI conversion boundaries are explicit;
- persisted values cannot contain NaN/Infinity;
- transform semantics use documented units;
- conversion-heavy types have unit tests.
