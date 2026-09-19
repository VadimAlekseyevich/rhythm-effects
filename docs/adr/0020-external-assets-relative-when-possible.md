# ADR 0020 — MVP Keeps Assets External and Stores Relative Paths When Possible

> **Status: Accepted for MVP**
>
> **Date: 2026-09-19**

## Context

Automatically copying/packing assets complicates project folders, large files, duplication, and recovery.

External absolute paths alone make project-folder moves fragile.

## Decision

- .rhfx stores external file references;
- files under the project directory are stored relatively when practical;
- outside files remain absolute;
- source files are never automatically copied/deleted;
- missing files produce unresolved asset state and Relink;
- packed/self-contained projects are post-MVP.

## Consequences

Positive:

- simple import/save model;
- no hidden large copies;
- project folder can be portable when assets are kept beside it.

Negative:

- moving absolute external sources breaks references;
- cross-machine portability is limited.

## Revisit condition

Add explicit Pack Project/Collect Assets workflow post-MVP.
