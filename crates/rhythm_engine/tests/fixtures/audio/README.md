# Audio codec fixtures

These files are tiny generated 440 Hz mono tones used only by decoder regression tests.
They contain no third-party media content and tests do not invoke FFmpeg or any external
codec executable.

Generation reference (Linux tooling used once to create committed bytes):

~~~text
ffmpeg -f lavfi -i "sine=frequency=440:sample_rate=8000:duration=0.08" -ac 1 -c:a libmp3lame -b:a 32k tone.mp3
ffmpeg -f lavfi -i "sine=frequency=440:sample_rate=8000:duration=0.08" -ac 1 -c:a pcm_s16le tone.wav
flac --no-padding --force --output-name=tone.flac tone.wav
ffmpeg -f lavfi -i "sine=frequency=440:sample_rate=8000:duration=0.08" -ac 1 -c:a libvorbis -q:a 2 tone.ogg
ffmpeg -f lavfi -i "sine=frequency=440:sample_rate=8000:duration=0.08" -ac 1 -c:a aac -b:a 24k -movflags +faststart tone.m4a
~~~

Recorded SHA-256:

~~~text
tone.mp3   e2017373e9289c6b7ceb598512f04d84809a7756a4e991b2debd576db7ef8b06
tone.flac  05948b22897fb1aecbae9c0239524a76af4891dab529e5c672fe622aef49b7a2
tone.ogg   8009b99f053c72fb315e780c2e8d8ca94cfeff408feada82eb9ba279a792a380
tone.m4a   adaeae0723268e4b53802f9f3321b6bbec2cb7b944a7f92672ce4f9cce52c2ca
~~~

The tests assert probe/decode success, 8 kHz mono semantics, finite PCM, and a
non-silent decoded result. Exact decoded frame counts are intentionally not fixed
for lossy codecs because encoder delay/padding is container/codec metadata.
