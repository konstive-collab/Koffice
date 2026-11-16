# Koffice Fusion v0.11 — bidirectional Sheets and Base-to-Slides snapshots

**Scope:** working local transfers using the existing Rust document engines. This is not automatic synchronization, OLE, or Microsoft Office format support.

## Writer → Sheets (lossless worksheet extraction)

1. Open Writer with an embedded Sheets table (created via **Insert Sheets table**).
2. Choose **To Sheets** on that embedded object.
3. Koffice **appends a new sheet** to the currently opened Sheets workbook, opens it, and marks the workbook unsaved.
4. Choose **Save** in Sheets to persist the changes to `.ksheet`.

The embedded object and original Writer document remain unchanged. The source worksheet's cell addresses, raw values and formulas are preserved; the entire sheet is copied, not just the visible grid. Existing Sheets tabs are not replaced. Conflicting sheet names receive numeric suffixes (e.g. `Sheet 1 (2)`). The operation checks the target workbook revision, 50-sheet limit and normal workbook constraints; on failure nothing is changed.

## Base → Slides (static first-page table snapshot)

1. Open a `.kdb` in Base and select a table.
2. Select **To Slides**.
3. Koffice appends a new slide to the current presentation. It contains up to 20 rows and 8 columns from the **first page** of the Base table, rendered as readable text.
4. Save Slides to persist the presentation.

A note on the slide identifies the source table and explicitly declares that this is **not a live connection**. If rows or columns are omitted, the slide contains a truncation notice. The current presentation's existing slides are not replaced, and the database is not mutated. Updating the database will **not** update this slide until a fresh snapshot is created.

## Validation and limitations

Rust tests verify lossless copy with formulas, Unicode, duplicate-name handling, stale revision rollback and bounded Base snapshots. Frontend contract tests check command wiring and UI callbacks. GitHub Actions builds each desktop platform when runners are available.

**Still to implement:** linked objects with dependency tracking/refresh/conflict resolution; stable live Base-to-Slides references; chart objects; insertion at a Writer text anchor; rich editing features; full Office interoperability. Work on these without sacrificing cold startup, responsiveness, or compatibility with existing `.kdoc`, `.ksheet`, `.kslide`, `.kdb` files.
