# ADR 0004 — Start with Three Application Crates

> **Status: Accepted for initial implementation**
>
> **Date: 2026-09-19**

## Context

The codebase needs useful separation between pure domain logic, heavy runtime systems, and editor UI, while build speed is a first-class requirement.

Over-splitting into many crates can increase API ceremony and refactor friction.

## Decision

Start with:

~~~text
rhythm_core
rhythm_engine
rhythm_app
~~~

Do not create a crate per subsystem until measurements or ownership boundaries justify it.

## Consequences

Positive:

- pure core stays fast to test;
- heavy GPU/audio dependencies stay out of core;
- UI iteration does not require moving domain types through many package boundaries;
- repository remains easy to understand.

Negative:

- engine crate will contain several subsystems initially;
- some modules may later need extraction.

## Revisit condition

Split when at least one is true:

- incremental build measurements justify it;
- independent testing becomes materially easier;
- module ownership becomes difficult;
- a component becomes independently reusable.
