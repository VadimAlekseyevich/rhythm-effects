# Effects

> **Status: Draft**
>
> Effects are a small MVP feature set proving that visual parameters can be animated on the BPM grid.

## 1. Principles

- small high-value set;
- parameters use Animated<T>;
- deterministic stack order;
- GPU-first;
- bounded cost;
- no plugin API;
- no node graph.

---

## 2. Data model

~~~rust
Effect {
    id: EffectId,
    enabled: bool,
    kind: EffectKind,
}
~~~

Typed MVP variants are preferred over stringly-typed parameter maps.

---

## 3. Candidate MVP priority

1. Blur;
2. Glow;
3. Tint/Color;
4. RGB Split;
5. Noise.

Schedule cut order should preserve at least:

- one multi-pass effect;
- one single-pass effect;
- animation of effect parameter.

---

## 4. Animation

Example:

~~~rust
BlurEffect {
    radius: Animated<f32>,
}
~~~

No effect-specific timeline engine.

---

## 5. Stack order

~~~text
source
→ effect 1
→ effect 2
→ ...
→ composite
~~~

UI order equals render order.

Disabled effect skips work where possible.

---

## 6. Isolation

Direct-render objects when no effect requires isolation.

Only create object-local target when effect chain needs it.

This avoids paying offscreen cost for every object.

---

## 7. Blur

MVP:

- radius.

Implementation candidate:

- horizontal pass;
- vertical pass.

Bound maximum radius.

Downsample optimization only after profiling.

---

## 8. Glow

Candidate parameters:

- radius;
- intensity;
- color;
- optional threshold.

Reuse blur where practical.

Keep inspector controls compact.

---

## 9. Tint/Color

Simple single-pass transformation.

Do not build a full grading system.

---

## 10. RGB Split

Candidate:

- amount;
- optional angle/direction.

Useful rhythm effect with limited UI.

---

## 11. Noise

Noise must be deterministic.

If time-varying:

~~~text
noise = f(project/effect seed, evaluation time)
~~~

Do not call uncontrolled RNG every frame.

Preview/export must agree semantically.

---

## 12. Parameter bounds

Every parameter has validated range.

This protects:

- usability;
- GPU cost;
- numerical stability.

---

## 13. Shader source

Built-in WGSL stored with source tree and embedded/loaded predictably.

No release dependency on editable external shader files.

Developer hot reload optional later.

---

## 14. Pipeline cache

One pipeline per implementation/configuration, reused across effect instances.

Per-instance data lives in uniform/storage data.

---

## 15. Temporary textures

Use renderer texture pool.

Measure transient memory in effect-heavy scene.

---

## 16. Effect bounds optimization

Rendering only affected object region + padding can reduce cost.

Defer until full-frame/object target approach is measured.

Correctness first.

---

## 17. Inspector UX

Each effect exposes:

- enabled;
- compact parameters;
- animation controls;
- reorder;
- remove.

No nested tabs.

---

## 18. Tests

- serialization;
- stack ordering;
- disabled bypass;
- parameter animation;
- bounds;
- deterministic noise;
- render-reference tests for key effects where practical.

---

## 19. Definition of Done

- MVP effect list fixed;
- parameters use core animation;
- unnecessary isolation avoided;
- stack order deterministic;
- multi-pass textures reused;
- effect-heavy benchmark exists.
