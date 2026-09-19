# Documentation Rules

> **Status: Accepted**

## 1. One canonical place per decision

Do not document the same technical contract independently in multiple files.

Other documents should link to the canonical spec.

## 2. Document status

Use:

- Placeholder
- Draft
- Accepted
- Implemented
- Deprecated

## 3. Technical language

Canonical technical docs are written in English to keep identifiers, APIs, library names, and code terminology consistent.

Russian documents may be used for high-level planning and discussion.

## 4. Every subsystem spec should contain

1. Problem
2. User-facing behavior
3. Requirements
4. Non-goals
5. Data model
6. Public API / boundaries
7. Architecture
8. Edge cases
9. Performance constraints
10. Testing
11. Implementation steps
12. Definition of Done

## 5. Decisions with meaningful tradeoffs

Create an ADR in ../adr instead of burying the rationale in chat or commit history.

## 6. Keep docs close to reality

When implementation changes an accepted contract, update the relevant spec in the same change whenever practical.

## 7. Avoid premature detail

Placeholder documents should capture questions and boundaries, not pretend undecided implementation details are final.
