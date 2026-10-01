Rhythm Effects portable Windows x86_64 package

Run RhythmEffects.exe directly. The application stores per-user settings,
logs, recovery data and cache under %LOCALAPPDATA%\RhythmEffects rather than
inside this portable directory.

The bundled FFmpeg executable is resolved only from .\ffmpeg\ffmpeg.exe.
Rhythm Effects does not use an FFmpeg found on PATH.

Third-party provenance and license material are under .\licenses\.

Inter fallback fonts and WGSL renderer shaders are embedded into
RhythmEffects.exe at build time; they are not separate runtime files in this
ZIP.

This MVP package is unsigned. Windows may display a SmartScreen warning.
