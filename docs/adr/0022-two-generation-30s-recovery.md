# ADR 0022 — Recovery Uses 30-Second Dirty Cadence and Two Generations

> **Status: Accepted for MVP**
>
> **Date: 2026-09-19**

## Decision

- dirty projects recover at most every 30 seconds;
- wait at least 1 second after active transaction;
- one write in flight;
- keep current + previous successful recovery;
- unsaved projects participate;
- explicit Don't Save deletes recovery;
- stale recovery cleanup threshold is 14 days.

## Why

This provides strong crash protection with simple storage/UX and a fallback if the newest recovery itself is corrupt.
