# Koffice Fusion v0.10 — editable Sheets inside Writer

**Status:** experimental alpha, first implementation of cross-editor embedding. Fusion is not currently equivalent to Microsoft OLE, linked documents or full Excel interoperability.

## What works

In Writer select **Insert Sheets table**. Choose either:
- **OK**: make an independent, complete copy of the currently selected Sheets worksheet; or
- **Cancel**: start with a blank worksheet.

The embedded table appears below Writer's text in the document page. Click **Edit** to open the Sheets cell grid inside Writer. Edits and pasted TSV ranges use the **same Rust SheetBook::put_batch** transaction logic as standalone Sheets. Formula text and full cells are retained, even outside the preview. Use **Go to cell** to navigate addresses A1–ZZ10000. The editor displays a limited viewport for performance.

HTML export and print include a **static rendering of embedded tables** (with formula expressions as stored). Large sparse sheets export as address/value rows instead of a giant mostly empty rectangle. Only the native .kdoc preserves editability.

The complete embedded worksheet is saved **inside the same .kdoc**. No sidecar spreadsheet is needed. Opening an older Koffice .kdoc (without the new optional embedded_objects field) still works. The original independent Sheets workbook is **not** changed by editing its embedded copy.

For formula evaluation, the embedded editor uses the same Sheets evaluation rules. Unsupported formulas remain unsupported: this is not Excel-compatible calculation.

## Data model

- A Writer Document may carry up to 12 typed embedded objects.
- First supported object kind: `sheet`, containing a whole standalone SheetBook instance with one worksheet.
- Unique in-document object IDs and labels; validation of workbook structure, cell addresses, limits and duplicate IDs.
- Maximum total 25,000 populated embedded cells per Writer document, plus the existing 32 MiB serialized file limit.
- Embedding, editing and deletion use document revision checks, copy-and-validate transactions, and revision increments. Invalid edits roll back.
- The existing Writer format identifier remains `koffice.writer.v1` with a backwards-compatible optional `embedded_objects` property.
- Documents can be serialized and reopened with formulas and embedded values intact.

## Limitations / next milestones

The embedded sheet appears **after the text**, rather than at an arbitrary text cursor position. It currently supports the same underlying cell data and formulas, but does **not** yet reproduce the complete standalone Sheets UI (formatting/rich styling, graphical charts, columns/rows resize etc.). There are no live links to another file, automatic source synchronization, Base-to-Slides relations, or cross-app Undo/Redo yet. Do not advertise 100% Excel feature parity.

Next steps: positional object anchors inside rich-text Writer; live linked/snapshot modes with dependency graph and explicit refresh/conflicts; shared chart rendering and data-backed Slides; interoperable office packages and performance benchmarks.

## Validation

Rust unit tests cover copying Sheets formulas, atomic updates and rollback, document revision guards, legacy Writer JSON compatibility, and on-disk .kdoc roundtrip. Frontend contract tests check the editing integration. The full platform CI run must pass before labeling a particular installer verified.
