# Waveform Pipeline

> **Status: Accepted for MVP**
>
> Waveform is derived navigation data, not a playback clock. It uses time-domain min/max aggregation, never FFT.

## 1. Input

Use decoded source PCM before temporary full decode memory is released.

Stereo is displayed as one envelope:

~~~text
bucket_min = minimum across samples/channels
bucket_max = maximum across samples/channels
~~~

## 2. Representation

~~~rust
WavePeak {
    min: f32,
    max: f32,
}
~~~

## 3. Base resolution

Level 0 aggregates 64 source audio frames per peak bucket.

At 48 kHz this is roughly 1.33 ms per bucket.

## 4. Mip levels

Every next level combines pairs:

~~~text
next.min = min(a.min, b.min)
next.max = max(a.max, b.max)
~~~

Continue until high levels represent the whole clip compactly.

## 5. Worker

Waveform generation runs outside main/audio callback paths.

Timeline can become usable before preprocessing completes.

Completed peak data is immutable.

## 6. Cache

Project JSON never stores waveform peaks.

MVP may persist them in app cache.

If disk caching is implemented, validity key includes:

- normalized source path;
- file size;
- last modified time;
- waveform cache format version.

Any mismatch or cache error rebuilds safely.

A manual/developer rebuild path exists.

## 7. Timeline query

For visible range:

1. map to source/project time;
2. choose level near screen pixel density;
3. slice only visible buckets plus margin;
4. draw a batched mesh/shape.

No full-song scan per frame.

## 8. Visual hierarchy

Waveform stays behind keyframes, playhead, and major rhythm lines.

## 9. Zoom stability

All peak positions derive from source frame/project time.

Changing mip level never changes horizontal timing.

## 10. Memory

Total mip peaks remain under roughly twice the base-level count.

This is much smaller than PCM.

## 11. Edge cases

- final partial bucket is included;
- very short non-empty clip produces at least one peak;
- empty decode is handled safely.

## 12. Tests

- extrema;
- stereo combination;
- partial bucket;
- mip reduction;
- range query;
- level selection;
- short clip;
- 44.1/48 kHz positioning.

## 13. Benchmarks

Use 3-minute and 10-minute generated fixtures plus rapid zoom/pan.

Waveform work must not disrupt audio or interaction.

## 14. Definition of Done

Background preprocessing, correct mips, visible-range query, stable zoom, responsive long tracks, and cache-rebuild safety all pass.
