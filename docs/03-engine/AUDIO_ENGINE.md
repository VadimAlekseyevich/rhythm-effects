# Audio Engine

> **Status: Draft**
>
> Audio playback provides the authoritative clock while music is playing.

## 1. Responsibilities

- decode supported audio;
- provide playback PCM;
- configure output stream;
- play/pause/seek;
- expose playback position;
- master/track gain;
- provide decoded data to waveform preprocessing;
- handle sample-rate mismatch;
- surface recoverable errors.

Not responsible for:

- BPM detection in MVP;
- DAW editing;
- multi-track mixing;
- VST;
- animation evaluation.

---

## 2. Proposed stack

- Symphonia: decode/demux;
- CPAL: output;
- Rubato: candidate sample-rate conversion.

Rubato's current API provides preallocated processing paths intended for realtime scenarios, which fits the callback constraints.

---

## 3. Internal PCM

Recommended decoded representation:

- f32;
- known channels;
- known source sample rate;
- immutable clip after decode.

Fully decoded PCM is the preferred MVP starting point because it simplifies exact seeking and waveform generation.

Approximate stereo f32 memory:

~~~text
48,000 frames/sec × 2 × 4 bytes
≈ 384 KB/sec
≈ 23 MB/min
≈ 115 MB/5 min
~~~

This is significant but reasonable for desktop MVP. Measure before introducing streaming complexity.

---

## 4. Decode flow

~~~text
import
→ probe
→ decode on worker
→ publish runtime AudioClip
→ waveform preprocess
~~~

Never decode a full compressed track on UI thread.

---

## 5. Target formats

Validate:

- WAV/PCM;
- MP3;
- AAC/M4A;
- FLAC;
- OGG/Vorbis.

Only enable required codec/container features.

Use fixture files from real encoders.

---

## 6. Output policy

MVP default:

- system default output device;
- compatible stream configuration;
- device selector deferred unless testing shows it is required.

Log device/config diagnostics.

---

## 7. Sample-rate mismatch

Must support cases such as 44.1 kHz audio on 48 kHz output.

Candidate approach:

- keep decoded source-rate PCM;
- resample into preallocated output buffers;
- preserve mapping between logical source position and device frames.

Alternative:

- preprocess to canonical internal rate, then adapt to device.

Prototype both only if clock implementation demands it.

The decision criterion is reliable sync and low glitch risk.

---

## 8. Playback state

~~~rust
enum PlaybackState {
    Stopped,
    Paused,
    Playing,
}
~~~

Runtime state includes:

- logical source position;
- current stream generation;
- resampler state;
- output accounting;
- gain.

Not serialized.

---

## 9. Clock contract

During Playing:

~~~text
audio progression
→ authoritative playback ProjectTime
→ TempoMap
→ animation evaluation
~~~

Public conceptual API:

~~~rust
fn playback_time(&self) -> ProjectTime
~~~

The caller should not know CPAL callback internals.

---

## 10. Clock implementation candidates

Possible ingredients:

- consumed/produced frame counters;
- CPAL stream timestamps where backend data is suitable;
- known buffering/latency;
- resampler delay correction;
- seek anchor.

Prototype and measure.

Do not derive clock from render/UI frame delta.

---

## 11. Perceived output latency

Visual sync should consider device buffering.

We must distinguish:

- logical stream position;
- samples queued;
- samples likely heard.

Windows/WASAPI behavior must be tested on multiple machines.

If exact device latency cannot be robustly known everywhere, document tolerance and provide a consistent compensation model.

---

## 12. Callback rules

No callback:

- allocations on normal path;
- blocking mutex;
- file I/O;
- decode;
- GPU calls;
- UI calls;
- project mutation;
- spam logging.

Callback should fill output from prepared state and update narrow clock counters.

---

## 13. Buffering

Use preallocated buffers/ring structures.

Balance:

- enough data to avoid xruns;
- low enough latency for responsive play/seek.

Track underrun/xrun diagnostics where backend exposes them.

---

## 14. Seek

Seek must invalidate stale buffered state.

Concept:

~~~text
seek request
→ new generation
→ reset logical source position
→ reset resampler/buffers
→ update clock anchor
→ resume/fill
~~~

Generation IDs prevent late work from old position being mistaken for current playback.

---

## 15. Pause/resume

Pause captures logical playback position.

Resume begins from captured position.

No dependence on UI redraw timestamp.

---

## 16. End of clip

At end:

- fill required remainder with silence;
- no out-of-bounds;
- transition predictably;
- editor playhead resolves to exact end.

Loop is optional.

---

## 17. Gain

One master/track gain is enough for MVP.

No mixer graph.

Apply gain in prepared playback path.

---

## 18. Background replacement safety

If imported audio changes while decode is running:

- cancel if convenient;
- otherwise ignore stale result using asset/job generation;
- never attach old clip to new asset ID.

---

## 19. Device errors

On device/stream failure:

- stop playback safely;
- preserve project;
- preserve playhead as accurately as possible;
- show recoverable error;
- allow reinitialize.

---

## 20. Thread communication

Prefer narrow communication:

- atomics for simple clock counters;
- bounded command queue;
- prepared immutable PCM;
- error/result channel.

Never share Arc<Mutex<Project>> with callback.

---

## 21. Sync diagnostics

Developer overlay/log should expose:

- logical time;
- audio frame position;
- bar/beat/tick;
- source/device rate;
- estimated output latency;
- xrun/underrun count;
- current buffer fill if available.

---

## 22. Required drift tests

- 60 sec;
- 5 min;
- 10+ min;
- 44.1→48;
- 48→48;
- repeated seek;
- rapid play/pause;
- low UI FPS;
- heavy renderer load;
- different output buffer sizes.

---

## 23. Standalone spike

Before editor integration:

1. decode fixtures;
2. play through CPAL;
3. seek;
4. expose clock;
5. resample;
6. compare long-duration drift;
7. throttle visual observer;
8. verify audio remains authority.

---

## 24. Definition of Done

- required formats decode;
- play/pause/seek stable;
- sample-rate mismatch works;
- callback avoids blocking/allocation by design;
- clock drives animation;
- drift tolerance documented and met;
- device failure does not threaten project data.
