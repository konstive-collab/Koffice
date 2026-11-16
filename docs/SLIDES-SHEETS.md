# Koffice Fusion v0.16: editable Sheets in Slides

This iteration adds actual **editable native Sheets worksheets inside a slide**. Unlike a screenshot or text snapshot, an embedded worksheet retains full cell addresses and formula strings, including arithmetic with cell references such as `=A1*2` and absolute references such as `=($A$1+A2)/3`. It is a separate copy of the source worksheet and remains editable after reopening the native `.kslide`.

## Workflow

1. In Slides select any slide and click **Insert Sheets table**.
2. Choose whether to copy the active worksheet from Sheets or insert a blank worksheet.
3. The embedded table appears in the slide editing view. Click **Edit** to open the shared Sheets cell grid without leaving Slides.
4. Edit cells, paste a tab/newline-separated range or jump to an address with **Go to cell**. Formula input and values run through the **same Rust Sheets engine** as in Writer and standalone Sheets.
5. **To Sheets** appends the entire embedded worksheet to the current Sheets workbook, without replacing its existing worksheets. Save Sheets to persist the new worksheet.
6. Save the presentation as `.kslide` to preserve the embedded workbook. Print, HTML export and presentation view show a static representation of the table, while the `.kslide` retains complete editable data.

The slide retains the embedded worksheet when its title/body/notes are edited, when the slide is duplicated or moved, and when the presentation is reopened. This extends the existing `koffice.slides.v1` format through the optional `embedded_objects` field; older presentations still deserialize.

## Integrity, performance and limits

- The native `SheetBook::put_batch` validates cell edits, rejects invalid addresses, and commits a complete batch or nothing. Deck revision guards prevent stale updates.
- A presentation is limited to **48 total embedded tables** and **25,000 total populated embedded cells**, plus existing per-table limits and 32 MiB file loading limit. This bounds untrusted documents and prevents enormous allocations during render/serialization.
- Table preview displays a bounded viewport, while the full native cell model is stored and accessible through the Sheets editor. The static print/export representation includes all populated data; sparse sheets use an address/value list.
- Source Sheets workbooks are never modified by editing an embedded copy. Transfer back to Sheets copies cells and resolves duplicate sheet names.
- Unit tests check formula recalculation, JSON roundtrip, local edits rollback, slide duplication/moving, and legacy `.kslide` compatibility. Frontend contract tests check Tauri wiring and display/export integrations.

## Still missing

A table is rendered in a standard content block in the slide editor, not a draggable resizable positioned object. Rich cell styles, interactive chart objects, true XLSX/PPTX feature parity, integrated undo/redo, live table sync and native mobile releases remain future work. Koffice is experimental alpha software; back up important documents.
