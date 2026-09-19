# Audio Engine

> **Status: Accepted for MVP**
>
> Audio playback is the timing authority during active playback. The MVP deliberately favors a simple fully prepared playback buffer over streaming complexity.

## 1. Responsibilities

The audio subsystem owns:

- probing/decoding supported music files;
- preparing playback PCM;
- output stream creation;
- play/pause/seek;
- audible-position clock publication;
- gain;
- device/runtime errors;
- source PCM handoff for waveform preprocessing.

It does not own BPM detection, project mutation, timeline UI, audio effects, multi-track mixing, or export timing.

## 2. Accepted stack

- Symphonia: decode/demux;
- Rubato: background/offline sample-rate conversion;
- CPAL: output stream.

No resampling, decode, file I/O, or allocation-heavy work occurs in the realtime callback.

## 3. MVP source formats

Required:

- WAV;
- MP3;
- FLAC;
- OGG/Vorbis;
- AAC in M4A/MP4 where supported by the pinned Symphonia feature set.

MVP semantic channel layouts:

- mono;
- stereo.

Files with unsupported multichannel layouts fail with a clear import error rather than using an undocumented downmix.

Mono is duplicated to stereo semantics where needed.

## 4. Decode representation

Background decode produces finite f32 PCM at source sample rate.

~~~rust
DecodedAudio {
    sample_rate: SampleRate,
    channels: 1 | 2,
    frames: Vec<f32>, // interleaved
    duration: DurationNs,
}
~~~

Decoded source PCM is temporary preparation data used to build waveform peaks and prepare the output-rate playback buffer.

After both complete, the large source PCM may be released.

The original source file remains the canonical audio asset.

## 5. Playback buffer

Before playback, prepare immutable PCM matching the selected output stream sample rate.

~~~rust
PlaybackBuffer {
    sample_rate,
    interleaved_stereo_f32,
    duration,
}
~~~

Rubato performs fixed-ratio conversion on a background worker.

The realtime callback reads already-prepared PCM.

No streaming decoder is required for MVP.

This is intentionally optimized for typical music tracks rather than hour-long media.

## 6. Device policy

MVP uses the current system default output device.

A custom audio-device picker is post-MVP.

Initialization policy:

1. obtain a normal supported output configuration;
2. prefer ordinary stereo output where supported;
3. use the selected configuration sample rate;
4. prepare PlaybackBuffer for that rate.

If the default device changes or becomes unavailable:

- stop playback safely;
- keep Project/editor state;
- show recoverable audio error;
- allow reinitialize against the new default device.

## 7. Output sample conversion

Internal prepared audio is f32.

The callback converts to the CPAL stream sample format when necessary using bounded per-sample conversion.

No heap allocation occurs for this conversion.

For unusual output channel counts, adapter behavior is explicit and tested; creative source semantics remain mono/stereo.

## 8. Callback rules

The output callback may:

- copy prepared samples;
- apply gain;
- convert sample format;
- advance local playback cursor;
- publish clock-anchor atomics;
- emit silence at end or on invalid generation.

It must not:

- lock Project;
- decode;
- resample;
- allocate on the normal path;
- access filesystem;
- call egui/wgpu;
- format/log high-volume messages.

## 9. Playback state

~~~text
Unavailable
Ready
Playing
Paused
Ended
Error
~~~

Project data never depends on the runtime state machine being alive.

## 10. Clock contract

During Playing, animation follows estimated audible project time, not callback producer time and not UI frame delta.

CPAL output timestamps expose the callback instant and predicted playback instant for written data. The stream also exposes a monotonic now() in the same stream-clock domain.

These timestamps are the primary clock source.

## 11. Clock anchor

Each callback publishes a coherent anchor:

~~~text
playback_stream_instant
project_time_of_first_frame_in_this_buffer
playback_generation
~~~

Publication must be lock-free or realtime-safe.

A sequence-counter/seqlock-style set of atomics is acceptable.

The editor samples:

