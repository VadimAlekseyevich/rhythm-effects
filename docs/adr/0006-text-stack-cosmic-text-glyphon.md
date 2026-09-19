# ADR 0006 — Prototype Composition Text with cosmic-text + glyphon

> **Status: Accepted for MVP**
>
> **Date: 2026-09-19**

## Context

Composition text requires Unicode shaping, font discovery/fallback, layout, glyph caching, and direct wgpu rendering.

The project already targets wgpu 30.

At evaluation time:

- cosmic-text 0.19 provides shaping, font discovery/fallback, layout, and optional rasterization;
- glyphon 0.12 provides 2D text rendering on wgpu and uses cosmic-text.

## Decision

Use cosmic-text + glyphon for the first composition-text prototype.

Do not use egui's text renderer for creative composition text.

## Consequences

Positive:

- aligned with wgpu renderer;
- mature shaping/fallback responsibilities are not reimplemented;
- direct GPU text path;
- Cyrillic can be tested immediately.

Negative:

- dependency/compile-time cost;
- glyph atlas/layout lifecycle needs integration;
- version alignment with wgpu matters.

## Revisit condition

Reopen if compile time, font behavior, export integration, or dependency alignment is unacceptable.
