# Koffice Fusion v0.15 — persistent Base → Slides links

Koffice Slides can now contain a **manually refreshed slide linked to a local Base database**. The linked slide keeps a complete readable snapshot inside the native `.kslide`, so it can be opened without the source database. This is not a live view or an embedded SQL editor.

## Workflow

1. Open a `.kdb` database in Base and select a table.
2. Choose **Link to Slides** (not the existing **To Slides**, which still creates an independent static snapshot). Confirm the database path.
3. Koffice appends a linked slide to the current presentation. Save the presentation as `.kslide`.
4. Select the linked slide in Slides. **Check Base** shows the saved source path and asks for permission to read it. The status is **current**, **changed**, **conflict**, or **disconnected**.
5. **Refresh Base** is a separate action with the same source confirmation. If the database snapshot has changed and the slide has not been edited locally, Koffice updates that slide. Other slides remain intact. Save Slides afterward.
6. **Relink Base** uses a native file picker to reconnect a moved database. It requires the original table name and refuses to overwrite local edits. **Detach Base** removes the link but preserves the current slide title, body, and notes.

The source path is canonical and absolute, so moving a presentation to another machine does not preserve the external connection automatically. Opening a `.kslide` does not follow database paths, connect to a database, or check for changes.

## Design choices

- Source metadata lives **inside the slide**, not in a separate index-based table. Move, duplicate and delete operations keep links attached to the right slide.
- Source metadata stores the database path, table name, and the last synced title/body/notes. Ordinary slide editing preserves the link metadata but makes that slide **conflicted**; Refresh refuses to overwrite it.
- The source database is opened read-only at the application workflow level: no mutations or writes are issued by the linking and refresh operations. The existing Base API is reused to query a bounded first-page view.
- The first-page snapshot shows at most 20 rows and eight columns. Truncation is explicitly indicated. Data is formatted as slide text, **not** a fully editable relational table or graph.
- Checks and refresh requests always show the source path and require user confirmation. There is no automatic update or background monitoring.
- The native `koffice.slides.v1` format is backward compatible: `base_source` is optional on each slide.
- Rust tests cover persistence/reopening, updates after source modification, conflict protection, missing database files, stale revisions, move/duplicate/delete, and old slide JSON.

## Limitations

No automatic refresh, dependency-graph propagation, multi-user synchronization, merge conflicts, embedded Base UI, data charts or full XLSX/PPTX interoperability. Koffice remains experimental alpha software, so keep backups of important files.
