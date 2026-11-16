# Koffice Fusion v0.17 — positioned and resizable Sheets objects in Slides

Slides now stores position and size for each embedded Sheets table **relative to the slide**, using a logical coordinate system of 0–1000 for both axes. Objects are no longer restricted to a stacked list in the slide editor.

## Using it

1. Open a slide containing an embedded table (or choose **Insert Sheets table**).
2. **Drag the green header** to reposition the table within the slide.
3. **Drag the bottom-right handle** to resize the table. A frame cannot be moved or resized beyond the slide bounds.
4. For precise adjustments, focus the move or resize handle and use **arrow keys** (1% increments), or **Shift + arrow keys** (5%).
5. Click **Edit** in the table to edit its underlying Sheets cells, including formulas. The position and the data are saved independently.
6. Save the `.kslide`. Geometry is preserved when the presentation is reopened, and when the slide is duplicated or rearranged.

The same relative geometry is used in HTML export, print and presentation views. The displayed tables are static there, but the native `.kslide` retains the editable workbook.

## Implementation

- `SlideObjectFrame` is a typed Rust value containing `x`, `y`, `width` and `height`. Bounds and minimum size are checked before committing.
- The optional `object_frames` map is keyed by embedded object ID, not slide index. Legacy files without geometry remain readable and use a standard layout.
- `slides_embedded_frame` applies a revision-guarded clone/validate/commit operation. An invalid frame or stale revision cannot partially mutate the presentation.
- Pointer movement updates only local DOM styling until the gesture ends; it performs **one native commit** at pointer release, avoiding expensive updates on every mouse movement.
- A keyboard-accessible alternative permits accurate placement.
- Embedded object removal clears the frame; text edits and slide duplication retain the frame.
- Rust regression tests cover validation, rollback, JSON roundtrip, duplication and move. Frontend contract tests cover wiring and exports.

## Current limitations

Object snapping, alignment guides, multi-selection, charts and a global Undo/Redo system are not implemented yet. Stacked objects may overlap; manual placement controls that. The in-app editor uses a responsive 16:9 slide canvas, and a slide may contain at most 12 embedded objects under the existing Fusion limits.
