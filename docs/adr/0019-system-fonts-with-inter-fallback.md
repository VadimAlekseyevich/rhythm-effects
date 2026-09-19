# ADR 0019 — System Fonts with Bundled Inter Fallback

> **Status: Accepted for MVP**
>
> **Date: 2026-09-19**

## Context

Imported/embedded fonts add portability, licensing, packing, and schema complexity.

MVP still needs reliable Cyrillic/Latin and deterministic missing-font behavior.

## Decision

- TextObject stores family, weight, style for installed/system fonts;
- imported/embedded font assets are post-MVP;
- Inter is bundled as deterministic fallback;
- missing requested font is visibly indicated;
- preview/export use identical fallback.

## Consequences

Positive:

- smaller asset/schema scope;
- predictable fallback;
- reliable Cyrillic baseline.

Negative:

- moving projects between machines may substitute fonts;
- exact typography portability is not guaranteed.

## Revisit condition

Add imported/embedded fonts through explicit Asset semantics after MVP.
