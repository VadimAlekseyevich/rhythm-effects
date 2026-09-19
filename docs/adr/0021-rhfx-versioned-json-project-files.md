# ADR 0021 — .rhfx Is Versioned JSON with Transactional Save

> **Status: Accepted for MVP**
>
> **Date: 2026-09-19**

## Decision

- project extension: .rhfx;
- payload: UTF-8 versioned JSON;
- schema version is independent from app version;
- Save writes temp then platform-safe replacement;
- unknown newer schema is never overwritten;
- runtime caches/history are outside project file.

## Why

Debuggability and migration safety are more valuable than binary compactness for MVP.

## Revisit condition

Only revisit format after measured project-file size/load cost becomes material.
