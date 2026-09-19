# ADR 0001 — Direct egui + winit + wgpu Integration

> **Status: Accepted for MVP**
>
> **Date: 2026-09-19**

## Context

Rhythm Effects needs a highly custom timeline, custom viewport rendering, low-latency interaction, and a composition renderer already built on wgpu.

A higher-level application wrapper could reduce startup code, but would also place another lifecycle abstraction between the editor and its window/GPU integration.

## Decision

Use:

- winit;
- egui;
- egui-winit;
- egui-wgpu;
- wgpu;

directly for the MVP prototype.

Do not make eframe the architectural base.

## Rationale

- explicit event-loop ownership;
- straightforward sharing of wgpu device/queue;
- direct offscreen composition rendering;
- clearer performance boundaries;
- fewer framework assumptions around a specialized editor.

## Consequences

Positive:

- maximum lifecycle control;
- renderer/editor integration remains explicit;
- easier to profile where frame work happens.

Negative:

- more bootstrap/integration code;
- we own more resize/DPI/event details;
- upstream egui/winit changes may require direct adaptation.

## Revisit condition

Reopen if the prototype shows that direct integration creates substantial maintenance cost without measurable control/performance benefit.
