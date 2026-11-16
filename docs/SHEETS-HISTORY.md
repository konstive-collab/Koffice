# Koffice 0.26 — sorting with formatting and Sheets Undo/Redo

The Sheets sort engine now moves **cell values and cell styles together** within the selected rectangle. It retains optional blank styles, moves bold/italic, alignment and fill with their row, and clears stale formatting at destination addresses if the incoming row has none. Stable numeric/text sorting and the optional header row remain supported. Formula-bearing sort ranges remain explicitly rejected until reference rebasing is implemented. Styles outside the rectangle are not changed.

## Undo and Redo

Sheets now offers **Undo** and **Redo** toolbar buttons. Ctrl+Z and Ctrl+Y work while focus is not inside a cell's text input. Undo/Redo restores the whole in-memory workbook, including formulas, formatting and freeze settings; the Rust restore command validates the snapshot and applies a fresh monotonically increasing revision.

The app keeps up to **20 successful workbook states**, in memory, for the current workbook. After Undo, a new edit clears the Redo branch. New/Open resets history, preventing history from the previous workbook from being applied. Failed native operations do not create a history entry. History is not saved into `.ksheet` and disappears when the app exits.

**Limits:** this is an alpha snapshot-based history, not a delta-based global command history. Large workbooks may consume extra RAM because previous states remain in memory. While formulas within a sorted rectangle are protected, formulas outside it that reference reordered cells can still change results. Undo/Redo applies to the Sheets workflow; it does not extend to Writer, Slides or Base. This release does not add XLSX interoperability or new spreadsheet formula functions.
