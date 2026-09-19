# Waveform Pipeline

> **Status: Draft**

## 1. Goal

Render a long audio waveform smoothly at arbitrary timeline zoom without processing raw samples every frame.

Waveform is navigation data, not playback state.

---

## 2. Input

Decoded PCM.

For MVP multichannel display, channels can be combined into one envelope if that keeps UI clearer.

---

## 3. Data representation

Store bucket extrema:

~~~rust
struct WavePeak {
    min: f32,
    max: f32,
}
~~~

Min/max preserves waveform envelope better than absolute magnitude only.

---

## 4. Multi-resolution preprocessing

~~~text
PCM
→ base peak buckets
→ level 1 aggregate
→ level 2 aggregate
→ level 3 ...
~~~

Higher levels summarize larger windows.

This behaves like mip levels for timeline zoom.

---

## 5. Base bucket

Do not store one point per raw sample.

Choose base samples-per-bucket based on maximum useful timeline zoom and memory target.

Benchmark before fixing value.

---

## 6. Aggregation

If level N summarizes K source frames, level N+1 combines adjacent entries:

~~~text
min = min(child mins)
max = max(child maxes)
~~~

Deterministic bucket boundaries are important for stable zoom appearance.

---

## 7. Worker thread

Preprocess off UI thread.

Job result is keyed to the current audio asset identity/generation.

Old job result is discarded if audio changed.

---

## 8. Cache semantics

Waveform is rebuildable runtime cache.

The core project does not depend on waveform cache bytes.

A disk sidecar cache may be added later if startup measurements justify it.

---

## 9. Timeline query

Timeline provides:

~~~text
visible start time
visible end time
pixel width
~~~

Waveform layer chooses a level and returns only visible buckets.

No full-song traversal per frame.

---

## 10. Level selection

Choose level so roughly one or a small number of peak samples map to each horizontal screen pixel.

Too fine:

- wasted CPU/GPU.

Too coarse:

- waveform loses detail.

Selection can include hysteresis if rapid level switching causes visual instability.

---

## 11. Drawing

Initial implementation:

- CPU selects visible peaks;
- generate compact line/triangle vertices;
- draw clipped to waveform row.

At typical widths only thousands of vertices are needed.

No compute shader required.

---

## 12. Grid alignment

Waveform and BPM grid use the same timeline coordinate transform.

Waveform does not maintain a separate pixel/time model.

This avoids subtle drift between visual transient and grid line.

---

## 13. Zoom stability

Because all levels derive from the same extrema hierarchy, amplitude envelope should remain stable as zoom changes.

Test level boundaries visually.

---

## 14. Memory

Total mip hierarchy is a bounded multiple of base level.

Use f32 first.

Do not introduce quantization unless memory profiling shows need.

---

## 15. No FFT

Waveform generation is time-domain min/max aggregation.

FFT is not required.

Do not add rustfft solely for waveform.

FFT can be introduced later for spectrum/BPM analysis features.

---

## 16. Empty/short files

Handle:

- zero frames;
- one frame;
- file shorter than one base bucket;
- final partial bucket.

No division-by-zero or empty-buffer panic.

---

## 17. Performance benchmark

Reference:

- 3–5 min stereo track;
- 1920–3840 px timeline;
- rapid zoom/pan.

Measure:

- preprocess time;
- cache memory;
- visible query cost;
- vertex generation;
- allocations/frame.

---

## 18. Tests

- min/max extraction;
- channel combination;
- level aggregation;
- partial final bucket;
- empty file;
- visible-range slicing;
- level choice;
- duration mapping.

---

## 19. Definition of Done

- preprocessing off main UI path;
- multi-resolution cache works;
- visible-range query is bounded by screen density;
- zoom/pan smooth;
- memory documented;
- no FFT dependency for waveform.
