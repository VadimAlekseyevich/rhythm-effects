# ADR 0009 — Use Versioned JSON for Project Schema V1

> **Status: Accepted for MVP**
>
> **Date: 2026-09-19**

## Context

MVP needs a project file format that is easy to debug, migrate, test, and inspect.

A binary format would reduce size but increase tooling and migration/debug complexity before project sizes justify it.

RON is Rust-friendly but less universal outside the Rust ecosystem.

## Decision

Use serde + JSON for schema V1.

The project file uses an application-specific extension later chosen by product naming, but its semantic payload is versioned JSON.

Root wrapper includes an explicit integer schema version independent from application version.

## Rationale

- ubiquitous tooling;
- easy fixture creation;
- readable migration debugging;
- straightforward serde support;
- no need for custom parser;
- simple crash/recovery inspection.

## Consequences

Positive:

- easy diagnostics;
- stable test fixtures;
- simple migration pipeline;
- low implementation risk.

Negative:

- larger files than binary formats;
- parsing may be slower for very large future projects;
- comments are not part of standard JSON.

These costs are acceptable for MVP semantic project sizes.

## Constraints

- derived waveform/image/GPU caches are not stored in the JSON project payload;
- NaN/Infinity are rejected;
- serialization order should be deterministic where practical;
- unknown newer schema fails safely.

## Revisit condition

Only reconsider after measurements show project size/load time is a real problem, or another format provides a clear compatibility benefit.
