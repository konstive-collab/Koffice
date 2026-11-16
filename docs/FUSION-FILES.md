# Koffice Fusion v0.13 — persistent file-backed Sheets links

**Working alpha scope:** Writer embeds complete worksheets from a user-selected `.ksheet` file. The selected canonical source path, workbook and sheet identity, revision and synchronization baseline are saved inside the Writer `.kdoc`. After closing and reopening Koffice, the link metadata persists and can be checked or refreshed manually.

## How it works

1. In Writer, choose **Link Sheets file**; select a `.ksheet` through the native file dialog, choose its worksheet by number, and name the embedded table.
2. Save Writer as `.kdoc`. All embedded cells and formulas are stored inside the document, along with the file reference.
3. After editing and saving the source `.ksheet` externally or in Koffice, click **Check file** on the embedded table. The UI shows the **full saved path** and asks for permission before opening it.
4. Status is `current`, `changed`, `conflict` (local edits in Writer) or `disconnected` (file missing, cannot parse, source renamed).
5. **Refresh file** is a manual operation with a confirmation prompt. It refuses to replace a locally edited embedded copy or to connect to a different workbook/sheet identity. Save Writer to persist the refreshed data.
6. **Relink file** uses the native picker again if the source was moved/renamed. It cannot overwrite local edits; use **Detach file** to keep those edits as an independent table.

The saved path is an **absolute, canonical path** to the chosen source file. This is intentionally not a portable link across computers; the embedded snapshot itself is portable and remains readable without the source. A moved document's file links may need explicit relinking. **Opening a .kdoc never automatically follows a stored path**. There is no background watcher or network access.

## Safety and correctness

- Reading a saved path is possible **only after explicit Check file or Refresh file**, each displaying the target path in a user confirmation. Opening the file source through the native picker is explicit too.
- Source files must be bounded valid Koffice `.ksheet` JSON, with valid sheets and cells; the source itself is never written to by this workflow.
- Changing the source title/worksheet requires relinking, preventing accidental substitution.
- File refreshes check Writer revision and clean embedded revision before committing a fully validated document; stale edits roll back without mutation.
- Source workbooks keep formula expressions; embedded formulas are neither rasterized nor replaced with calculated values.
- Legacy unlinked and session-linked embedded tables remain readable via optional fields.
- Rust tests exercise on-disk persistence, modified/missing sources, formula preservation, detach, conflict, and relink.
- Node tests cover Tauri command registrations and UI actions. CI checks native builds on Windows/Linux/macOS.

## Limitations

No automatic refresh, symbolic relative workspace links, cloud documents, full Excel interoperability, graph-wide propagation, or automatic three-way conflict merge. `Check file` reads the path explicitly approved by the user; do not approve arbitrary paths in untrusted documents. Full-featured Sheets formatting and rich Writer placement remain future development priorities.
