# Koffice 0.23 — Sheets Workbench

This release expands Sheets beyond a tiny fixed-grid prototype.

**Full-grid navigation:** A bounded 40 × 16 cell viewport can scroll through the existing 10,000-row, 702-column address space using Rows/Columns controls or **Go to cell**. Only visible inputs are rendered, so moving to row 8,000 does not allocate thousands of DOM elements. Workbook files and existing formulas do not change merely because the viewport moves.

**Sort range:** sort a selected rectangle (maximum 2,000 rows × 30 columns) by any column inside the range, ascending or descending, with an optional header row. Entire selected rows move together. Numeric keys are compared numerically and text keys case-insensitively. This is an atomic Rust operation, revision guarded.

**Fill series:** populate one row or column (up to 1,000 cells) with an arithmetic progression using start and step. **Clear range:** clear up to 20,000 cells without affecting data outside the rectangle. Both require confirmation and save back into the current native workbook.

### Formula safety

Sorting formula cells is deliberately rejected. A complete spreadsheet must rebase relative references and understand dependencies when moving formula expressions; doing so naively can silently corrupt results. Sorting data that is *referenced* by formulas elsewhere can still change their results, so review the impact. No undo/redo history is provided for these batch operations yet; save a backup first.

### Limits

This is not yet a high-performance million-row spreadsheet. The native grid remains limited to 10,000 rows and 702 columns, with at most 20,000 updates in one batch. There are no cell formats, filter dropdowns, conditional formatting, pivot tables, charts, true spreadsheet clipboard selection or XLSX parity yet. The viewport is navigable but not a full virtualization engine with row heights, column resizing and frozen panes.
