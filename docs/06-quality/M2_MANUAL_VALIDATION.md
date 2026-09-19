# M2 Manual Validation

> **Status: Accepted validation procedure**
>
> This checklist covers the M2 checks that require a real Windows desktop and GPU presentation path.

## Top-left composition orientation

Goal: confirm that composition-space origin semantics remain top-left with +X right and +Y down when the offscreen target is shown inside the editor viewport.

1. Start the project with `start_project.bat`.
2. Confirm the viewport preview is not vertically flipped.
3. When the first asymmetric renderer reference primitive is available, place it near composition coordinate (0, 0).
4. Expected: it appears at the visible top-left of the composition; increasing X moves right and increasing Y moves down.
5. Any vertical flip, rotation, or swapped axis is a renderer bug and blocks M2 validation.

The current black composition target is symmetric, so step 3 becomes visually decisive as soon as the first reference primitive is introduced. The UV path is already defined with top-left semantic orientation and must preserve this contract.

## Shell size validation

Run the current application and validate both sizes:

### 1280 × 720

- resize to the minimum supported size;
- Transport/Rhythm remains visible;
- Objects and Inspector remain usable;
- Timeline remains visible and resizable;
- Viewport remains present and the composition fits without changing its 1920×1080 creative dimensions;
- no global font shrinking occurs.

### 1920 × 1080

- resize the outer window to approximately 1920×1080;
- left panel starts around 240 logical px;
- right panel starts around 320 logical px;
- Timeline remains roughly in the accepted lower-workspace proportion;
- Viewport composition preserves 16:9;
- no overlapping permanent panels or clipped primary headings.

## Completion evidence

AI-038 may be checked only after both sizes have been observed on a real Windows desktop. Record the date, display scaling percentage, GPU, and whether each size passed.
