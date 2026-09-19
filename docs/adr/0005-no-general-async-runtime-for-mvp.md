# ADR 0005 — No General Async Runtime by Default

> **Status: Accepted for MVP**
>
> **Date: 2026-09-19**

## Context

The app needs background decode/cache/export work, but its primary execution model is a native event loop, GPU rendering, and a real-time audio callback.

A full async runtime adds dependencies and conceptual/runtime complexity.

## Decision

Start with explicit worker threads, bounded communication, and narrow synchronization primitives.

Do not add Tokio or another general async runtime unless a concrete subsystem demonstrates a strong need.

## Consequences

Positive:

- simpler runtime model;
- fewer dependencies;
- clearer thread ownership;
- reduced compile-time pressure.

Negative:

- some future network/cloud APIs may be less convenient;
- we may later adopt an async runtime and migrate specific tasks.

## Revisit condition

A required API or substantial concurrency workload becomes materially simpler and safer with async.