~~~text
stream.now()
relative to playback_stream_instant
+ anchor project time
~~~

to estimate what project time is currently being heard.

UI frame loss therefore does not create musical drift.

## 12. Clock fallback

If a backend cannot provide usable playback timestamps:

- use output-frame cursor plus the best known buffer-latency estimate;
- expose degraded-clock diagnostics;
- keep the same ProjectTimeNs API.

Fallback quality is tested on supported Windows configurations.

## 13. Playback generation

Every discontinuity increments a playback generation:

- seek;
- source replacement;
- stream rebuild;
- restart after device failure.

Clock anchors from old generations are ignored.

## 14. Play

Play begins from EditorSession playhead.

1. clamp/resolve desired project time into audio range;
2. map to PlaybackBuffer frame;
3. create a new playback generation;
4. arm cursor;
5. start stream;
6. receive first valid clock anchor;
7. audio clock becomes authoritative.

## 15. Pause

1. sample current audible clock;
2. pause stream;
3. store ProjectTimeNs into EditorSession playhead;
4. editor playhead becomes authoritative.

Do not derive pause position merely from frames produced ahead of the DAC.

## 16. Seek while paused

Silent and immediate:

- update EditorSession playhead;
- no audio callback work is required.

## 17. Seek while playing

Seek creates a new playback generation.

The callback switches to the new PCM cursor at its next safe callback boundary.

Already queued device audio may still be heard for the output-latency interval.

The visual clock does not jump early to the target before the new generation becomes the audible anchor.

## 18. Scrubbing

MVP scrubbing is silent.

Audible/jog scrubbing is post-MVP.

## 19. End of audio

At end:

- emit silence for remaining requested output;
- mark Ended;
- resolve playhead to audio end/composition policy;
- do not loop automatically.

## 20. Gain

AudioTrack gain defaults to 1.0.

No compressor, limiter, mixer, or audio automation is in MVP.

## 21. Waveform preparation

Waveform is generated from source-rate decoded PCM before that temporary buffer is discarded.

Playback buffer and waveform are independent derived representations of the same source asset.

## 22. Memory policy

Normal steady state after import:

- immutable prepared playback buffer;
- waveform peaks;
- original source-file reference.

Do not retain a second full decoded source PCM after preparation.

A 10+ minute track is a performance fixture.

Streaming is a post-MVP revision only if measurement proves memory unacceptable.

## 23. Sync targets

Reference wired/local audio:

- no cumulative drift over 10 minutes;
- steady-state alignment target within 10 ms where timestamps are reliable;
- release ceiling: no systematic drift beyond 25 ms on reference hardware.

Bluetooth/wireless output is best-effort because device/OS latency reporting may vary.

The key invariant is that error must not grow with duration.

## 24. Diagnostics

Expose:

- output device description/id;
- sample rate/channels/sample format;
- callback buffer size;
- playback generation;
- audible ProjectTimeNs;
- playback-latency estimate;
- stream error count;
- playback-buffer memory.

## 25. Failure policy

On stream/device failure:

- Project remains untouched;
- playback stops;
- best-known playhead is retained;
- show recoverable error;
- Retry/Reinitialize uses system default device.

## 26. Required tests

Pure tests:

- project time to output frame;
- mono/stereo preparation;
- source duration;
- seek clamping;
- stale generation rejection;
- end-of-buffer behavior.

Integration/manual:

- 44.1 kHz source on 48 kHz output;
- 48 kHz source;
- rapid play/pause;
- repeated seek;
- seek while playing;
- 1, 5, and 10 minute drift;
- heavy renderer load;
- low UI FPS;
- device loss/reinitialize.

## 27. Definition of Done

Audio is MVP-ready when:

- required formats import;
- mono/stereo work;
- playback buffer is prepared outside callback;
- callback has no decode/resample/file I/O;
- CPAL timestamp clock drives animation;
- seek-generation semantics are correct;
- 10-minute drift gate passes;
- device failure cannot threaten Project data.
