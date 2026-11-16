# Koffice Fusion v0.14 — Link Center

Fusion Link Center is an experimental, **local and manual** control panel for connected Writer tables. Open it from the main toolbar using **Fusion links**.

## Workflow

1. **Inventory** lists all embedded Sheets objects in the current Writer document, including independent copies, session links and file-backed links. **Inventory does not open external source files.** Session-link status can use only the already-open Sheets workbook.
2. Select one or more **file-backed** tables (maximum 12). Choose **Preview selected files**. The panel shows the *full source paths* and asks for confirmation **before reading anything**.
3. Inspect the preview of each object: unchanged, changed, disconnected or conflict. For changed tables, the app shows the number of differing cells and up to eight address/old-value/new-value examples. This comparison uses **stored cell values/formulas**, not rasterized table images or evaluated formula results.
4. Choose **Apply changes atomically**, then approve the source paths again. Koffice re-reads every selected file and validates that its actual sheet contents, workbook revision and identity match the preview. If any file or Writer changed, the entire batch aborts; **no tables are partially updated**.
5. **Save Writer** to persist the updated embedded tables. The linked original files are never modified by this operation.

Select only the links you want to scan. If the preview contains conflicts or disconnected sources, deselect or fix those links and preview again. The Apply action is disabled while blocked rows exist.

## Design and safety

- Rust `fusion_manager.rs` provides a serializable inventory, bounded per-cell diff examples, and two-phase prepare/commit operations.
- A one-use, in-memory preview token is tied to Writer revision and exact source snapshots. Failed or consumed previews cannot be replayed. Data stays local; no network access.
- Native source revalidation is **exact** (not relying on file modified timestamps or revision alone), and prevents changes made between preview and apply from being silently accepted.
- Writer changes are applied to a cloned document with one validate/commit; all updates share a single document revision increment. No partial updates survive errors.
- Link Center does not follow saved file paths on startup, document open, or mere inventory viewing.
- Each operation is bounded to at most 12 objects; the preview list reports up to eight examples per table.
- The file-backed scope is explicit. Session links and independent objects are visible but not eligible for file batch refresh. Session links still have individual controls in Writer.

## Limitations

No automatic watching, cloud sync, dependency propagation to Slides/Base, chart editing, persistent cross-application undo, or three-way merge. A preview is valid only for the current Writer document revision and current source files. Inspect sources from untrusted documents carefully before approving any file read.

## Validation

New Rust unit tests cover two-file atomic refresh, stale preview token, modified sources between scan/apply, protected local edits, independent legacy objects and inventory without filesystem access. Frontend contract tests verify registration, UI actions and explicit confirmation. CI also builds Windows, Linux and macOS desktop packages (subject to runner results).
